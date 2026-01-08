//! Quest Registry
//!
//! Loads, caches, and manages quest definitions from TOML files.
//! Supports hot-reloading during development.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

use super::definition::{Quest, RawQuestFile, ObjectiveType};
use super::state::{PlayerQuestState, QuestProgress};
use super::events::{QuestEvent, QuestEventResult};

/// Registry for all quest definitions
pub struct QuestRegistry {
    /// Loaded quest definitions
    quests: RwLock<HashMap<String, Arc<Quest>>>,
    /// Lua script sources (script_path -> source code)
    lua_scripts: RwLock<HashMap<String, String>>,
    /// Base directory for quest data
    data_dir: PathBuf,
    /// Base directory for scripts
    scripts_dir: PathBuf,
}

impl QuestRegistry {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            quests: RwLock::new(HashMap::new()),
            lua_scripts: RwLock::new(HashMap::new()),
            data_dir: data_dir.join("quests"),
            scripts_dir: data_dir.join("scripts").join("quests"),
        }
    }

    /// Load all quest definitions from the data directory
    pub async fn load_all(&self) -> Result<(), String> {
        info!("Loading quests from {:?}", self.data_dir);

        if !self.data_dir.exists() {
            warn!("Quest directory does not exist: {:?}", self.data_dir);
            return Ok(());
        }

        // Collect all TOML files first (sync), then load them (async)
        let mut paths = Vec::new();
        self.load_quests_recursive_sync(&self.data_dir, &mut paths)?;

        let quest_count = self.load_quest_files(paths).await?;
        info!("Loaded {} quest definitions", quest_count);

        // Validate quest chains
        self.validate_quest_chains().await?;

        Ok(())
    }

    /// Recursively load quests from a directory (non-async to avoid boxing)
    fn load_quests_recursive_sync(&self, dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                self.load_quests_recursive_sync(&path, paths)?;
            } else if path.extension().map_or(false, |ext| ext == "toml") {
                paths.push(path);
            }
        }

        Ok(())
    }

    /// Load all quest files from collected paths
    async fn load_quest_files(&self, paths: Vec<PathBuf>) -> Result<usize, String> {
        let mut count = 0;
        for path in paths {
            if let Err(e) = self.load_quest_file(&path).await {
                warn!("Failed to load quest {:?}: {}", path, e);
            } else {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Load a single quest file
    async fn load_quest_file(&self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;

        let raw: RawQuestFile = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;

        let quest = Quest::from_raw(&raw.quest)?;
        let quest_id = quest.id.clone();

        // Load associated Lua script if present
        if let Some(ref script_path) = quest.lua_script {
            self.load_lua_script(script_path).await?;
        }

        info!("Loaded quest: {} ({})", quest.name, quest_id);

        let mut quests = self.quests.write().await;
        quests.insert(quest_id, Arc::new(quest));

        Ok(())
    }

    /// Load a Lua script file
    async fn load_lua_script(&self, script_path: &str) -> Result<(), String> {
        let full_path = self.scripts_dir.join(script_path);

        if !full_path.exists() {
            return Err(format!("Script not found: {:?}", full_path));
        }

        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read script {:?}: {}", full_path, e))?;

        let mut scripts = self.lua_scripts.write().await;
        scripts.insert(script_path.to_string(), content);

        info!("Loaded script: {}", script_path);
        Ok(())
    }

    /// Validate quest chain references
    async fn validate_quest_chains(&self) -> Result<(), String> {
        let quests = self.quests.read().await;

        for quest in quests.values() {
            // Check previous quest exists
            if let Some(ref prev_id) = quest.chain.previous {
                if !quests.contains_key(prev_id) {
                    warn!(
                        "Quest '{}' references non-existent previous quest '{}'",
                        quest.id, prev_id
                    );
                }
            }

            // Check next quest exists
            if let Some(ref next_id) = quest.chain.next {
                if !quests.contains_key(next_id) {
                    warn!(
                        "Quest '{}' references non-existent next quest '{}'",
                        quest.id, next_id
                    );
                }
            }

            // Check branch_from quest exists
            if let Some(ref branch_id) = quest.chain.branch_from {
                if !quests.contains_key(branch_id) {
                    warn!(
                        "Quest '{}' references non-existent branch_from quest '{}'",
                        quest.id, branch_id
                    );
                }
            }
        }

        Ok(())
    }

    /// Get a quest by ID
    pub async fn get(&self, quest_id: &str) -> Option<Arc<Quest>> {
        let quests = self.quests.read().await;
        quests.get(quest_id).cloned()
    }

    /// Get all quest IDs
    pub async fn all_ids(&self) -> Vec<String> {
        let quests = self.quests.read().await;
        quests.keys().cloned().collect()
    }

    /// Get quests available from a specific NPC
    pub async fn get_quests_for_npc(&self, npc_id: &str) -> Vec<Arc<Quest>> {
        let quests = self.quests.read().await;
        quests
            .values()
            .filter(|q| q.giver_npc == npc_id)
            .cloned()
            .collect()
    }

    /// Get initial quests (those without prerequisites)
    pub async fn get_starting_quests(&self) -> Vec<Arc<Quest>> {
        let quests = self.quests.read().await;
        quests
            .values()
            .filter(|q| q.chain.previous.is_none() && q.chain.branch_from.is_none())
            .cloned()
            .collect()
    }

    /// Check if a player can start a quest
    pub async fn can_start_quest(
        &self,
        quest_id: &str,
        player_level: i32,
        player_state: &PlayerQuestState,
    ) -> bool {
        let quest = match self.get(quest_id).await {
            Some(q) => q,
            None => return false,
        };

        // Check level requirement
        if player_level < quest.level_required {
            return false;
        }

        // Check if already completed
        if player_state.is_quest_completed(quest_id) {
            return false;
        }

        // Check if already active
        if player_state.is_quest_active(quest_id) {
            return false;
        }

        // Check prerequisite quest
        if let Some(ref prev_id) = quest.chain.previous {
            if !player_state.is_quest_completed(prev_id) {
                return false;
            }
        }

        // Check branch_from quest
        if let Some(ref branch_id) = quest.chain.branch_from {
            if !player_state.is_quest_completed(branch_id) {
                return false;
            }
        }

        true
    }

    /// Process a quest event and return any updates
    pub async fn process_event(
        &self,
        event: &QuestEvent,
        player_state: &mut PlayerQuestState,
    ) -> Vec<QuestEventResult> {
        let mut results = Vec::new();

        match event {
            QuestEvent::MonsterKilled { entity_type, .. } => {
                results.extend(
                    self.update_kill_objectives(player_state, entity_type).await
                );
            }
            QuestEvent::ItemCollected { item_id, count, .. } => {
                results.extend(
                    self.update_collect_objectives(player_state, item_id, *count).await
                );
            }
            QuestEvent::NpcInteraction { npc_id, .. } => {
                results.extend(
                    self.update_talk_objectives(player_state, npc_id).await
                );
            }
            QuestEvent::LocationReached { location_id, .. } => {
                results.extend(
                    self.update_location_objectives(player_state, location_id).await
                );
            }
            _ => {}
        }

        results
    }

    /// Update kill objectives for active quests
    async fn update_kill_objectives(
        &self,
        player_state: &mut PlayerQuestState,
        entity_type: &str,
    ) -> Vec<QuestEventResult> {
        let mut results = Vec::new();
        let quests = self.quests.read().await;

        // Find all active quests with kill objectives matching this entity
        let quest_ids: Vec<String> = player_state.active_quests.keys().cloned().collect();

        for quest_id in quest_ids {
            if let Some(quest) = quests.get(&quest_id) {
                for objective in &quest.objectives {
                    if objective.objective_type == ObjectiveType::KillMonster
                        && objective.target == entity_type
                    {
                        if let Some(result) = self.update_single_objective(
                            player_state, &quest_id, &objective.id, 1
                        ) {
                            results.push(result);
                        }
                    }
                }
            }
        }

        results
    }

    /// Helper to update a single objective and check quest completion
    fn update_single_objective(
        &self,
        player_state: &mut PlayerQuestState,
        quest_id: &str,
        objective_id: &str,
        amount: i32,
    ) -> Option<QuestEventResult> {
        let progress = player_state.get_quest_mut(quest_id)?;
        let was_complete = progress.status == super::state::QuestStatus::ReadyToComplete;

        // Update the objective
        let (current, target, newly_completed) = {
            let obj = progress.objectives.get_mut(objective_id)?;
            if obj.completed {
                return None;
            }
            let newly_completed = obj.add_progress(amount);
            (obj.current, obj.target, newly_completed)
        };

        // Check if all objectives are now complete (separate borrow)
        let quest_ready = !was_complete && progress.objectives.values().all(|o| o.completed);
        if quest_ready {
            progress.status = super::state::QuestStatus::ReadyToComplete;
        }

        Some(QuestEventResult::objective_updated(
            quest_id,
            objective_id,
            current,
            target,
            newly_completed,
            quest_ready,
        ))
    }

    /// Update collect objectives for active quests
    async fn update_collect_objectives(
        &self,
        player_state: &mut PlayerQuestState,
        item_id: &str,
        count: i32,
    ) -> Vec<QuestEventResult> {
        let mut results = Vec::new();
        let quests = self.quests.read().await;

        let quest_ids: Vec<String> = player_state.active_quests.keys().cloned().collect();

        for quest_id in quest_ids {
            if let Some(quest) = quests.get(&quest_id) {
                for objective in &quest.objectives {
                    if objective.objective_type == ObjectiveType::CollectItem
                        && objective.target == item_id
                    {
                        if let Some(result) = self.update_single_objective(
                            player_state, &quest_id, &objective.id, count
                        ) {
                            results.push(result);
                        }
                    }
                }
            }
        }

        results
    }

    /// Update talk_to objectives for active quests
    async fn update_talk_objectives(
        &self,
        player_state: &mut PlayerQuestState,
        npc_id: &str,
    ) -> Vec<QuestEventResult> {
        let mut results = Vec::new();
        let quests = self.quests.read().await;

        let quest_ids: Vec<String> = player_state.active_quests.keys().cloned().collect();

        for quest_id in quest_ids {
            if let Some(quest) = quests.get(&quest_id) {
                for objective in &quest.objectives {
                    if objective.objective_type == ObjectiveType::TalkTo
                        && objective.target == npc_id
                    {
                        // Check if this is a sequential objective (must complete others first)
                        if objective.sequential {
                            // Get all other objectives and check if they're complete
                            if let Some(progress) = player_state.get_quest(&quest_id) {
                                let others_complete = quest.objectives.iter()
                                    .filter(|o| o.id != objective.id)
                                    .all(|o| {
                                        progress.objectives.get(&o.id)
                                            .map(|p| p.completed)
                                            .unwrap_or(false)
                                    });
                                if !others_complete {
                                    // Can't complete this objective yet
                                    continue;
                                }
                            }
                        }

                        if let Some(result) = self.force_complete_objective(
                            player_state, &quest_id, &objective.id
                        ) {
                            results.push(result);
                        }
                    }
                }
            }
        }

        results
    }

    /// Helper to force-complete an objective (for talk_to, reach_location)
    fn force_complete_objective(
        &self,
        player_state: &mut PlayerQuestState,
        quest_id: &str,
        objective_id: &str,
    ) -> Option<QuestEventResult> {
        let progress = player_state.get_quest_mut(quest_id)?;
        let was_complete = progress.status == super::state::QuestStatus::ReadyToComplete;

        // Force complete the objective
        let (current, target) = {
            let obj = progress.objectives.get_mut(objective_id)?;
            if obj.completed {
                return None;
            }
            obj.force_complete();
            (obj.current, obj.target)
        };

        // Check if all objectives are now complete
        let quest_ready = !was_complete && progress.objectives.values().all(|o| o.completed);
        if quest_ready {
            progress.status = super::state::QuestStatus::ReadyToComplete;
        }

        Some(QuestEventResult::objective_updated(
            quest_id,
            objective_id,
            current,
            target,
            true,
            quest_ready,
        ))
    }

    /// Update location objectives for active quests
    async fn update_location_objectives(
        &self,
        player_state: &mut PlayerQuestState,
        location_id: &str,
    ) -> Vec<QuestEventResult> {
        let mut results = Vec::new();
        let quests = self.quests.read().await;

        let quest_ids: Vec<String> = player_state.active_quests.keys().cloned().collect();

        for quest_id in quest_ids {
            if let Some(quest) = quests.get(&quest_id) {
                for objective in &quest.objectives {
                    if objective.objective_type == ObjectiveType::ReachLocation
                        && objective.target == location_id
                    {
                        if let Some(result) = self.force_complete_objective(
                            player_state, &quest_id, &objective.id
                        ) {
                            results.push(result);
                        }
                    }
                }
            }
        }

        results
    }

    /// Get Lua script source by path
    pub async fn get_script(&self, script_path: &str) -> Option<String> {
        let scripts = self.lua_scripts.read().await;
        scripts.get(script_path).cloned()
    }

    /// Reload a specific quest (for hot-reload)
    pub async fn reload_quest(&self, quest_id: &str) -> Result<(), String> {
        // Find the file for this quest
        let quest = self.get(quest_id).await
            .ok_or_else(|| format!("Quest '{}' not found", quest_id))?;

        // We'd need to track file -> quest_id mapping for proper hot-reload
        // For now, just reload all quests
        warn!("Hot-reload of single quest not yet implemented, reloading all");
        self.load_all().await
    }

    /// Get count of loaded quests
    pub async fn count(&self) -> usize {
        self.quests.read().await.len()
    }

    /// Start file watcher for hot-reload
    /// Returns a channel receiver that signals when reloads occur
    pub fn start_file_watcher(
        self: &Arc<Self>,
    ) -> Result<tokio::sync::mpsc::Receiver<HotReloadEvent>, String> {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
        use std::time::Duration;

        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let registry = Arc::clone(self);

        // Create the watcher in a blocking thread since notify is sync
        let data_dir = self.data_dir.clone();
        let scripts_dir = self.scripts_dir.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Handle::current();

            let (notify_tx, notify_rx) = std::sync::mpsc::channel();

            let mut watcher = match RecommendedWatcher::new(
                move |res: Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        let _ = notify_tx.send(event);
                    }
                },
                Config::default().with_poll_interval(Duration::from_secs(1)),
            ) {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!("Failed to create file watcher: {}", e);
                    return;
                }
            };

            // Watch quest and script directories
            if data_dir.exists() {
                if let Err(e) = watcher.watch(&data_dir, RecursiveMode::Recursive) {
                    tracing::error!("Failed to watch quest directory: {}", e);
                }
            }

            if scripts_dir.exists() {
                if let Err(e) = watcher.watch(&scripts_dir, RecursiveMode::Recursive) {
                    tracing::error!("Failed to watch scripts directory: {}", e);
                }
            }

            info!("Quest hot-reload watcher started for {:?} and {:?}", data_dir, scripts_dir);

            // Process events
            loop {
                match notify_rx.recv() {
                    Ok(event) => {
                        // Only process modify/create events
                        use notify::EventKind;
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) => {
                                for path in &event.paths {
                                    let extension = path.extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("");

                                    if extension == "toml" || extension == "lua" {
                                        info!("Detected change in {:?}, triggering reload", path);

                                        // Trigger async reload
                                        let reg = Arc::clone(&registry);
                                        let tx = tx.clone();
                                        let path_clone = path.clone();

                                        rt.spawn(async move {
                                            if let Err(e) = reg.load_all().await {
                                                tracing::error!("Hot-reload failed: {}", e);
                                                let _ = tx.send(HotReloadEvent::Error(e)).await;
                                            } else {
                                                info!("Hot-reload completed successfully");
                                                let _ = tx.send(HotReloadEvent::Reloaded(
                                                    path_clone.to_string_lossy().to_string()
                                                )).await;
                                            }
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {
                        // Channel closed, exit
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

/// Events from the hot-reload watcher
#[derive(Debug, Clone)]
pub enum HotReloadEvent {
    /// A file was reloaded successfully
    Reloaded(String),
    /// An error occurred during reload
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_quest_toml() -> &'static str {
        r#"
[quest]
id = "test_quest"
name = "Test Quest"
description = "A test quest"
giver_npc = "test_npc"

[[quest.objectives]]
id = "kill_slimes"
type = "kill_monster"
target = "slime"
count = 3
description = "Kill 3 slimes"

[quest.rewards]
exp = 50
gold = 25
"#
    }

    #[tokio::test]
    async fn test_load_quest() {
        let temp_dir = TempDir::new().unwrap();
        let quest_dir = temp_dir.path().join("quests");
        std::fs::create_dir_all(&quest_dir).unwrap();

        std::fs::write(
            quest_dir.join("test.toml"),
            create_test_quest_toml(),
        ).unwrap();

        let registry = QuestRegistry::new(temp_dir.path());
        registry.load_all().await.unwrap();

        let quest = registry.get("test_quest").await;
        assert!(quest.is_some());

        let quest = quest.unwrap();
        assert_eq!(quest.name, "Test Quest");
        assert_eq!(quest.objectives.len(), 1);
        assert_eq!(quest.rewards.exp, 50);
    }
}
