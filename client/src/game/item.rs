use macroquad::rand::gen_range;

// ============================================================================
// Gold Pile Animation State
// ============================================================================

/// Animation state for a single gold nugget in a pile
#[derive(Debug, Clone)]
pub struct GoldNuggetState {
    /// Target/resting position after animation settles (pixels, pre-zoom)
    pub target_x: f32,
    pub target_y: f32,
    /// Phase offset for bob animation (creates shimmer effect)
    pub phase_offset: f64,
}

/// Animation state for a gold pile (multiple nuggets)
#[derive(Debug, Clone)]
pub struct GoldPileState {
    pub nuggets: Vec<GoldNuggetState>,
    pub spawn_time: f64,
}

impl GoldPileState {
    /// Create a new gold pile with nuggets based on quantity
    pub fn new(quantity: i32, spawn_time: f64) -> Self {
        let nugget_count = Self::calculate_nugget_count(quantity);
        let nuggets = Self::generate_nuggets(nugget_count);
        Self { nuggets, spawn_time }
    }

    /// Calculate number of nuggets (1-15) based on gold quantity
    fn calculate_nugget_count(quantity: i32) -> usize {
        match quantity {
            1..=3 => 1,
            4..=6 => 2,
            7..=10 => 3,
            11..=15 => 4,
            16..=20 => 5,
            21..=25 => 6,
            26..=35 => 7,
            36..=50 => 8,
            51..=65 => 9,
            66..=80 => 10,
            81..=100 => 11,
            101..=150 => 12,
            151..=250 => 13,
            251..=400 => 14,
            _ => 15,
        }
    }

    /// Generate nugget positions using golden-angle spiral
    fn generate_nuggets(count: usize) -> Vec<GoldNuggetState> {
        let mut nuggets = Vec::with_capacity(count);

        // Golden angle for natural spiral distribution
        let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());

        for i in 0..count {
            // Calculate target resting position (spiral pattern)
            let (target_x, target_y) = if count == 1 {
                (0.0, 0.0)
            } else {
                let angle = i as f32 * golden_angle;
                let max_radius = 6.0 + (count as f32 * 0.4);
                let radius = max_radius * ((i as f32 + 0.5) / count as f32).sqrt();
                let x = angle.cos() * radius;
                let y = angle.sin() * radius * 0.5; // Compress Y for isometric
                (x, y)
            };

            // Random phase for bob animation
            let phase_offset = gen_range(0.0, std::f64::consts::TAU);

            nuggets.push(GoldNuggetState {
                target_x,
                target_y,
                phase_offset,
            });
        }

        nuggets
    }
}

// ============================================================================
// Ground Item
// ============================================================================

/// Item on the ground
#[derive(Debug, Clone)]
pub struct GroundItem {
    pub id: String,
    pub item_id: String,
    pub x: f32,
    pub y: f32,
    pub quantity: i32,
    pub animation_time: f64,
    /// Special animation state for gold piles (None for non-gold items)
    pub gold_pile: Option<GoldPileState>,
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
            gold_pile: None,
        }
    }

    /// Create a gold item with pile animation state
    pub fn new_gold(id: String, x: f32, y: f32, quantity: i32) -> Self {
        let spawn_time = macroquad::time::get_time();
        Self {
            id,
            item_id: "gold".to_string(),
            x,
            y,
            quantity,
            animation_time: spawn_time,
            gold_pile: Some(GoldPileState::new(quantity, spawn_time)),
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
