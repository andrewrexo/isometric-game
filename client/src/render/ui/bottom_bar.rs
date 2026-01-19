//! Bottom bar UI components: experience bar and menu buttons

use macroquad::prelude::*;
use crate::game::GameState;
use crate::ui::{UiElementId, UiLayout};
use super::super::Renderer;
use super::common::*;

/// Menu button icon frame indices in background_icons.png
/// Order: inventory, character, settings, skills, social
const ICON_INVENTORY: usize = 0;
const ICON_CHARACTER: usize = 1;
const ICON_SETTINGS: usize = 2;
const ICON_SKILLS: usize = 3;
const ICON_SOCIAL: usize = 4;

/// Icon dimensions in the sprite sheet (160x32 total, 5 icons = 32x32 each)
const ICON_SIZE: f32 = 32.0;

impl Renderer {
    /// Returns the Y position where the exp bar starts (for other UI to position above it)
    pub fn get_exp_bar_top(&self) -> f32 {
        screen_height() - EXP_BAR_HEIGHT
    }

    /// Render the experience bar at the bottom of the screen
    pub(crate) fn render_exp_bar(&self, state: &GameState) {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // Bar fills full width, sits at very bottom
        let bar_height = EXP_BAR_HEIGHT;
        let bar_x = 0.0;
        let bar_y = screen_h - bar_height;
        let bar_width = screen_w;

        // Background
        draw_rectangle(bar_x, bar_y, bar_width, bar_height, PANEL_BG_DARK);

        // Top border only
        let border_color = Color::new(FRAME_MID.r, FRAME_MID.g, FRAME_MID.b, 0.8);
        draw_line(bar_x, bar_y, bar_x + bar_width, bar_y, 1.0, border_color);

        // Get player skill data - show total level and combat level
        let (combat_level, total_level, avg_progress) = if let Some(player) = state.get_local_player() {
            // Calculate average progress across combat skills (HP + Combat)
            let hp_prog = player.skills.hitpoints.level_progress();
            let combat_prog = player.skills.combat.level_progress();
            let avg = (hp_prog + combat_prog) / 2.0;
            (player.combat_level(), player.skills.total_level(), avg)
        } else {
            (6, 13, 0.0) // Default: HP 10 + Combat 3 = 13, combat level = (10+3)/2 = 6
        };

        // Draw fill bar showing average skill progress
        let fill_width = bar_width * avg_progress;
        if fill_width > 0.0 {
            let exp_green = Color::new(0.18, 0.72, 0.35, 1.0);
            draw_rectangle(bar_x, bar_y + 1.0, fill_width, bar_height - 1.0, exp_green);
        }

        // Show combat level and total level
        let exp_text = format!("Combat Lv: {}  |  Total Lv: {}", combat_level, total_level);
        let text_dims = self.measure_text_sharp(&exp_text, 16.0);
        let text_x = (bar_width - text_dims.width) / 2.0;
        let text_y = bar_y + (bar_height + 12.0) / 2.0 - 2.0;

        // Text shadow
        self.draw_text_sharp(&exp_text, text_x + 1.0, text_y + 1.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.8));
        // Text
        self.draw_text_sharp(&exp_text, text_x, text_y, 16.0, TEXT_NORMAL);
    }

    /// Render the menu buttons in the bottom-right corner
    pub(crate) fn render_menu_buttons(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let screen_w = screen_width();

        // Position above the exp bar with gap
        let exp_bar_top = self.get_exp_bar_top();
        let button_y = exp_bar_top - EXP_BAR_GAP - MENU_BUTTON_SIZE;

        // 5 buttons: Inventory, Character, Skills, Social, Settings
        let num_buttons = 5;
        let total_width = num_buttons as f32 * MENU_BUTTON_SIZE + (num_buttons - 1) as f32 * MENU_BUTTON_SPACING;

        // Right-aligned with padding
        let start_x = screen_w - total_width - 8.0;

        // Buttons with their icon frame indices
        let buttons = [
            (UiElementId::MenuButtonInventory, ICON_INVENTORY, state.ui_state.inventory_open),
            (UiElementId::MenuButtonCharacter, ICON_CHARACTER, state.ui_state.character_open),
            (UiElementId::MenuButtonSkills, ICON_SKILLS, state.ui_state.skills_open),
            (UiElementId::MenuButtonSocial, ICON_SOCIAL, state.ui_state.social_open),
            (UiElementId::MenuButtonSettings, ICON_SETTINGS, state.ui_state.escape_menu_open),
        ];

        for (i, (element_id, icon_frame, is_active)) in buttons.iter().enumerate() {
            let x = start_x + i as f32 * (MENU_BUTTON_SIZE + MENU_BUTTON_SPACING);
            let y = button_y;

            // Register bounds for hit detection
            let bounds = Rect::new(x, y, MENU_BUTTON_SIZE, MENU_BUTTON_SIZE);
            layout.add(element_id.clone(), bounds);

            // Check if hovered
            let is_hovered = hovered.as_ref() == Some(element_id);

            // Draw button with icon
            self.draw_menu_button_icon(x, y, MENU_BUTTON_SIZE, *icon_frame, is_hovered, *is_active);
        }
    }

    /// Draw a single menu button with an icon from the sprite sheet
    fn draw_menu_button_icon(&self, x: f32, y: f32, size: f32, icon_frame: usize, is_hovered: bool, is_active: bool) {
        // Frame colors based on state
        let (bg_color, border_color) = if is_active {
            // Active state - brighter
            (SLOT_HOVER_BG, SLOT_SELECTED_BORDER)
        } else if is_hovered {
            // Hover state
            (SLOT_HOVER_BG, SLOT_HOVER_BORDER)
        } else {
            // Normal state
            (SLOT_BG_EMPTY, SLOT_BORDER)
        };

        // Outer border with slight transparency
        let border_alpha = Color::new(border_color.r, border_color.g, border_color.b, 0.9);
        draw_rectangle(x - 1.0, y - 1.0, size + 2.0, size + 2.0, border_alpha);

        // Background
        let bg_alpha = Color::new(bg_color.r, bg_color.g, bg_color.b, 0.85);
        draw_rectangle(x, y, size, size, bg_alpha);

        // Inner shadow for depth
        draw_rectangle(x, y, size, 2.0, SLOT_INNER_SHADOW);
        draw_rectangle(x, y, 2.0, size, SLOT_INNER_SHADOW);

        // Bottom/right highlight
        if is_active {
            let accent = Color::new(FRAME_ACCENT.r, FRAME_ACCENT.g, FRAME_ACCENT.b, 0.5);
            draw_line(x + 2.0, y + size - 1.0, x + size - 2.0, y + size - 1.0, 1.0, accent);
            draw_line(x + size - 1.0, y + 2.0, x + size - 1.0, y + size - 2.0, 1.0, accent);
        }

        // Draw icon from sprite sheet if available
        if let Some(ref texture) = self.menu_button_icons {
            // Calculate source rectangle for this frame
            let src_x = icon_frame as f32 * ICON_SIZE;
            let src_rect = Rect::new(src_x, 0.0, ICON_SIZE, ICON_SIZE);

            // Center icon in button (32x32 icon in 40x40 button = 4px padding each side)
            let icon_x = x + (size - ICON_SIZE) / 2.0;
            let icon_y = y + (size - ICON_SIZE) / 2.0;

            // Tint based on state
            let tint = if is_active {
                TEXT_GOLD
            } else if is_hovered {
                TEXT_TITLE
            } else {
                TEXT_NORMAL
            };

            draw_texture_ex(
                texture,
                icon_x,
                icon_y,
                tint,
                DrawTextureParams {
                    source: Some(src_rect),
                    dest_size: Some(Vec2::new(ICON_SIZE, ICON_SIZE)),
                    ..Default::default()
                },
            );
        } else {
            // Fallback: draw text label if texture not loaded
            let labels = ["I", "C", "K", "S", "O"]; // Inventory, Character, sKills, Social, Options
            let label = labels.get(icon_frame).unwrap_or(&"?");

            let text_dims = self.measure_text_sharp(label, 18.0);
            let text_x = x + (size - text_dims.width) / 2.0;
            let text_y = y + (size + 12.0) / 2.0;

            let text_color = if is_active {
                TEXT_GOLD
            } else if is_hovered {
                TEXT_TITLE
            } else {
                TEXT_NORMAL
            };

            self.draw_text_sharp(label, text_x, text_y, 18.0, text_color);
        }
    }
}
