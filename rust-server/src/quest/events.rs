//! Quest Event Types
//!
//! Events that can trigger quest objective progress.

use serde::{Deserialize, Serialize};

/// Events that can trigger quest progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestEvent {
    /// Player killed a monster
    MonsterKilled {
        player_id: String,
        /// Entity prototype ID (e.g., "pig", "elder_villager")
        entity_type: String,
        /// Level of the killed monster
        level: i32,
    },

    /// Player collected an item
    ItemCollected {
        player_id: String,
        /// Item ID (e.g., "slime_core", "health_potion")
        item_id: String,
        /// Quantity collected
        count: i32,
    },

    /// Player talked to an NPC
    NpcInteraction {
        player_id: String,
        /// NPC entity prototype ID
        npc_id: String,
    },

    /// Player reached a location
    LocationReached {
        player_id: String,
        /// Location identifier
        location_id: String,
        /// World coordinates
        x: i32,
        y: i32,
    },

    /// Player accepted a quest (from dialogue)
    QuestAccepted {
        player_id: String,
        quest_id: String,
    },

    /// Player abandoned a quest
    QuestAbandoned {
        player_id: String,
        quest_id: String,
    },

    /// Player made a dialogue choice
    DialogueChoice {
        player_id: String,
        quest_id: String,
        choice_id: String,
    },
}

impl QuestEvent {
    /// Get the player ID associated with this event
    pub fn player_id(&self) -> &str {
        match self {
            QuestEvent::MonsterKilled { player_id, .. } => player_id,
            QuestEvent::ItemCollected { player_id, .. } => player_id,
            QuestEvent::NpcInteraction { player_id, .. } => player_id,
            QuestEvent::LocationReached { player_id, .. } => player_id,
            QuestEvent::QuestAccepted { player_id, .. } => player_id,
            QuestEvent::QuestAbandoned { player_id, .. } => player_id,
            QuestEvent::DialogueChoice { player_id, .. } => player_id,
        }
    }

    /// Get event type as string (for logging/debugging)
    pub fn event_type(&self) -> &'static str {
        match self {
            QuestEvent::MonsterKilled { .. } => "monster_killed",
            QuestEvent::ItemCollected { .. } => "item_collected",
            QuestEvent::NpcInteraction { .. } => "npc_interaction",
            QuestEvent::LocationReached { .. } => "location_reached",
            QuestEvent::QuestAccepted { .. } => "quest_accepted",
            QuestEvent::QuestAbandoned { .. } => "quest_abandoned",
            QuestEvent::DialogueChoice { .. } => "dialogue_choice",
        }
    }
}

/// Result of processing a quest event
#[derive(Debug, Clone)]
pub struct QuestEventResult {
    /// Quest ID that was affected
    pub quest_id: String,
    /// Objective ID that was updated (if any)
    pub objective_id: Option<String>,
    /// New progress value
    pub new_progress: Option<i32>,
    /// Target value for objective
    pub target: Option<i32>,
    /// Whether the objective was just completed
    pub objective_completed: bool,
    /// Whether the entire quest is now ready to complete
    pub quest_ready: bool,
}

impl QuestEventResult {
    pub fn objective_updated(
        quest_id: &str,
        objective_id: &str,
        new_progress: i32,
        target: i32,
        objective_completed: bool,
        quest_ready: bool,
    ) -> Self {
        Self {
            quest_id: quest_id.to_string(),
            objective_id: Some(objective_id.to_string()),
            new_progress: Some(new_progress),
            target: Some(target),
            objective_completed,
            quest_ready,
        }
    }

    pub fn no_change(quest_id: &str) -> Self {
        Self {
            quest_id: quest_id.to_string(),
            objective_id: None,
            new_progress: None,
            target: None,
            objective_completed: false,
            quest_ready: false,
        }
    }
}
