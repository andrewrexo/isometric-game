//! Quick slots bar rendering

use macroquad::prelude::*;
use crate::game::{GameState, DragSource};
use crate::ui::{UiElementId, UiLayout};
use super::super::Renderer;
use super::super::isometric::world_to_screen;
use super::common::*;

impl Renderer {
    pub(crate) fn render_quick_slots(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        // Smaller slot size for quick bar (6px less than inventory)
        let slot_size = INV_SLOT_SIZE - 6.0;
        let padding = SLOT_SPACING;
        let total_width = 5.0 * (slot_size + padding) - padding;

        // Add some padding for the background panel
        let panel_padding = 5.0;
        let panel_width = total_width + panel_padding * 2.0;
        let panel_height = slot_size + panel_padding * 2.0;

        let panel_x = (screen_width() - panel_width) / 2.0;
        let panel_y = screen_height() - panel_height - 10.0;

        // Draw semi-transparent background panel
        let frame_outer_alpha = Color::new(FRAME_OUTER.r, FRAME_OUTER.g, FRAME_OUTER.b, 0.7);
        let panel_bg_alpha = Color::new(PANEL_BG_MID.r, PANEL_BG_MID.g, PANEL_BG_MID.b, 0.75);
        draw_rectangle(panel_x - 1.0, panel_y - 1.0, panel_width + 2.0, panel_height + 2.0, frame_outer_alpha);
        draw_rectangle(panel_x, panel_y, panel_width, panel_height, panel_bg_alpha);

        // Inner highlight (also semi-transparent)
        let frame_mid_alpha = Color::new(FRAME_MID.r, FRAME_MID.g, FRAME_MID.b, 0.6);
        draw_line(panel_x + 1.0, panel_y + 1.0, panel_x + panel_width - 1.0, panel_y + 1.0, 1.0, frame_mid_alpha);

        let start_x = panel_x + panel_padding;
        let start_y = panel_y + panel_padding;

        for i in 0..5 {
            let x = start_x + i as f32 * (slot_size + padding);
            let y = start_y;

            // Register slot bounds for hit detection
            let bounds = Rect::new(x, y, slot_size, slot_size);
            layout.add(UiElementId::QuickSlot(i), bounds);

            // Check if this slot is hovered
            let is_hovered = matches!(hovered, Some(UiElementId::QuickSlot(idx)) if *idx == i);

            // Check if this slot is being dragged (quick slots are first 5 inventory slots)
            let is_dragging = matches!(&state.ui_state.drag_state, Some(drag) if matches!(&drag.source, DragSource::Inventory(idx) if *idx == i));

            // Determine slot state
            let slot_state = if is_dragging {
                SlotState::Dragging
            } else if is_hovered {
                SlotState::Hovered
            } else {
                SlotState::Normal
            };

            // Draw the slot with bevel effect (matching inventory style)
            let has_item = state.inventory.slots[i].is_some();
            self.draw_inventory_slot(x, y, slot_size, has_item, slot_state);

            // Draw item if present (hide if being dragged)
            if let Some(slot) = &state.inventory.slots[i] {
                if !is_dragging {
                    self.draw_item_icon(&slot.item_id, x, y, slot_size, slot_size, state, false);

                    // Quantity badge (bottom-left with shadow)
                    if slot.quantity > 1 {
                        let qty_text = slot.quantity.to_string();
                        self.draw_text_sharp(&qty_text, x + 3.0, y + slot_size - 2.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.8));
                        self.draw_text_sharp(&qty_text, x + 2.0, y + slot_size - 3.0, 16.0, TEXT_NORMAL);
                    }
                }
            }

            // Slot number badge (top-right)
            let num_text = (i + 1).to_string();
            let text_w = self.measure_text_sharp(&num_text, 16.0).width;
            let badge_w = text_w + 2.0;
            let badge_h = 13.0;
            let num_x = x + slot_size - badge_w - 1.0;
            let num_y = y + 1.0;
            draw_rectangle(num_x, num_y, badge_w, badge_h, Color::new(0.0, 0.0, 0.0, 0.5));
            self.draw_text_sharp(&num_text, num_x + 1.0, num_y + 11.0, 16.0, TEXT_NORMAL);
        }
    }

    pub(crate) fn render_ground_item_overlays(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let zoom = state.camera.zoom;

        for (item_id, item) in &state.ground_items {
            let (screen_x, screen_y) = world_to_screen(item.x, item.y, &state.camera);

            // Clickable area - centered on where items actually render (slightly above tile center)
            let click_width = 44.0 * zoom;
            let click_height = 28.0 * zoom;
            let bounds = Rect::new(
                screen_x - click_width / 2.0,
                screen_y - click_height,
                click_width,
                click_height,
            );
            layout.add(UiElementId::GroundItem(item_id.clone()), bounds);

            // Check if hovered
            let is_hovered = matches!(hovered, Some(UiElementId::GroundItem(id)) if id == item_id);

            if is_hovered {
                // Draw tile hover effect
                self.render_tile_hover(item.x as i32, item.y as i32, &state.camera);

                // Get item definition for display name
                let item_def = state.item_registry.get_or_placeholder(&item.item_id);

                // Build label text
                let label = if item.quantity > 1 {
                    format!("{} (x{})", item_def.display_name, item.quantity)
                } else {
                    item_def.display_name.clone()
                };

                // Draw label just above the clickable area
                let label_width = self.measure_text_sharp(&label, 16.0).width;
                let label_x = screen_x - label_width / 2.0;
                // Gold piles sit lower, so offset label down by 12px
                let gold_offset = if item.item_id == "gold" { 22.0 } else { 0.0 };
                let label_y = screen_y - click_height - 16.0 + gold_offset;

                // Background for readability
                let padding = 4.0;
                draw_rectangle(
                    label_x - padding,
                    label_y - 14.0,
                    label_width + padding * 2.0,
                    18.0,
                    Color::from_rgba(0, 0, 0, 180),
                );

                // Label text
                self.draw_text_sharp(&label, label_x, label_y, 16.0, WHITE);
            }
        }
    }
}
