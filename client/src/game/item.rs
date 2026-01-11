/// Item on the ground
#[derive(Debug, Clone)]
pub struct GroundItem {
    pub id: String,
    pub item_id: String,
    pub x: f32,
    pub y: f32,
    pub quantity: i32,
    pub animation_time: f64, // For bobbing animation
}

impl GroundItem {
    pub fn new(id: String, item_id: String, x: f32, y: f32, quantity: i32) -> Self {
        Self {
            id,
            item_id,
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
    pub item_id: String,
    pub quantity: i32,
}

impl InventorySlot {
    pub fn new(item_id: String, quantity: i32) -> Self {
        Self { item_id, quantity }
    }
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

    /// Count total quantity of an item by ID across all slots
    pub fn count_item_by_id(&self, item_id: &str) -> i32 {
        self.slots
            .iter()
            .filter_map(|slot| slot.as_ref())
            .filter(|slot| slot.item_id == item_id)
            .map(|slot| slot.quantity)
            .sum()
    }

    /// Swap two inventory slots (for optimistic UI updates)
    pub fn swap_slots(&mut self, from_slot: usize, to_slot: usize) {
        if from_slot < INVENTORY_SIZE && to_slot < INVENTORY_SIZE {
            self.slots.swap(from_slot, to_slot);
        }
    }

    /// Move item from a slot to an empty slot, or set a slot directly (for optimistic unequip)
    pub fn set_slot(&mut self, slot_index: usize, item_id: String, quantity: i32) {
        if slot_index < INVENTORY_SIZE {
            self.slots[slot_index] = Some(InventorySlot { item_id, quantity });
        }
    }

    /// Clear a slot (for optimistic equip from inventory)
    pub fn clear_slot(&mut self, slot_index: usize) {
        if slot_index < INVENTORY_SIZE {
            self.slots[slot_index] = None;
        }
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
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
