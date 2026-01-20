use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Animation Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum AnimationType {
    Blob = 0,
    Humanoid = 1,
    Quadruped = 2,
    Flying = 3,
}

impl Default for AnimationType {
    fn default() -> Self {
        AnimationType::Blob
    }
}

impl AnimationType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "blob" => AnimationType::Blob,
            "humanoid" => AnimationType::Humanoid,
            "quadruped" => AnimationType::Quadruped,
            "flying" => AnimationType::Flying,
            _ => AnimationType::Blob,
        }
    }
}

// ============================================================================
// Raw TOML Structures (direct deserialization)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawEntityStats {
    pub max_hp: Option<i32>,
    pub damage: Option<i32>,
    pub attack_range: Option<i32>,
    pub aggro_range: Option<i32>,
    pub chase_range: Option<i32>,
    pub move_cooldown_ms: Option<u64>,
    pub attack_cooldown_ms: Option<u64>,
    pub respawn_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawEntityRewards {
    pub exp_base: Option<i32>,
    pub gold_min: Option<i32>,
    pub gold_max: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LootEntry {
    pub item_id: String,
    pub drop_chance: f32,
    #[serde(default = "default_one")]
    pub quantity_min: i32,
    #[serde(default = "default_one")]
    pub quantity_max: i32,
}

fn default_one() -> i32 { 1 }

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawEntityBehaviors {
    #[serde(default)]
    pub hostile: bool,
    #[serde(default)]
    pub merchant: bool,
    #[serde(default)]
    pub quest_giver: bool,
    #[serde(default)]
    pub banker: bool,
    #[serde(default)]
    pub craftsman: bool,
    #[serde(default)]
    pub teleporter: bool,
    #[serde(default)]
    pub wander_enabled: bool,
    pub wander_radius: Option<i32>,
    pub wander_pause_min_ms: Option<u64>,
    pub wander_pause_max_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MerchantConfig {
    pub shop_id: String,
    #[serde(default = "default_buy_mult")]
    pub buy_multiplier: f32,
    #[serde(default = "default_sell_mult")]
    pub sell_multiplier: f32,
    pub restock_interval_minutes: Option<u32>,
}

fn default_buy_mult() -> f32 { 0.5 }
fn default_sell_mult() -> f32 { 1.0 }

#[derive(Debug, Clone, Deserialize)]
pub struct QuestGiverConfig {
    #[serde(default)]
    pub available_quests: Vec<String>,
    pub requires_reputation: Option<ReputationRequirement>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReputationRequirement {
    pub faction: String,
    pub level: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DialogueConfig {
    pub greeting: Option<String>,
    pub shop_open: Option<String>,
    pub quest_available: Option<String>,
    pub quest_complete: Option<String>,
}

/// Raw entity prototype as loaded directly from TOML
#[derive(Debug, Clone, Deserialize)]
pub struct RawEntityPrototype {
    pub extends: Option<String>,

    pub display_name: Option<String>,
    pub sprite: Option<String>,
    pub animation_type: Option<String>,
    pub description: Option<String>,

    #[serde(default)]
    pub stats: RawEntityStats,

    #[serde(default)]
    pub rewards: RawEntityRewards,

    #[serde(default)]
    pub loot: Vec<LootEntry>,

    #[serde(default)]
    pub behaviors: RawEntityBehaviors,

    pub merchant: Option<MerchantConfig>,
    pub quest_giver: Option<QuestGiverConfig>,
    pub dialogue: Option<DialogueConfig>,
}

// ============================================================================
// Resolved Structures (after inheritance)
// ============================================================================

#[derive(Debug, Clone)]
pub struct ResolvedStats {
    pub max_hp: i32,
    pub damage: i32,
    pub attack_range: i32,
    pub aggro_range: i32,
    pub chase_range: i32,
    pub move_cooldown_ms: u64,
    pub attack_cooldown_ms: u64,
    pub respawn_time_ms: u64,
}

impl Default for ResolvedStats {
    fn default() -> Self {
        Self {
            max_hp: 100,
            damage: 10,
            attack_range: 1,
            aggro_range: 5,
            chase_range: 8,
            move_cooldown_ms: 500,
            attack_cooldown_ms: 800,
            respawn_time_ms: 10000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedRewards {
    pub exp_base: i32,
    pub gold_min: i32,
    pub gold_max: i32,
}

impl Default for ResolvedRewards {
    fn default() -> Self {
        Self {
            exp_base: 10,
            gold_min: 1,
            gold_max: 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntityBehaviors {
    pub hostile: bool,
    pub merchant: bool,
    pub quest_giver: bool,
    pub banker: bool,
    pub craftsman: bool,
    pub teleporter: bool,
    pub wander_enabled: bool,
    pub wander_radius: i32,
    pub wander_pause_min_ms: u64,
    pub wander_pause_max_ms: u64,
}

impl Default for EntityBehaviors {
    fn default() -> Self {
        Self {
            hostile: false,
            merchant: false,
            quest_giver: false,
            banker: false,
            craftsman: false,
            teleporter: false,
            wander_enabled: false,
            wander_radius: 3,
            wander_pause_min_ms: 2000,
            wander_pause_max_ms: 5000,
        }
    }
}

impl From<&RawEntityBehaviors> for EntityBehaviors {
    fn from(raw: &RawEntityBehaviors) -> Self {
        Self {
            hostile: raw.hostile,
            merchant: raw.merchant,
            quest_giver: raw.quest_giver,
            banker: raw.banker,
            craftsman: raw.craftsman,
            teleporter: raw.teleporter,
            wander_enabled: raw.wander_enabled,
            wander_radius: raw.wander_radius.unwrap_or(3),
            wander_pause_min_ms: raw.wander_pause_min_ms.unwrap_or(2000),
            wander_pause_max_ms: raw.wander_pause_max_ms.unwrap_or(5000),
        }
    }
}

/// Fully resolved entity prototype (after inheritance resolution)
#[derive(Debug, Clone)]
pub struct EntityPrototype {
    pub id: String,
    pub display_name: String,
    pub sprite: String,
    pub animation_type: AnimationType,
    pub description: String,

    pub stats: ResolvedStats,
    pub rewards: ResolvedRewards,
    pub loot: Vec<LootEntry>,

    pub behaviors: EntityBehaviors,
    pub merchant: Option<MerchantConfig>,
    pub quest_giver: Option<QuestGiverConfig>,
    pub dialogue: DialogueConfig,
}

impl EntityPrototype {
    /// Check if this entity is a hostile monster
    pub fn is_hostile(&self) -> bool {
        self.behaviors.hostile || self.stats.damage > 0
    }

    /// Check if this entity is an NPC with interactions
    pub fn is_npc(&self) -> bool {
        self.behaviors.merchant
            || self.behaviors.quest_giver
            || self.behaviors.banker
            || self.behaviors.craftsman
            || self.behaviors.teleporter
    }
}
