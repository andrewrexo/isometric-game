//! Quest Script Runner
//!
//! Manages Lua VM instances for executing quest scripts.
//! Each player gets their own Lua state for isolation.

use std::collections::HashMap;
use std::sync::Arc;
use mlua::{Lua, Result as LuaResult, Value, Function, Table, MultiValue};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use super::registry::QuestRegistry;
use super::state::PlayerQuestState;
use super::api::QuestContext;

/// Manages Lua script execution for quests
pub struct QuestRunner {
    /// Registry of quest definitions
    registry: Arc<QuestRegistry>,
    /// Per-player Lua states
    player_states: RwLock<HashMap<String, PlayerLuaState>>,
}

/// Lua state for a single player
struct PlayerLuaState {
    lua: Lua,
    loaded_scripts: Vec<String>,
}

impl PlayerLuaState {
    fn new() -> LuaResult<Self> {
        let lua = Lua::new();

        // Set up sandbox - disable potentially dangerous functions
        lua.scope(|scope| {
            let globals = lua.globals();

            // Remove dangerous functions
            globals.set("os", Value::Nil)?;
            globals.set("io", Value::Nil)?;
            globals.set("loadfile", Value::Nil)?;
            globals.set("dofile", Value::Nil)?;
            globals.set("require", Value::Nil)?; // We'll provide our own module system

            Ok(())
        })?;

        Ok(Self {
            lua,
            loaded_scripts: Vec::new(),
        })
    }

    fn load_script(&mut self, script_path: &str, source: &str) -> LuaResult<()> {
        if self.loaded_scripts.contains(&script_path.to_string()) {
            return Ok(());
        }

        // Load the script as a chunk and execute it to define functions
        self.lua.load(source)
            .set_name(script_path)
            .exec()?;

        self.loaded_scripts.push(script_path.to_string());
        Ok(())
    }

    fn has_function(&self, name: &str) -> bool {
        self.lua.globals()
            .get::<Function>(name)
            .is_ok()
    }
}

/// Result from running a quest script interaction
#[derive(Debug, Clone)]
pub struct ScriptResult {
    /// Dialogue to show (if any)
    pub dialogue: Option<DialogueResult>,
    /// Whether quest was accepted
    pub quest_accepted: bool,
    /// Whether quest was completed
    pub quest_completed: bool,
    /// Notifications to show
    pub notifications: Vec<String>,
    /// Bonus rewards granted
    pub bonus_rewards: Option<BonusReward>,
}

impl Default for ScriptResult {
    fn default() -> Self {
        Self {
            dialogue: None,
            quest_accepted: false,
            quest_completed: false,
            notifications: Vec::new(),
            bonus_rewards: None,
        }
    }
}

/// Dialogue result from a script
#[derive(Debug, Clone)]
pub struct DialogueResult {
    pub speaker: String,
    pub text: String,
    pub choices: Vec<DialogueChoice>,
}

/// A dialogue choice
#[derive(Debug, Clone)]
pub struct DialogueChoice {
    pub id: String,
    pub text: String,
}

/// Bonus rewards from quest completion
#[derive(Debug, Clone)]
pub struct BonusReward {
    pub gold: Option<i32>,
    pub exp: Option<i32>,
    pub items: Vec<(String, i32)>,
}

impl QuestRunner {
    pub fn new(registry: Arc<QuestRegistry>) -> Self {
        Self {
            registry,
            player_states: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create a Lua state for a player
    async fn get_or_create_state(&self, player_id: &str) -> LuaResult<()> {
        let mut states = self.player_states.write().await;
        if !states.contains_key(player_id) {
            let state = PlayerLuaState::new()?;
            states.insert(player_id.to_string(), state);
            debug!("Created Lua state for player {}", player_id);
        }
        Ok(())
    }

    /// Load a quest script for a player
    pub async fn load_quest_script(
        &self,
        player_id: &str,
        quest_id: &str,
    ) -> Result<(), String> {
        // Get the quest to find its script path
        let quest = self.registry.get(quest_id).await
            .ok_or_else(|| format!("Quest '{}' not found", quest_id))?;

        let script_path = quest.lua_script.as_ref()
            .ok_or_else(|| format!("Quest '{}' has no Lua script", quest_id))?;

        // Get the script source
        let source = self.registry.get_script(script_path).await
            .ok_or_else(|| format!("Script '{}' not loaded", script_path))?;

        // Ensure player has a Lua state
        self.get_or_create_state(player_id).await
            .map_err(|e| format!("Failed to create Lua state: {}", e))?;

        // Load the script
        let mut states = self.player_states.write().await;
        if let Some(state) = states.get_mut(player_id) {
            state.load_script(script_path, &source)
                .map_err(|e| format!("Failed to load script: {}", e))?;
        }

        Ok(())
    }

    /// Run the on_interact handler for a quest
    pub async fn run_on_interact(
        &self,
        player_id: &str,
        quest_id: &str,
        quest_state: &mut PlayerQuestState,
        player_choice: Option<&str>,
    ) -> Result<ScriptResult, String> {
        // Load the quest script if not already loaded
        self.load_quest_script(player_id, quest_id).await?;

        let states = self.player_states.read().await;
        let state = states.get(player_id)
            .ok_or_else(|| format!("No Lua state for player {}", player_id))?;

        if !state.has_function("on_interact") {
            return Err("Quest script has no on_interact function".to_string());
        }

        // Create context object for the script
        let ctx = QuestContext::new(
            player_id.to_string(),
            quest_id.to_string(),
            quest_state.clone(),
        );

        // Run the interaction
        // Note: Full async Lua integration would require more complex setup
        // For now, we use a synchronous approach that works for dialogue trees

        let result = self.run_interact_sync(&state.lua, ctx, player_choice)
            .map_err(|e| format!("Script error: {}", e))?;

        Ok(result)
    }

    /// Synchronous interaction runner
    fn run_interact_sync(
        &self,
        lua: &Lua,
        ctx: QuestContext,
        player_choice: Option<&str>,
    ) -> LuaResult<ScriptResult> {
        let mut result = ScriptResult::default();

        // Create the context table for Lua
        let ctx_table = lua.create_table()?;

        // Add quest state getter
        let quest_state = ctx.get_quest_state_string();
        ctx_table.set("_quest_state", quest_state)?;
        ctx_table.set("_player_id", ctx.player_id.clone())?;
        ctx_table.set("_quest_id", ctx.quest_id.clone())?;
        ctx_table.set("_player_choice", player_choice.unwrap_or(""))?;

        // Add result accumulator
        let result_table = lua.create_table()?;
        result_table.set("quest_accepted", false)?;
        result_table.set("quest_completed", false)?;
        result_table.set("notifications", lua.create_table()?)?;
        ctx_table.set("_result", result_table.clone())?;

        // Add get_quest_state method
        let get_quest_state = lua.create_function(|lua, this: Table| {
            let state: String = this.get("_quest_state")?;
            Ok(state)
        })?;
        ctx_table.set("get_quest_state", get_quest_state)?;

        // Add accept_quest method
        let accept_quest = lua.create_function(|lua, this: Table| {
            let result: Table = this.get("_result")?;
            result.set("quest_accepted", true)?;
            Ok(())
        })?;
        ctx_table.set("accept_quest", accept_quest)?;

        // Add complete_quest method
        let complete_quest = lua.create_function(|lua, this: Table| {
            let result: Table = this.get("_result")?;
            result.set("quest_completed", true)?;
            Ok(())
        })?;
        ctx_table.set("complete_quest", complete_quest)?;

        // Add show_notification method
        let show_notification = lua.create_function(|lua, (this, text): (Table, String)| {
            let result: Table = this.get("_result")?;
            let notifications: Table = result.get("notifications")?;
            let len = notifications.len()? + 1;
            notifications.set(len, text)?;
            Ok(())
        })?;
        ctx_table.set("show_notification", show_notification)?;

        // Add get_objective_progress method
        let objectives = ctx.quest_state.clone();
        let quest_id = ctx.quest_id.clone();
        let get_objective_progress = lua.create_function(move |lua, (this, obj_id): (Table, String)| {
            let progress_table = lua.create_table()?;

            if let Some(quest_progress) = objectives.get_quest(&quest_id) {
                if let Some(obj) = quest_progress.objectives.get(&obj_id) {
                    progress_table.set("current", obj.current)?;
                    progress_table.set("target", obj.target)?;
                } else {
                    progress_table.set("current", 0)?;
                    progress_table.set("target", 0)?;
                }
            } else {
                progress_table.set("current", 0)?;
                progress_table.set("target", 0)?;
            }

            Ok(progress_table)
        })?;
        ctx_table.set("get_objective_progress", get_objective_progress)?;

        // Add get_quest_duration method
        let start_time = ctx.quest_state.get_quest(&ctx.quest_id)
            .and_then(|p| Some(p.started_at))
            .flatten();
        let get_quest_duration = lua.create_function(move |_, _: Table| {
            if let Some(started) = start_time {
                let duration = chrono::Utc::now().signed_duration_since(started);
                Ok(duration.num_seconds())
            } else {
                Ok(0i64)
            }
        })?;
        ctx_table.set("get_quest_duration", get_quest_duration)?;

        // Add grant_bonus_reward method
        let grant_bonus_reward = lua.create_function(|lua, (this, reward): (Table, Table)| {
            let result: Table = this.get("_result")?;
            result.set("bonus_reward", reward)?;
            Ok(())
        })?;
        ctx_table.set("grant_bonus_reward", grant_bonus_reward)?;

        // Add show_dialogue method - returns the player's choice or throws error to pause
        let player_choice_val = player_choice.map(|s| s.to_string());
        let show_dialogue = lua.create_function(move |lua, (this, options): (Table, Table)| {
            let result: Table = this.get("_result")?;

            // Store the dialogue info
            result.set("dialogue", options.clone())?;

            let current_choice: String = this.get("_player_choice").unwrap_or_default();

            // If there are choices, we need a real choice (not __continue__)
            let choices: Option<Table> = options.get("choices").ok();
            if let Some(ref choice_table) = choices {
                // Check if there are actual choices
                let has_choices = choice_table.len().unwrap_or(0) > 0;
                if has_choices {
                    // Check if we have a real player choice (not __continue__)
                    if !current_choice.is_empty() && current_choice != "__continue__" {
                        // Clear the choice so it's not reused in recursive calls
                        this.set("_player_choice", "")?;
                        return Ok(Value::String(lua.create_string(&current_choice)?));
                    }
                    // No valid choice yet - throw special error to pause script execution
                    // This prevents the script from continuing to the else branch
                    return Err(mlua::Error::RuntimeError("__WAIT_FOR_CHOICE__".to_string()));
                }
            }

            // No choices - check if we have a continue signal from client
            if current_choice == "__continue__" {
                // Client acknowledged, clear the signal and continue script
                this.set("_player_choice", "")?;
                return Ok(Value::Nil);
            }

            // No continue signal yet - pause script and show this dialogue
            return Err(mlua::Error::RuntimeError("__WAIT_FOR_CONTINUE__".to_string()));
        })?;
        ctx_table.set("show_dialogue", show_dialogue)?;

        // Add unlock_quest method
        let unlock_quest = lua.create_function(|lua, (this, quest_id): (Table, String)| {
            let result: Table = this.get("_result")?;
            let mut unlocks: Vec<String> = result.get("unlocked_quests").unwrap_or_default();
            unlocks.push(quest_id);
            result.set("unlocked_quests", unlocks)?;
            Ok(())
        })?;
        ctx_table.set("unlock_quest", unlock_quest)?;

        // Call on_interact
        let on_interact: Function = lua.globals().get("on_interact")?;
        let call_result = on_interact.call::<()>(ctx_table.clone());

        // Check if we got a special "wait" error (for choices or continue)
        let is_wait_error = match &call_result {
            Err(mlua::Error::RuntimeError(msg)) => {
                msg.contains("__WAIT_FOR_CHOICE__") || msg.contains("__WAIT_FOR_CONTINUE__")
            }
            Err(mlua::Error::CallbackError { cause, .. }) => {
                matches!(cause.as_ref(), mlua::Error::RuntimeError(msg)
                    if msg.contains("__WAIT_FOR_CHOICE__") || msg.contains("__WAIT_FOR_CONTINUE__"))
            }
            _ => false,
        };

        // If it's a real error (not a wait signal), propagate it
        if !is_wait_error {
            call_result?;
        }

        // Extract results
        let result_table: Table = ctx_table.get("_result")?;
        result.quest_accepted = result_table.get("quest_accepted")?;
        result.quest_completed = result_table.get("quest_completed")?;

        // Extract notifications
        if let Ok(notifications) = result_table.get::<Table>("notifications") {
            for pair in notifications.pairs::<i32, String>() {
                if let Ok((_, text)) = pair {
                    result.notifications.push(text);
                }
            }
        }

        // Extract dialogue if present
        if let Ok(dialogue) = result_table.get::<Table>("dialogue") {
            let speaker: String = dialogue.get("speaker").unwrap_or_default();
            let text: String = dialogue.get("text").unwrap_or_default();

            let mut choices = Vec::new();
            if let Ok(choice_table) = dialogue.get::<Table>("choices") {
                for pair in choice_table.pairs::<i32, Table>() {
                    if let Ok((_, choice)) = pair {
                        let id: String = choice.get("id").unwrap_or_default();
                        let text: String = choice.get("text").unwrap_or_default();
                        choices.push(DialogueChoice { id, text });
                    }
                }
            }

            result.dialogue = Some(DialogueResult {
                speaker,
                text,
                choices,
            });
        }

        // Extract bonus rewards if present
        if let Ok(bonus) = result_table.get::<Table>("bonus_reward") {
            let gold: Option<i32> = bonus.get("gold").ok();
            let exp: Option<i32> = bonus.get("exp").ok();
            result.bonus_rewards = Some(BonusReward {
                gold,
                exp,
                items: Vec::new(),
            });
        }

        Ok(result)
    }

    /// Run on_objective_progress handler
    pub async fn run_on_objective_progress(
        &self,
        player_id: &str,
        quest_id: &str,
        objective_id: &str,
        new_count: i32,
        quest_state: &PlayerQuestState,
    ) -> Result<Vec<String>, String> {
        // Load the quest script if not already loaded
        self.load_quest_script(player_id, quest_id).await?;

        let states = self.player_states.read().await;
        let state = states.get(player_id)
            .ok_or_else(|| format!("No Lua state for player {}", player_id))?;

        if !state.has_function("on_objective_progress") {
            return Ok(Vec::new()); // Not an error, just no handler
        }

        let mut notifications = Vec::new();

        // Create minimal context
        let lua = &state.lua;
        let ctx_table = lua.create_table()
            .map_err(|e| format!("Lua error: {}", e))?;

        let result_table = lua.create_table()
            .map_err(|e| format!("Lua error: {}", e))?;
        result_table.set("notifications", lua.create_table()
            .map_err(|e| format!("Lua error: {}", e))?)
            .map_err(|e| format!("Lua error: {}", e))?;
        ctx_table.set("_result", result_table.clone())
            .map_err(|e| format!("Lua error: {}", e))?;

        // Add show_notification
        let show_notification = lua.create_function(|lua, (this, text): (Table, String)| {
            let result: Table = this.get("_result")?;
            let notifs: Table = result.get("notifications")?;
            let len = notifs.len()? + 1;
            notifs.set(len, text)?;
            Ok(())
        }).map_err(|e| format!("Lua error: {}", e))?;
        ctx_table.set("show_notification", show_notification)
            .map_err(|e| format!("Lua error: {}", e))?;

        // Call on_objective_progress
        let on_progress: Function = lua.globals().get("on_objective_progress")
            .map_err(|e| format!("Lua error: {}", e))?;
        on_progress.call::<()>((ctx_table.clone(), objective_id, new_count))
            .map_err(|e| format!("Script error: {}", e))?;

        // Extract notifications
        let result_table: Table = ctx_table.get("_result")
            .map_err(|e| format!("Lua error: {}", e))?;
        if let Ok(notifs) = result_table.get::<Table>("notifications") {
            for pair in notifs.pairs::<i32, String>() {
                if let Ok((_, text)) = pair {
                    notifications.push(text);
                }
            }
        }

        Ok(notifications)
    }

    /// Clean up Lua state for a disconnected player
    pub async fn cleanup_player(&self, player_id: &str) {
        let mut states = self.player_states.write().await;
        if states.remove(player_id).is_some() {
            debug!("Cleaned up Lua state for player {}", player_id);
        }
    }

    /// Reload all scripts (for hot-reload)
    pub async fn reload_scripts(&self) {
        let mut states = self.player_states.write().await;
        for (player_id, state) in states.iter_mut() {
            state.loaded_scripts.clear();
            debug!("Cleared loaded scripts for player {}", player_id);
        }
        info!("All quest scripts marked for reload");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_lua_state_creation() {
        let state = PlayerLuaState::new();
        assert!(state.is_ok());
    }

    #[test]
    fn test_script_loading() {
        let mut state = PlayerLuaState::new().unwrap();

        let script = r#"
            function on_interact(ctx)
                return "hello"
            end
        "#;

        let result = state.load_script("test.lua", script);
        assert!(result.is_ok());
        assert!(state.has_function("on_interact"));
    }

    #[test]
    fn test_sandbox() {
        let state = PlayerLuaState::new().unwrap();

        // os should be nil
        let os: Value = state.lua.globals().get("os").unwrap();
        assert!(os.is_nil());

        // io should be nil
        let io: Value = state.lua.globals().get("io").unwrap();
        assert!(io.is_nil());
    }
}
