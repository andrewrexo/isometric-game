//! Right-click context menu rendering

use macroquad::prelude::*;
use crate::game::{GameState, ContextMenu, ContextMenuTarget};
use crate::ui::{UiElementId, UiLayout};
use super::super::Renderer;
use super::common::*;

impl Renderer {
    /// Render the right-click context menu for items
    pub(crate) fn render_context_menu(&self, menu: &ContextMenu, state: &GameState, layout: &mut UiLayout) {
        let padding = 8.0;
        let header_height = 24.0;
        let option_height = 28.0;
        let menu_width = 120.0;

        // Determine which options to show and header title
        let mut options: Vec<(&str, UiElementId)> = Vec::new();
        let header_title = match &menu.target {
            ContextMenuTarget::EquipmentSlot(_) => {
                options.push(("Unequip", UiElementId::ContextMenuOption(0)));
                "Equipment"
            }
            ContextMenuTarget::Gold => {
                options.push(("Drop", UiElementId::ContextMenuOption(0)));
                "Gold"
            }
            ContextMenuTarget::InventorySlot(slot_index) => {
                // Check if item is equippable
                if let Some(slot) = state.inventory.slots.get(*slot_index).and_then(|s| s.as_ref()) {
                    let item_def = state.item_registry.get_or_placeholder(&slot.item_id);
                    if item_def.equipment.is_some() {
                        options.push(("Equip", UiElementId::ContextMenuOption(0)));
                    }
                }
                options.push(("Drop", UiElementId::ContextMenuOption(options.len())));
                "Item"
            }
        };

        let content_height = options.len() as f32 * option_height + padding;
        let menu_height = header_height + content_height + padding;

        // Position menu at cursor, but keep on screen (pixel-aligned)
        let mut menu_x = menu.x.floor();
        let mut menu_y = menu.y.floor();

        if menu_x + menu_width > screen_width() {
            menu_x = (screen_width() - menu_width - 5.0).floor();
        }
        if menu_y + menu_height > screen_height() {
            menu_y = (screen_height() - menu_height - 5.0).floor();
        }

        // ===== PANEL FRAME =====
        // Outer border
        draw_rectangle(menu_x - 2.0, menu_y - 2.0, menu_width + 4.0, menu_height + 4.0, FRAME_OUTER);
        // Mid frame
        draw_rectangle(menu_x - 1.0, menu_y - 1.0, menu_width + 2.0, menu_height + 2.0, FRAME_MID);
        // Main background
        draw_rectangle(menu_x, menu_y, menu_width, menu_height, PANEL_BG_MID);

        // ===== HEADER =====
        draw_rectangle(menu_x, menu_y, menu_width, header_height, HEADER_BG);
        draw_line(menu_x, menu_y + header_height, menu_x + menu_width, menu_y + header_height, 1.0, HEADER_BORDER);

        // Header title
        self.draw_text_sharp(header_title, (menu_x + 8.0).floor(), (menu_y + 17.0).floor(), 16.0, TEXT_TITLE);

        // Small accent dots on header
        draw_rectangle((menu_x + menu_width - 18.0).floor(), (menu_y + 10.0).floor(), 3.0, 3.0, FRAME_ACCENT);
        draw_rectangle((menu_x + menu_width - 12.0).floor(), (menu_y + 10.0).floor(), 3.0, 3.0, FRAME_ACCENT);

        // ===== OPTIONS =====
        let (mouse_x, mouse_y) = mouse_position();
        let mut y = (menu_y + header_height + padding).floor();

        for (_i, (label, element_id)) in options.iter().enumerate() {
            let option_bounds = Rect::new((menu_x + 4.0).floor(), y, menu_width - 8.0, option_height - 4.0);
            layout.add(element_id.clone(), option_bounds);

            // Check hover
            let is_hovered = mouse_x >= option_bounds.x && mouse_x <= option_bounds.x + option_bounds.w
                && mouse_y >= option_bounds.y && mouse_y <= option_bounds.y + option_bounds.h;

            if is_hovered {
                // Hover background
                draw_rectangle(option_bounds.x, option_bounds.y, option_bounds.w, option_bounds.h, SLOT_HOVER_BG);
            }

            // Label
            let text_color = if is_hovered { TEXT_TITLE } else { TEXT_NORMAL };
            self.draw_text_sharp(label, (option_bounds.x + 10.0).floor(), (y + 16.0).floor(), 16.0, text_color);

            y += option_height;
        }

        // Bottom inner shadow
        draw_line(menu_x + 1.0, menu_y + menu_height - 1.0, menu_x + menu_width - 1.0, menu_y + menu_height - 1.0, 1.0, FRAME_OUTER);
    }
}
