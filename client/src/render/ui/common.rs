//! Common UI constants and types shared across UI components

use macroquad::prelude::Color;

// ============================================================================
// UI Color Palette - Medieval Fantasy Theme
// ============================================================================

// Panel backgrounds (darker to lighter for depth)
pub const PANEL_BG_DARK: Color = Color::new(0.071, 0.071, 0.094, 0.961);    // rgba(18, 18, 24, 245)
pub const PANEL_BG_MID: Color = Color::new(0.110, 0.110, 0.149, 1.0);       // rgba(28, 28, 38, 255)

// Frame/Border colors (bronze/gold medieval theme)
pub const FRAME_OUTER: Color = Color::new(0.322, 0.243, 0.165, 1.0);        // rgba(82, 62, 42, 255)
pub const FRAME_MID: Color = Color::new(0.557, 0.424, 0.267, 1.0);          // rgba(142, 108, 68, 255)
pub const FRAME_INNER: Color = Color::new(0.729, 0.580, 0.361, 1.0);        // rgba(186, 148, 92, 255)
pub const FRAME_ACCENT: Color = Color::new(0.855, 0.698, 0.424, 1.0);       // rgba(218, 178, 108, 255)

// Slot colors
pub const SLOT_BG_EMPTY: Color = Color::new(0.086, 0.086, 0.118, 1.0);      // rgba(22, 22, 30, 255)
pub const SLOT_BG_FILLED: Color = Color::new(0.125, 0.125, 0.173, 1.0);     // rgba(32, 32, 44, 255)
pub const SLOT_INNER_SHADOW: Color = Color::new(0.047, 0.047, 0.063, 1.0);  // rgba(12, 12, 16, 255)
pub const SLOT_HIGHLIGHT: Color = Color::new(0.188, 0.188, 0.251, 1.0);     // rgba(48, 48, 64, 255)
pub const SLOT_BORDER: Color = Color::new(0.227, 0.212, 0.188, 1.0);        // rgba(58, 54, 48, 255)

// Hover/Selection states
pub const SLOT_HOVER_BG: Color = Color::new(0.188, 0.188, 0.282, 1.0);      // rgba(48, 48, 72, 255)
pub const SLOT_HOVER_BORDER: Color = Color::new(0.659, 0.580, 0.424, 1.0);  // rgba(168, 148, 108, 255)
pub const SLOT_SELECTED_BORDER: Color = Color::new(0.855, 0.737, 0.502, 1.0); // rgba(218, 188, 128, 255)
pub const SLOT_DRAG_SOURCE: Color = Color::new(0.314, 0.392, 0.627, 0.706); // rgba(80, 100, 160, 180)

// Equipment section
pub const EQUIP_SLOT_EMPTY: Color = Color::new(0.110, 0.110, 0.165, 1.0);   // rgba(28, 28, 42, 255)
pub const EQUIP_ACCENT: Color = Color::new(0.424, 0.345, 0.580, 1.0);       // rgba(108, 88, 148, 255)

// Header/Footer
pub const HEADER_BG: Color = Color::new(0.141, 0.125, 0.165, 1.0);          // rgba(36, 32, 42, 255)
pub const HEADER_BORDER: Color = Color::new(0.463, 0.384, 0.267, 1.0);      // rgba(118, 98, 68, 255)
pub const FOOTER_BG: Color = Color::new(0.094, 0.086, 0.110, 1.0);          // rgba(24, 22, 28, 255)

// Text colors
pub const TEXT_TITLE: Color = Color::new(0.855, 0.737, 0.502, 1.0);         // rgba(218, 188, 128, 255)
pub const TEXT_NORMAL: Color = Color::new(0.824, 0.824, 0.855, 1.0);        // rgba(210, 210, 218, 255)
pub const TEXT_DIM: Color = Color::new(0.502, 0.502, 0.541, 1.0);           // rgba(128, 128, 138, 255)
pub const TEXT_GOLD: Color = Color::new(1.0, 0.843, 0.314, 1.0);            // rgba(255, 215, 80, 255)

// Tooltip colors
pub const TOOLTIP_BG: Color = Color::new(0.063, 0.063, 0.086, 0.980);       // rgba(16, 16, 22, 250)
pub const TOOLTIP_FRAME: Color = Color::new(0.322, 0.282, 0.227, 1.0);      // rgba(82, 72, 58, 255)

// Item category colors
pub const CATEGORY_EQUIPMENT: Color = Color::new(0.345, 0.549, 0.824, 1.0);  // rgba(88, 140, 210, 255)
pub const CATEGORY_CONSUMABLE: Color = Color::new(0.824, 0.345, 0.345, 1.0); // rgba(210, 88, 88, 255)
pub const CATEGORY_MATERIAL: Color = Color::new(0.620, 0.620, 0.659, 1.0);   // rgba(158, 158, 168, 255)
pub const CATEGORY_QUEST: Color = Color::new(1.0, 0.824, 0.314, 1.0);        // rgba(255, 210, 80, 255)

// ============================================================================
// Health Bar Colors - Ornate Medieval Style
// ============================================================================

// Health bar frame (bronze-tinted dark metal)
pub const HEALTHBAR_FRAME_DARK: Color = Color::new(0.18, 0.14, 0.10, 1.0);   // rgba(46, 36, 26, 255)
pub const HEALTHBAR_FRAME_MID: Color = Color::new(0.35, 0.27, 0.18, 1.0);    // rgba(89, 69, 46, 255)
pub const HEALTHBAR_FRAME_LIGHT: Color = Color::new(0.55, 0.43, 0.28, 1.0);  // rgba(140, 110, 72, 255)
pub const HEALTHBAR_FRAME_ACCENT: Color = Color::new(0.72, 0.58, 0.38, 1.0); // rgba(184, 148, 97, 255) gold highlight

// Health bar background (recessed dark)
pub const HEALTHBAR_BG_OUTER: Color = Color::new(0.04, 0.04, 0.05, 1.0);     // rgba(10, 10, 13, 255)
pub const HEALTHBAR_BG_INNER: Color = Color::new(0.08, 0.07, 0.09, 1.0);     // rgba(20, 18, 23, 255)

// Health colors - rich jewel tones
pub const HEALTH_GREEN_DARK: Color = Color::new(0.12, 0.45, 0.22, 1.0);      // rgba(31, 115, 56, 255) emerald base
pub const HEALTH_GREEN_MID: Color = Color::new(0.20, 0.62, 0.32, 1.0);       // rgba(51, 158, 82, 255) emerald bright
pub const HEALTH_GREEN_LIGHT: Color = Color::new(0.35, 0.78, 0.48, 1.0);     // rgba(89, 199, 122, 255) emerald highlight

pub const HEALTH_YELLOW_DARK: Color = Color::new(0.65, 0.45, 0.08, 1.0);     // rgba(166, 115, 20, 255) amber base
pub const HEALTH_YELLOW_MID: Color = Color::new(0.85, 0.62, 0.12, 1.0);      // rgba(217, 158, 31, 255) amber bright
pub const HEALTH_YELLOW_LIGHT: Color = Color::new(0.95, 0.78, 0.25, 1.0);    // rgba(242, 199, 64, 255) amber highlight

pub const HEALTH_RED_DARK: Color = Color::new(0.55, 0.12, 0.12, 1.0);        // rgba(140, 31, 31, 255) ruby base
pub const HEALTH_RED_MID: Color = Color::new(0.75, 0.18, 0.18, 1.0);         // rgba(191, 46, 46, 255) ruby bright
pub const HEALTH_RED_LIGHT: Color = Color::new(0.90, 0.35, 0.35, 1.0);       // rgba(230, 89, 89, 255) ruby highlight

// Experience bar colors - gold/amber theme
pub const EXP_BAR_FILL_DARK: Color = Color::new(0.55, 0.40, 0.08, 1.0);      // rgba(140, 102, 20, 255) gold base
pub const EXP_BAR_FILL_MID: Color = Color::new(0.75, 0.55, 0.12, 1.0);       // rgba(191, 140, 31, 255) gold bright
pub const EXP_BAR_FILL_LIGHT: Color = Color::new(0.90, 0.70, 0.25, 1.0);     // rgba(230, 179, 64, 255) gold highlight

// ============================================================================
// Layout Constants
// ============================================================================

pub const INV_WIDTH: f32 = 460.0;
pub const INV_HEIGHT: f32 = 360.0;
pub const HEADER_HEIGHT: f32 = 40.0;
pub const FOOTER_HEIGHT: f32 = 30.0;
pub const GRID_PADDING: f32 = 15.0;
pub const INV_SLOT_SIZE: f32 = 48.0;
pub const SLOT_SPACING: f32 = 4.0;
pub const EQUIP_PANEL_WIDTH: f32 = 150.0;
pub const EQUIP_SLOT_SIZE: f32 = 38.0;
pub const EQUIP_SLOT_SPACING: f32 = 4.0;
pub const FRAME_THICKNESS: f32 = 4.0;
pub const CORNER_ACCENT_SIZE: f32 = 8.0;

// Tab Button Constants (standardized across panels)
pub const TAB_HEIGHT: f32 = 28.0;
pub const TAB_FONT_SIZE: f32 = 16.0;

// Item Row Constants
pub const ITEM_ICON_SIZE: f32 = 32.0;
pub const ITEM_TEXT_OFFSET: f32 = 48.0;  // icon padding (8) + icon size (32) + gap (8)

// Transaction Bar Constants
pub const TRANSACTION_BAR_HEIGHT: f32 = 100.0;

// Experience Bar Constants
pub const EXP_BAR_HEIGHT: f32 = 16.0;
pub const EXP_BAR_GAP: f32 = 8.0;  // Gap between exp bar and UI elements above

// Menu Button Constants
pub const MENU_BUTTON_SIZE: f32 = 40.0;
pub const MENU_BUTTON_SPACING: f32 = 4.0;

// ============================================================================
// Shared Types
// ============================================================================

/// Slot visual state for rendering
#[derive(Clone, Copy, PartialEq)]
pub enum SlotState {
    Normal,
    Hovered,
    Dragging,
}
