use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ItemType {
    HealthPotion = 0,
    ManaPotion = 1,
    Gold = 2,
    SlimeCore = 3,
}

impl ItemType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => ItemType::HealthPotion,
            1 => ItemType::ManaPotion,
            2 => ItemType::Gold,
            3 => ItemType::SlimeCore,
            _ => ItemType::Gold,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ItemType::HealthPotion => "Health Potion",
            ItemType::ManaPotion => "Mana Potion",
            ItemType::Gold => "Gold",
            ItemType::SlimeCore => "Slime Core",
        }
    }

    pub fn color(&self) -> macroquad::prelude::Color {
        use macroquad::prelude::*;
        match self {
            ItemType::HealthPotion => RED,
            ItemType::ManaPotion => BLUE,
            ItemType::Gold => GOLD,
            ItemType::SlimeCore => GREEN,
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
}
