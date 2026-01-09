use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ItemType {
    HealthPotion = 0,
    ManaPotion = 1,
    Gold = 2,
    SlimeCore = 3,
    IronOre = 4,
    GoblinEar = 5,
}

impl ItemType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => ItemType::HealthPotion,
            1 => ItemType::ManaPotion,
            2 => ItemType::Gold,
            3 => ItemType::SlimeCore,
            4 => ItemType::IronOre,
            5 => ItemType::GoblinEar,
            _ => ItemType::Gold,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ItemType::HealthPotion => "Health Potion",
            ItemType::ManaPotion => "Mana Potion",
            ItemType::Gold => "Gold",
            ItemType::SlimeCore => "Slime Core",
            ItemType::IronOre => "Iron Ore",
            ItemType::GoblinEar => "Goblin Ear",
        }
    }

    pub fn color(&self) -> macroquad::prelude::Color {
        use macroquad::prelude::*;
        match self {
            ItemType::HealthPotion => RED,
            ItemType::ManaPotion => BLUE,
            ItemType::Gold => GOLD,
            ItemType::SlimeCore => GREEN,
            ItemType::IronOre => GRAY,
            ItemType::GoblinEar => DARKGREEN,
        }
    }

    /// Get ItemType from string ID (for recipe matching)
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "health_potion" => Some(ItemType::HealthPotion),
            "mana_potion" => Some(ItemType::ManaPotion),
            "gold" => Some(ItemType::Gold),
            "slime_core" => Some(ItemType::SlimeCore),
            "iron_ore" => Some(ItemType::IronOre),
            "goblin_ear" => Some(ItemType::GoblinEar),
            _ => None,
        }
    }
}

/// Item on the ground
#[derive(Debug, Clone)]
pub struct GroundItem {
    pub id: String,
    pub item_type: ItemType,
    pub x: f32,
    pub y: f32,
    pub quantity: i32,
    pub animation_time: f64, // For bobbing animation
}

impl GroundItem {
    pub fn new(id: String, item_type: ItemType, x: f32, y: f32, quantity: i32) -> Self {
        Self {
            id,
            item_type,
            x,
            y,
            quantity,
            animation_time: macroquad::time::get_time(),
        }
    }
}

/// Inventory slot (client-side)
#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub item_type: ItemType,
    pub quantity: i32,
}

pub const INVENTORY_SIZE: usize = 20;

/// Player's inventory
#[derive(Debug, Clone)]
pub struct Inventory {
    pub slots: Vec<Option<InventorySlot>>,
    pub gold: i32,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            slots: vec![None; INVENTORY_SIZE],
            gold: 0,
        }
    }

    /// Count total quantity of an item type across all slots
    pub fn count_item(&self, item_type: ItemType) -> i32 {
        self.slots
            .iter()
            .filter_map(|slot| slot.as_ref())
            .filter(|slot| slot.item_type == item_type)
            .map(|slot| slot.quantity)
            .sum()
    }

    /// Count total quantity of an item by string ID
    pub fn count_item_by_id(&self, item_id: &str) -> i32 {
        if let Some(item_type) = ItemType::from_id(item_id) {
            self.count_item(item_type)
        } else {
            0
        }
    }
}

// ============================================================================
// Recipe Definitions (received from server)
// ============================================================================

#[derive(Debug, Clone)]
pub struct RecipeIngredient {
    pub item_id: String,
    pub item_name: String,
    pub count: i32,
}

#[derive(Debug, Clone)]
pub struct RecipeResult {
    pub item_id: String,
    pub item_name: String,
    pub count: i32,
}

#[derive(Debug, Clone)]
pub struct RecipeDefinition {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub level_required: i32,
    pub ingredients: Vec<RecipeIngredient>,
    pub results: Vec<RecipeResult>,
}
