//! Quest Lua API
//!
//! Defines the context object and functions exposed to Lua scripts.

use super::state::PlayerQuestState;

/// Context object passed to Lua quest scripts
///
/// This provides all the information and methods a quest script needs
/// to handle interactions, check progress, and update state.
#[derive(Clone)]
pub struct QuestContext {
    /// The player interacting with the quest
    pub player_id: String,
    /// The quest being interacted with
    pub quest_id: String,
    /// Current quest state for this player
    pub quest_state: PlayerQuestState,
}

impl QuestContext {
    pub fn new(player_id: String, quest_id: String, quest_state: PlayerQuestState) -> Self {
        Self {
            player_id,
            quest_id,
            quest_state,
        }
    }

    /// Get the quest state as a string for Lua
    pub fn get_quest_state_string(&self) -> String {
        if self.quest_state.is_quest_completed(&self.quest_id) {
            "completed".to_string()
        } else if let Some(progress) = self.quest_state.get_quest(&self.quest_id) {
            if progress.status == super::state::QuestStatus::ReadyToComplete {
                "ready_to_complete".to_string()
            } else {
                "in_progress".to_string()
            }
        } else {
            "not_started".to_string()
        }
    }
}

/// Dialogue options that can be shown to the player
#[derive(Debug, Clone)]
pub struct DialogueOptions {
    /// Who is speaking
    pub speaker: String,
    /// The dialogue text
    pub text: String,
    /// Available choices (empty for non-branching dialogue)
    pub choices: Vec<DialogueChoice>,
}

/// A single dialogue choice
#[derive(Debug, Clone)]
pub struct DialogueChoice {
    /// Unique ID for this choice
    pub id: String,
    /// Display text for the choice
    pub text: String,
}

/// Result of a dialogue interaction
#[derive(Debug, Clone)]
pub enum DialogueResult {
    /// Player selected a choice
    Choice(String),
    /// Dialogue was acknowledged (no choices)
    Acknowledged,
    /// Dialogue was cancelled
    Cancelled,
}

/// Reward that can be granted by scripts
#[derive(Debug, Clone, Default)]
pub struct ScriptReward {
    pub gold: Option<i32>,
    pub exp: Option<i32>,
    pub items: Vec<(String, i32)>,
}
