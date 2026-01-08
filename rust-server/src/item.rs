use serde::Serialize;

// ============================================================================
// Item Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum ItemType {
    HealthPotion = 0,
    ManaPotion = 1,
    Gold = 2,
    SlimeCore = 3, // Drop from slimes
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

    pub fn max_stack(&self) -> i32 {
        match self {
            ItemType::HealthPotion => 10,
            ItemType::ManaPotion => 10,
            ItemType::Gold => 9999,
            ItemType::SlimeCore => 99,
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(self, ItemType::HealthPotion | ItemType::ManaPotion)
    }
}

// ============================================================================
// Inventory
// ============================================================================

pub const INVENTORY_SIZE: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub struct InventorySlot {
    pub item_type: ItemType,
    pub quantity: i32,
}

impl InventorySlot {
    pub fn new(item_type: ItemType, quantity: i32) -> Self {
        Self { item_type, quantity }
    }
}

#[derive(Debug, Clone)]
pub struct Inventory {
    pub slots: Vec<Option<InventorySlot>>,
    pub gold: i32, // Gold is stored separately
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            slots: vec![None; INVENTORY_SIZE],
            gold: 0,
        }
    }

    /// Try to add an item to inventory. Returns the quantity that couldn't fit.
    pub fn add_item(&mut self, item_type: ItemType, mut quantity: i32) -> i32 {
        // Gold goes to separate counter
        if item_type == ItemType::Gold {
            self.gold += quantity;
            return 0;
        }

        let max_stack = item_type.max_stack();

        // First, try to stack with existing items
        for slot in &mut self.slots {
            if quantity <= 0 {
                break;
            }
            if let Some(ref mut inv_slot) = slot {
                if inv_slot.item_type == item_type {
                    let can_add = max_stack - inv_slot.quantity;
                    if can_add > 0 {
                        let add = quantity.min(can_add);
                        inv_slot.quantity += add;
                        quantity -= add;
                    }
                }
            }
        }

        // Then, try to find empty slots for remaining quantity
        for slot in &mut self.slots {
            if quantity <= 0 {
                break;
            }
            if slot.is_none() {
                let add = quantity.min(max_stack);
                *slot = Some(InventorySlot::new(item_type, add));
                quantity -= add;
            }
        }

        quantity // Return what couldn't fit
    }

    /// Use an item at the given slot. Returns true if successful.
    pub fn use_item(&mut self, slot_index: usize) -> Option<ItemType> {
        if slot_index >= INVENTORY_SIZE {
            return None;
        }

        if let Some(ref mut slot) = self.slots[slot_index] {
            if slot.item_type.is_usable() {
                let item_type = slot.item_type;
                slot.quantity -= 1;
                if slot.quantity <= 0 {
                    self.slots[slot_index] = None;
                }
                return Some(item_type);
            }
        }
        None
    }

    /// Get inventory as a serializable update
    pub fn to_update(&self) -> Vec<InventorySlotUpdate> {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            slot.as_ref().map(|s| InventorySlotUpdate {
                slot: i as u8,
                item_type: s.item_type as u8,
                quantity: s.quantity,
            })
        }).collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InventorySlotUpdate {
    pub slot: u8,
    pub item_type: u8,
    pub quantity: i32,
}

// ============================================================================
// Ground Item (dropped in world)
// ============================================================================

#[derive(Debug, Clone)]
pub struct GroundItem {
    pub id: String,
    pub item_type: ItemType,
    pub x: f32,
    pub y: f32,
    pub quantity: i32,
    pub owner_id: Option<String>, // Player who can pick up (None = anyone)
    pub drop_time: u64,           // When the item was dropped
}

impl GroundItem {
    pub fn new(id: &str, item_type: ItemType, x: f32, y: f32, quantity: i32, owner_id: Option<String>, current_time: u64) -> Self {
        Self {
            id: id.to_string(),
            item_type,
            x,
            y,
            quantity,
            owner_id,
            drop_time: current_time,
        }
    }

    /// Check if the item has expired (60 second lifetime)
    pub fn is_expired(&self, current_time: u64) -> bool {
        const ITEM_LIFETIME_MS: u64 = 60000; // 60 seconds
        current_time - self.drop_time > ITEM_LIFETIME_MS
    }

    /// Check if a player can pick up this item
    pub fn can_pickup(&self, player_id: &str, current_time: u64) -> bool {
        // Owner-only period: first 10 seconds
        const OWNER_PERIOD_MS: u64 = 10000;

        if current_time - self.drop_time < OWNER_PERIOD_MS {
            // During owner period, only owner can pick up
            match &self.owner_id {
                Some(owner) => owner == player_id,
                None => true, // No owner = anyone can pick up
            }
        } else {
            // After owner period, anyone can pick up
            true
        }
    }
}

// ============================================================================
// Item Update (sent to client)
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct GroundItemUpdate {
    pub id: String,
    pub item_type: u8,
    pub x: f32,
    pub y: f32,
    pub quantity: i32,
}

impl From<&GroundItem> for GroundItemUpdate {
    fn from(item: &GroundItem) -> Self {
        Self {
            id: item.id.clone(),
            item_type: item.item_type as u8,
            x: item.x,
            y: item.y,
            quantity: item.quantity,
        }
    }
}

// ============================================================================
// Loot Tables
// ============================================================================

use crate::npc::NpcType;
use rand::Rng;

/// Generate random drops for an NPC
pub fn generate_drops(npc_type: NpcType, x: f32, y: f32, killer_id: &str, current_time: u64) -> Vec<GroundItem> {
    let mut drops = Vec::new();
    let mut item_counter = 0u32;
    let mut rng = rand::thread_rng();

    match npc_type {
        NpcType::Slime => {
            // Always drop some gold (5-15)
            let gold_amount = rng.gen_range(5..=15);
            let id = format!("item_{}_{}", current_time, item_counter);
            item_counter += 1;
            drops.push(GroundItem::new(
                &id,
                ItemType::Gold,
                x,
                y,
                gold_amount,
                Some(killer_id.to_string()),
                current_time,
            ));

            // 30% chance to drop Slime Core
            if rng.gen_range(0..100) < 30 {
                let id = format!("item_{}_{}", current_time, item_counter);
                item_counter += 1;
                drops.push(GroundItem::new(
                    &id,
                    ItemType::SlimeCore,
                    x + 0.3, // Offset slightly so items don't stack
                    y + 0.3,
                    1,
                    Some(killer_id.to_string()),
                    current_time,
                ));
            }

            // 20% chance to drop Health Potion
            if rng.gen_range(0..100) < 20 {
                let id = format!("item_{}_{}", current_time, item_counter);
                drops.push(GroundItem::new(
                    &id,
                    ItemType::HealthPotion,
                    x - 0.3,
                    y - 0.3,
                    1,
                    Some(killer_id.to_string()),
                    current_time,
                ));
            }
        }
    }

    drops
}
