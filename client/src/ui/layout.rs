use macroquad::prelude::{Rect, Vec2};

/// Identifier for a clickable UI element
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiElementId {
    // Dialogue
    DialogueChoice(usize),
    DialogueContinue,

    // Crafting
    CraftingCategoryTab(usize),
    CraftingRecipeItem(usize),
    CraftingButton,

    // Inventory & Quick Slots
    InventorySlot(usize),
    QuickSlot(usize),

    // Equipment Slots
    EquipmentSlot(String), // e.g., "body"

    // Quest Log
    QuestLogEntry(usize),
}

/// A single interactive UI element with its bounds
pub struct UiElement {
    pub id: UiElementId,
    pub bounds: Rect,
}

/// Layout for all interactive elements in the current frame
#[derive(Default)]
pub struct UiLayout {
    pub elements: Vec<UiElement>,
}

impl UiLayout {
    pub fn new() -> Self {
        Self {
            elements: Vec::with_capacity(32),
        }
    }

    pub fn clear(&mut self) {
        self.elements.clear();
    }

    pub fn add(&mut self, id: UiElementId, bounds: Rect) {
        self.elements.push(UiElement { id, bounds });
    }

    /// Find element at mouse position (topmost - iterate in reverse)
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&UiElementId> {
        self.elements
            .iter()
            .rev()
            .find(|e| e.bounds.contains(Vec2::new(x, y)))
            .map(|e| &e.id)
    }
}
