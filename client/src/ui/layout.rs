use macroquad::prelude::{Rect, Vec2};

/// Identifier for a clickable UI element
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiElementId {
    // Dialogue
    DialogueChoice(usize),
    DialogueContinue,
    DialogueClose,

    // Crafting
    CraftingCategoryTab(usize),
    CraftingRecipeItem(usize),
    CraftingButton,
    MainTab(usize), // 0=Recipes, 1=Shop

    // Inventory & Quick Slots
    InventorySlot(usize),
    QuickSlot(usize),

    // Equipment Slots
    EquipmentSlot(String), // e.g., "body"

    // Quest Log
    QuestLogEntry(usize),

    // Context Menu
    ContextMenuOption(usize),

    // Escape Menu
    EscapeMenuZoom05x,
    EscapeMenuZoom1x,
    EscapeMenuZoom2x,
    EscapeMenuMusicSlider,
    EscapeMenuSfxSlider,
    EscapeMenuMuteToggle,
    EscapeMenuUiScaleSlider,
    EscapeMenuShiftDropToggle,
    EscapeMenuChatLogToggle,
    EscapeMenuTapPathfindToggle,
    EscapeMenuDisconnect,

    // World Items
    GroundItem(String), // item instance ID

    // Shop
    ShopBuyItem(usize),
    ShopSellItem(usize),
    ShopBuyScrollArea,
    ShopSellScrollArea,
    ShopBuyQuantityMinus,
    ShopBuyQuantityPlus,
    ShopBuyConfirmButton,
    ShopSellQuantityMinus,
    ShopSellQuantityPlus,
    ShopSellConfirmButton,

    // Menu Buttons
    MenuButtonInventory,
    MenuButtonCharacter,
    MenuButtonSocial,
    MenuButtonSkills,
    MenuButtonSettings,

    // Skills Panel
    SkillSlot(usize),

    // Gold Display (inventory header)
    GoldDisplay,

    // Inventory grid scroll area
    InventoryGridArea,

    // Gold Drop Dialog
    GoldDropConfirm,
    GoldDropCancel,
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
