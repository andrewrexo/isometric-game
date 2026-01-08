//! Quest Definition Structures
//!
//! These structures are deserialized from TOML quest files.

use serde::{Deserialize, Serialize};

/// A quest definition loaded from TOML
#[derive(Debug, Clone, Deserialize)]
pub struct RawQuestFile {
    pub quest: RawQuest,
}

/// Raw quest data as it appears in TOML
#[derive(Debug, Clone, Deserialize)]
pub struct RawQuest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub giver_npc: String,
    #[serde(default)]
    pub level_required: i32,
    /// Optional Lua script for complex logic
    pub lua_script: Option<String>,
    /// Quest chain configuration
    #[serde(default)]
    pub chain: Option<RawQuestChain>,
    /// Quest objectives
    #[serde(default)]
    pub objectives: Vec<RawObjective>,
    /// Quest rewards
    #[serde(default)]
    pub rewards: Option<RawReward>,
    /// Simple dialogue strings (complex dialogue handled in Lua)
    #[serde(default)]
    pub dialogue: Option<RawQuestDialogue>,
}

/// Quest chain configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawQuestChain {
    /// Previous quest that must be completed
    pub previous: Option<String>,
    /// Quest that unlocks after this one
    pub next: Option<String>,
    /// Quest this branches from (for alternate paths)
    pub branch_from: Option<String>,
}

/// Raw objective as it appears in TOML
#[derive(Debug, Clone, Deserialize)]
pub struct RawObjective {
    pub id: String,
    #[serde(rename = "type")]
    pub objective_type: String,
    pub target: String,
    #[serde(default = "default_count")]
    pub count: i32,
    pub description: String,
    /// Whether this objective must be completed in order
    #[serde(default)]
    pub sequential: bool,
}

fn default_count() -> i32 {
    1
}

/// Raw reward as it appears in TOML
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawReward {
    #[serde(default)]
    pub exp: i32,
    #[serde(default)]
    pub gold: i32,
    #[serde(default)]
    pub items: Vec<RawItemReward>,
}

/// Item reward entry
#[derive(Debug, Clone, Deserialize)]
pub struct RawItemReward {
    pub id: String,
    #[serde(default = "default_count")]
    pub count: i32,
}

/// Simple dialogue strings for quests without Lua scripts
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawQuestDialogue {
    pub offer: Option<String>,
    pub accept: Option<String>,
    pub progress: Option<String>,
    pub complete: Option<String>,
}

// ============================================================================
// Resolved Quest Structures (after parsing)
// ============================================================================

/// Objective types supported by the quest system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveType {
    /// Kill X monsters of type Y
    KillMonster,
    /// Collect X items of type Y
    CollectItem,
    /// Talk to a specific NPC
    TalkTo,
    /// Reach a specific location
    ReachLocation,
}

impl ObjectiveType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "kill_monster" | "kill" => Some(ObjectiveType::KillMonster),
            "collect_item" | "collect" => Some(ObjectiveType::CollectItem),
            "talk_to" | "talk" => Some(ObjectiveType::TalkTo),
            "reach_location" | "reach" | "location" => Some(ObjectiveType::ReachLocation),
            _ => None,
        }
    }
}

/// A resolved quest objective
#[derive(Debug, Clone, Serialize)]
pub struct Objective {
    pub id: String,
    pub objective_type: ObjectiveType,
    /// Target entity/item/npc/location ID
    pub target: String,
    /// Number required (1 for talk_to, reach_location)
    pub count: i32,
    /// Display description
    pub description: String,
    /// Whether this must be completed before subsequent objectives
    pub sequential: bool,
}

impl Objective {
    pub fn from_raw(raw: &RawObjective) -> Option<Self> {
        let objective_type = ObjectiveType::from_str(&raw.objective_type)?;
        Some(Self {
            id: raw.id.clone(),
            objective_type,
            target: raw.target.clone(),
            count: raw.count,
            description: raw.description.clone(),
            sequential: raw.sequential,
        })
    }
}

/// Item reward entry
#[derive(Debug, Clone, Serialize)]
pub struct ItemReward {
    pub item_id: String,
    pub count: i32,
}

/// Quest rewards
#[derive(Debug, Clone, Default, Serialize)]
pub struct Reward {
    pub exp: i32,
    pub gold: i32,
    pub items: Vec<ItemReward>,
}

impl Reward {
    pub fn from_raw(raw: &RawReward) -> Self {
        Self {
            exp: raw.exp,
            gold: raw.gold,
            items: raw.items.iter().map(|i| ItemReward {
                item_id: i.id.clone(),
                count: i.count,
            }).collect(),
        }
    }
}

/// Quest chain configuration
#[derive(Debug, Clone, Default, Serialize)]
pub struct QuestChain {
    /// Previous quest that must be completed
    pub previous: Option<String>,
    /// Quest that unlocks after this one
    pub next: Option<String>,
    /// Quest this branches from (for alternate paths)
    pub branch_from: Option<String>,
}

impl QuestChain {
    pub fn from_raw(raw: &RawQuestChain) -> Self {
        Self {
            previous: raw.previous.clone(),
            next: raw.next.clone(),
            branch_from: raw.branch_from.clone(),
        }
    }
}

/// Simple dialogue strings
#[derive(Debug, Clone, Default, Serialize)]
pub struct QuestDialogue {
    /// Dialogue when offering the quest
    pub offer: Option<String>,
    /// Dialogue when quest is accepted
    pub accept: Option<String>,
    /// Dialogue while quest is in progress
    pub progress: Option<String>,
    /// Dialogue when quest is completed
    pub complete: Option<String>,
}

impl QuestDialogue {
    pub fn from_raw(raw: &RawQuestDialogue) -> Self {
        Self {
            offer: raw.offer.clone(),
            accept: raw.accept.clone(),
            progress: raw.progress.clone(),
            complete: raw.complete.clone(),
        }
    }
}

/// A fully resolved quest definition
#[derive(Debug, Clone)]
pub struct Quest {
    pub id: String,
    pub name: String,
    pub description: String,
    /// NPC entity ID that gives this quest
    pub giver_npc: String,
    /// Minimum player level required
    pub level_required: i32,
    /// Optional Lua script path for complex logic
    pub lua_script: Option<String>,
    /// Quest chain configuration
    pub chain: QuestChain,
    /// Quest objectives
    pub objectives: Vec<Objective>,
    /// Quest rewards
    pub rewards: Reward,
    /// Simple dialogue (used when no Lua script)
    pub dialogue: QuestDialogue,
}

impl Quest {
    /// Create a Quest from raw TOML data
    pub fn from_raw(raw: &RawQuest) -> Result<Self, String> {
        let objectives: Vec<Objective> = raw.objectives
            .iter()
            .enumerate()
            .map(|(i, o)| {
                Objective::from_raw(o)
                    .ok_or_else(|| format!("Invalid objective type '{}' at index {}", o.objective_type, i))
            })
            .collect::<Result<Vec<_>, _>>()?;

        if objectives.is_empty() {
            return Err(format!("Quest '{}' has no objectives", raw.id));
        }

        Ok(Self {
            id: raw.id.clone(),
            name: raw.name.clone(),
            description: raw.description.clone(),
            giver_npc: raw.giver_npc.clone(),
            level_required: raw.level_required,
            lua_script: raw.lua_script.clone(),
            chain: raw.chain.as_ref()
                .map(QuestChain::from_raw)
                .unwrap_or_default(),
            objectives,
            rewards: raw.rewards.as_ref()
                .map(Reward::from_raw)
                .unwrap_or_default(),
            dialogue: raw.dialogue.as_ref()
                .map(QuestDialogue::from_raw)
                .unwrap_or_default(),
        })
    }

    /// Check if this quest has a Lua script for complex logic
    pub fn has_script(&self) -> bool {
        self.lua_script.is_some()
    }

    /// Get objective by ID
    pub fn get_objective(&self, id: &str) -> Option<&Objective> {
        self.objectives.iter().find(|o| o.id == id)
    }

    /// Check if a prerequisite quest is required
    pub fn requires_quest(&self) -> Option<&str> {
        self.chain.previous.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_type_parsing() {
        assert_eq!(ObjectiveType::from_str("kill_monster"), Some(ObjectiveType::KillMonster));
        assert_eq!(ObjectiveType::from_str("collect_item"), Some(ObjectiveType::CollectItem));
        assert_eq!(ObjectiveType::from_str("talk_to"), Some(ObjectiveType::TalkTo));
        assert_eq!(ObjectiveType::from_str("reach_location"), Some(ObjectiveType::ReachLocation));
        assert_eq!(ObjectiveType::from_str("invalid"), None);
    }
}
