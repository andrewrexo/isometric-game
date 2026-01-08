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
}
