use serde::{Deserialize, Serialize};

// ============================================================================
// Item Categories
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemCategory {
    Consumable,
    Material,
    Equipment,
    Quest,
}

impl Default for ItemCategory {
    fn default() -> Self {
        ItemCategory::Material
    }
}

// ============================================================================
// Equipment Slots
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSlot {
    #[default]
    None,
    Head,
    Body,
    Weapon,
    Back,
    Feet,
    Ring,
    Gloves,
}

impl EquipmentSlot {
    pub fn as_str(&self) -> &'static str {
        match self {
            EquipmentSlot::None => "none",
            EquipmentSlot::Head => "head",
            EquipmentSlot::Body => "body",
            EquipmentSlot::Weapon => "weapon",
            EquipmentSlot::Back => "back",
            EquipmentSlot::Feet => "feet",
            EquipmentSlot::Ring => "ring",
            EquipmentSlot::Gloves => "gloves",
        }
    }
}

// ============================================================================
// Equipment Stats
// ============================================================================

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EquipmentStats {
    #[serde(default)]
    pub slot_type: EquipmentSlot,
    #[serde(default)]
    pub level_required: i32,
    #[serde(default)]
    pub damage_bonus: i32,
    #[serde(default)]
    pub defense_bonus: i32,
}

// ============================================================================
// Use Effects
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UseEffect {
    Heal { amount: i32 },
    RestoreMana { amount: i32 },
    Buff {
        stat: String,
        amount: i32,
        duration_ms: u64,
    },
    Teleport { destination: String },
}

// ============================================================================
// Raw Item Definition (direct from TOML)
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct RawItemDefinition {
    pub display_name: Option<String>,
    pub sprite: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub category: ItemCategory,
    pub max_stack: Option<i32>,
    pub base_price: Option<i32>,
    #[serde(default = "default_true")]
    pub sellable: bool,
    pub use_effect: Option<UseEffect>,
    /// Equipment-specific stats (only for equipment items)
    pub equipment: Option<EquipmentStats>,
}

fn default_true() -> bool { true }

// ============================================================================
// Resolved Item Definition
// ============================================================================

#[derive(Debug, Clone)]
pub struct ItemDefinition {
    pub id: String,
    pub display_name: String,
    pub sprite: String,
    pub description: String,
    pub category: ItemCategory,
    pub max_stack: i32,
    pub base_price: i32,
    pub sellable: bool,
    pub use_effect: Option<UseEffect>,
    /// Equipment-specific stats (only for equipment items)
    pub equipment: Option<EquipmentStats>,
}

impl ItemDefinition {
    pub fn from_raw(id: &str, raw: &RawItemDefinition) -> Self {
        Self {
            id: id.to_string(),
            display_name: raw.display_name.clone()
                .unwrap_or_else(|| id.to_string()),
            sprite: raw.sprite.clone()
                .unwrap_or_else(|| format!("item_{}", id)),
            description: raw.description.clone()
                .unwrap_or_default(),
            category: raw.category,
            max_stack: raw.max_stack.unwrap_or(99),
            base_price: raw.base_price.unwrap_or(1),
            sellable: raw.sellable,
            use_effect: raw.use_effect.clone(),
            equipment: raw.equipment.clone(),
        }
    }

    /// Check if this item can be used (has a use effect)
    pub fn is_usable(&self) -> bool {
        self.use_effect.is_some()
    }

    /// Check if this is a consumable
    pub fn is_consumable(&self) -> bool {
        self.category == ItemCategory::Consumable
    }

    /// Check if this is equippable (has equipment stats with a valid slot)
    pub fn is_equippable(&self) -> bool {
        self.equipment.as_ref()
            .map(|e| e.slot_type != EquipmentSlot::None)
            .unwrap_or(false)
    }

    /// Check if this is body equipment
    pub fn is_body_equipment(&self) -> bool {
        self.equipment.as_ref()
            .map(|e| e.slot_type == EquipmentSlot::Body)
            .unwrap_or(false)
    }

    /// Check if this is feet equipment
    pub fn is_feet_equipment(&self) -> bool {
        self.equipment.as_ref()
            .map(|e| e.slot_type == EquipmentSlot::Feet)
            .unwrap_or(false)
    }

    /// Get equipment slot type if equippable
    pub fn equipment_slot(&self) -> Option<EquipmentSlot> {
        self.equipment.as_ref().map(|e| e.slot_type)
    }
}
