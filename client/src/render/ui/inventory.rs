//! Inventory panel rendering

use macroquad::prelude::*;
use crate::game::{GameState, DragState, DragSource};
use crate::ui::{UiElementId, UiLayout};
use super::super::Renderer;
use super::common::*;

impl Renderer {
    /// Draw equipment slot with silhouette icon when empty
    pub(crate) fn draw_equipment_slot(&self, x: f32, y: f32, size: f32, slot_type: &str, has_item: bool, is_hovered: bool, is_dragging: bool) {
        // Outer border (purple accent for equipment)
        let border_color = if is_dragging {
            SLOT_SELECTED_BORDER
        } else if is_hovered {
            EQUIP_ACCENT
        } else {
            SLOT_BORDER
        };
        draw_rectangle(x, y, size, size, border_color);

        // Inner background
        let bg = if is_dragging {
            SLOT_DRAG_SOURCE
        } else if is_hovered {
            SLOT_HOVER_BG
        } else {
            EQUIP_SLOT_EMPTY
        };
        draw_rectangle(x + 1.0, y + 1.0, size - 2.0, size - 2.0, bg);

        // Inner bevel effect
        draw_line(x + 2.0, y + 2.0, x + size - 2.0, y + 2.0, 2.0, SLOT_INNER_SHADOW);
        draw_line(x + 2.0, y + 2.0, x + 2.0, y + size - 2.0, 2.0, SLOT_INNER_SHADOW);

        // Draw silhouette if empty (and not dragging)
        if !has_item && !is_dragging {
            let center_x = x + size / 2.0;
            let center_y = y + size / 2.0;
            let icon_color = Color::new(0.188, 0.188, 0.227, 1.0); // rgba(48, 48, 58, 255)

            match slot_type {
                "head" => {
                    // Helmet silhouette (rounded head shape)
                    draw_rectangle(center_x - 8.0, center_y - 8.0, 16.0, 14.0, icon_color);
                    draw_rectangle(center_x - 10.0, center_y - 4.0, 20.0, 8.0, icon_color);
                    draw_rectangle(center_x - 6.0, center_y - 12.0, 12.0, 6.0, icon_color);
                },
                "body" => {
                    // Armor silhouette (torso shape)
                    draw_rectangle(center_x - 8.0, center_y - 10.0, 16.0, 20.0, icon_color);
                    draw_rectangle(center_x - 12.0, center_y - 6.0, 5.0, 12.0, icon_color);
                    draw_rectangle(center_x + 7.0, center_y - 6.0, 5.0, 12.0, icon_color);
                },
                "weapon" => {
                    // Sword silhouette
                    draw_rectangle(center_x - 2.0, center_y - 14.0, 4.0, 24.0, icon_color);
                    draw_rectangle(center_x - 8.0, center_y + 4.0, 16.0, 4.0, icon_color);
                    draw_rectangle(center_x - 3.0, center_y + 8.0, 6.0, 4.0, icon_color);
                },
                "back" => {
                    // Cape/backpack silhouette
                    draw_rectangle(center_x - 10.0, center_y - 10.0, 20.0, 6.0, icon_color);
                    draw_rectangle(center_x - 8.0, center_y - 4.0, 16.0, 16.0, icon_color);
                    draw_rectangle(center_x - 6.0, center_y + 10.0, 12.0, 4.0, icon_color);
                },
                "feet" => {
                    // Boots silhouette
                    draw_rectangle(center_x - 8.0, center_y - 4.0, 6.0, 12.0, icon_color);
                    draw_rectangle(center_x + 2.0, center_y - 4.0, 6.0, 12.0, icon_color);
                    draw_rectangle(center_x - 10.0, center_y + 6.0, 9.0, 4.0, icon_color);
                    draw_rectangle(center_x + 1.0, center_y + 6.0, 9.0, 4.0, icon_color);
                },
                "ring" => {
                    // Ring silhouette (circular band)
                    draw_rectangle(center_x - 6.0, center_y - 8.0, 12.0, 4.0, icon_color);
                    draw_rectangle(center_x - 8.0, center_y - 4.0, 4.0, 8.0, icon_color);
                    draw_rectangle(center_x + 4.0, center_y - 4.0, 4.0, 8.0, icon_color);
                    draw_rectangle(center_x - 6.0, center_y + 4.0, 12.0, 4.0, icon_color);
                    // Gem on top
                    draw_rectangle(center_x - 3.0, center_y - 12.0, 6.0, 6.0, icon_color);
                },
                "gloves" => {
                    // Glove silhouette (hand shape)
                    draw_rectangle(center_x - 8.0, center_y - 2.0, 16.0, 12.0, icon_color);
                    // Fingers
                    draw_rectangle(center_x - 8.0, center_y - 10.0, 3.0, 10.0, icon_color);
                    draw_rectangle(center_x - 4.0, center_y - 12.0, 3.0, 12.0, icon_color);
                    draw_rectangle(center_x, center_y - 12.0, 3.0, 12.0, icon_color);
                    draw_rectangle(center_x + 4.0, center_y - 10.0, 3.0, 10.0, icon_color);
                    // Thumb
                    draw_rectangle(center_x + 8.0, center_y - 4.0, 4.0, 8.0, icon_color);
                },
                "necklace" => {
                    // Necklace silhouette (pendant on chain)
                    // Chain part (U shape)
                    draw_rectangle(center_x - 8.0, center_y - 10.0, 3.0, 8.0, icon_color);
                    draw_rectangle(center_x + 5.0, center_y - 10.0, 3.0, 8.0, icon_color);
                    draw_rectangle(center_x - 6.0, center_y - 2.0, 12.0, 3.0, icon_color);
                    // Pendant (diamond shape)
                    draw_rectangle(center_x - 4.0, center_y + 1.0, 8.0, 8.0, icon_color);
                    draw_rectangle(center_x - 2.0, center_y + 9.0, 4.0, 4.0, icon_color);
                },
                "belt" => {
                    // Belt silhouette (horizontal band with buckle)
                    draw_rectangle(center_x - 12.0, center_y - 3.0, 24.0, 6.0, icon_color);
                    // Buckle (square with center)
                    draw_rectangle(center_x - 5.0, center_y - 6.0, 10.0, 12.0, icon_color);
                    draw_rectangle(center_x - 2.0, center_y - 3.0, 4.0, 6.0, EQUIP_SLOT_EMPTY);
                },
                _ => {}
            }
        }
    }

    pub(crate) fn render_inventory(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let inv_x = (screen_width() - INV_WIDTH) / 2.0;
        let inv_y = (screen_height() - INV_HEIGHT) / 2.0;

        // Draw panel frame with corner accents
        self.draw_panel_frame(inv_x, inv_y, INV_WIDTH, INV_HEIGHT);
        self.draw_corner_accents(inv_x, inv_y, INV_WIDTH, INV_HEIGHT);

        // ===== HEADER SECTION =====
        let header_x = inv_x + FRAME_THICKNESS;
        let header_y = inv_y + FRAME_THICKNESS;
        let header_w = INV_WIDTH - FRAME_THICKNESS * 2.0;

        // Header background
        draw_rectangle(header_x, header_y, header_w, HEADER_HEIGHT, HEADER_BG);

        // Header bottom separator
        draw_line(header_x + 10.0, header_y + HEADER_HEIGHT, header_x + header_w - 10.0, header_y + HEADER_HEIGHT, 2.0, HEADER_BORDER);

        // Decorative dots on separator
        let dot_spacing = 50.0;
        let num_dots = ((header_w - 40.0) / dot_spacing) as i32;
        let start_dot_x = header_x + 20.0;
        for i in 0..num_dots {
            let dot_x = start_dot_x + i as f32 * dot_spacing;
            draw_rectangle(dot_x - 1.5, header_y + HEADER_HEIGHT - 1.5, 3.0, 3.0, FRAME_ACCENT);
        }

        // Title text
        self.draw_text_sharp("INVENTORY", header_x + 12.0, header_y + 26.0, 16.0, TEXT_TITLE);

        // Gold display (right side)
        let gold_text = format!("{}g", state.inventory.gold);
        let gold_width = self.measure_text_sharp(&gold_text, 16.0).width;

        // Nugget icon size and spacing
        let icon_size = 12.0;
        let icon_margin = 4.0;
        let coin_x = header_x + header_w - 12.0 - gold_width - icon_size - icon_margin;

        // Gold nugget icon
        if let Some(texture) = &self.gold_nugget_texture {
            draw_texture_ex(
                texture,
                coin_x,
                header_y + (HEADER_HEIGHT - icon_size) / 2.0 + 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(icon_size, icon_size)),
                    ..Default::default()
                },
            );
        }

        self.draw_text_sharp(&gold_text, coin_x + icon_size + icon_margin, header_y + 26.0, 16.0, TEXT_GOLD);

        // ===== INVENTORY GRID (left side) =====
        let content_y = inv_y + FRAME_THICKNESS + HEADER_HEIGHT + 10.0;
        let grid_x = inv_x + GRID_PADDING;
        let grid_y = content_y;
        let slots_per_row = 5;

        for i in 0..20 {
            let row = i / slots_per_row;
            let col = i % slots_per_row;
            let x = grid_x + col as f32 * (INV_SLOT_SIZE + SLOT_SPACING);
            let y = grid_y + row as f32 * (INV_SLOT_SIZE + SLOT_SPACING);

            // Register slot bounds for hit detection
            let bounds = Rect::new(x, y, INV_SLOT_SIZE, INV_SLOT_SIZE);
            layout.add(UiElementId::InventorySlot(i), bounds);

            // Check if this slot is hovered
            let is_hovered = matches!(hovered, Some(UiElementId::InventorySlot(idx)) if *idx == i);

            // Check if this slot is being dragged
            let is_dragging = matches!(&state.ui_state.drag_state, Some(drag) if matches!(&drag.source, DragSource::Inventory(idx) if *idx == i));

            // Determine slot state
            let slot_state = if is_dragging {
                SlotState::Dragging
            } else if is_hovered {
                SlotState::Hovered
            } else {
                SlotState::Normal
            };

            // Draw the slot with bevel effect
            let has_item = state.inventory.slots[i].is_some();
            self.draw_inventory_slot(x, y, INV_SLOT_SIZE, has_item, slot_state);

            // Draw item if present (hide if being dragged)
            if let Some(slot) = &state.inventory.slots[i] {
                if !is_dragging {
                    self.draw_item_icon(&slot.item_id, x, y, INV_SLOT_SIZE, INV_SLOT_SIZE, state);

                    // Quantity badge (bottom-left with shadow)
                    if slot.quantity > 1 {
                        let qty_text = slot.quantity.to_string();
                        // Shadow
                        self.draw_text_sharp(&qty_text, x + 3.0, y + INV_SLOT_SIZE - 2.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.8));
                        // Text
                        self.draw_text_sharp(&qty_text, x + 2.0, y + INV_SLOT_SIZE - 3.0, 16.0, TEXT_NORMAL);
                    }
                }
            }

            // Show slot number badge for first 5 (quick slots)
            if i < 5 {
                let num_text = (i + 1).to_string();
                let text_w = self.measure_text_sharp(&num_text, 16.0).width;
                let badge_w = text_w + 2.0;
                let badge_h = 13.0;
                let num_x = x + INV_SLOT_SIZE - badge_w - 1.0;
                let num_y = y + 1.0;
                draw_rectangle(num_x, num_y, badge_w, badge_h, Color::new(0.0, 0.0, 0.0, 0.5));
                self.draw_text_sharp(&num_text, num_x + 1.0, num_y + 11.0, 16.0, TEXT_DIM);
            }
        }

        // ===== VERTICAL DIVIDER =====
        let divider_x = inv_x + GRID_PADDING + 5.0 * (INV_SLOT_SIZE + SLOT_SPACING) + 8.0;
        let divider_top = content_y;
        let divider_bottom = inv_y + INV_HEIGHT - FRAME_THICKNESS - FOOTER_HEIGHT - 5.0;

        // Divider line with highlight
        draw_line(divider_x, divider_top, divider_x, divider_bottom, 2.0, FRAME_MID);
        draw_line(divider_x + 1.0, divider_top, divider_x + 1.0, divider_bottom, 1.0, FRAME_INNER);

        // ===== EQUIPMENT PANEL (right side) =====
        let equip_x = divider_x + 12.0;
        let equip_y = content_y;
        let equip_panel_w = EQUIP_PANEL_WIDTH - 20.0;

        // Equipment header
        self.draw_text_sharp("GEAR", equip_x + (equip_panel_w - self.measure_text_sharp("GEAR", 16.0).width) / 2.0, equip_y + 16.0, 16.0, TEXT_TITLE);

        // Decorative line under header
        draw_line(equip_x + 2.0, equip_y + 22.0, equip_x + equip_panel_w - 2.0, equip_y + 22.0, 1.0, HEADER_BORDER);

        // Equipment slots - arranged in body-shaped layout
        let slot_step = EQUIP_SLOT_SIZE + EQUIP_SLOT_SPACING;
        let grid_width = 3.0 * EQUIP_SLOT_SIZE + 2.0 * EQUIP_SLOT_SPACING;
        let grid_start_x = equip_x + (equip_panel_w - grid_width) / 2.0;
        let grid_start_y = equip_y + 28.0;

        let equipment_slots: [(&str, i32, i32); 9] = [
            ("head", 1, 0),
            ("back", 0, 1),
            ("body", 1, 1),
            ("weapon", 2, 1),
            ("gloves", 0, 2),
            ("ring", 2, 2),
            ("necklace", 0, 3),
            ("belt", 2, 3),
            ("feet", 1, 4),
        ];

        for (slot_type, col, row) in equipment_slots.iter() {
            let slot_x = grid_start_x + (*col as f32) * slot_step;
            let slot_y = grid_start_y + (*row as f32) * slot_step;

            let bounds = Rect::new(slot_x, slot_y, EQUIP_SLOT_SIZE, EQUIP_SLOT_SIZE);
            layout.add(UiElementId::EquipmentSlot(slot_type.to_string()), bounds);

            let is_hovered = matches!(hovered, Some(UiElementId::EquipmentSlot(s)) if s == *slot_type);
            let is_dragging = matches!(&state.ui_state.drag_state, Some(drag) if matches!(&drag.source, DragSource::Equipment(s) if s == *slot_type));

            let has_item = state.get_local_player().map(|p| {
                match *slot_type {
                    "head" => p.equipped_head.is_some(),
                    "body" => p.equipped_body.is_some(),
                    "weapon" => p.equipped_weapon.is_some(),
                    "back" => p.equipped_back.is_some(),
                    "feet" => p.equipped_feet.is_some(),
                    "ring" => p.equipped_ring.is_some(),
                    "gloves" => p.equipped_gloves.is_some(),
                    "necklace" => p.equipped_necklace.is_some(),
                    "belt" => p.equipped_belt.is_some(),
                    _ => false,
                }
            }).unwrap_or(false);

            self.draw_equipment_slot(slot_x, slot_y, EQUIP_SLOT_SIZE, slot_type, has_item, is_hovered, is_dragging);

            if !is_dragging {
                if let Some(local_player) = state.get_local_player() {
                    let item_id = match *slot_type {
                        "head" => local_player.equipped_head.as_ref(),
                        "body" => local_player.equipped_body.as_ref(),
                        "weapon" => local_player.equipped_weapon.as_ref(),
                        "back" => local_player.equipped_back.as_ref(),
                        "feet" => local_player.equipped_feet.as_ref(),
                        "ring" => local_player.equipped_ring.as_ref(),
                        "gloves" => local_player.equipped_gloves.as_ref(),
                        "necklace" => local_player.equipped_necklace.as_ref(),
                        "belt" => local_player.equipped_belt.as_ref(),
                        _ => None,
                    };
                    if let Some(id) = item_id {
                        self.draw_item_icon(id, slot_x, slot_y, EQUIP_SLOT_SIZE, EQUIP_SLOT_SIZE, state);
                    }
                }
            }
        }

        // ===== FOOTER SECTION =====
        let footer_x = inv_x + FRAME_THICKNESS;
        let footer_y = inv_y + INV_HEIGHT - FRAME_THICKNESS - FOOTER_HEIGHT;
        let footer_w = INV_WIDTH - FRAME_THICKNESS * 2.0;

        draw_rectangle(footer_x, footer_y, footer_w, FOOTER_HEIGHT, FOOTER_BG);
        draw_line(footer_x + 10.0, footer_y, footer_x + footer_w - 10.0, footer_y, 1.0, HEADER_BORDER);

        self.draw_text_sharp("[I] Close", footer_x + 10.0, footer_y + 20.0, 16.0, TEXT_DIM);
        self.draw_text_sharp("Right-click: Options", footer_x + 100.0, footer_y + 20.0, 16.0, Color::new(0.392, 0.392, 0.431, 1.0));
        self.draw_text_sharp("Drag to move", footer_x + 270.0, footer_y + 20.0, 16.0, Color::new(0.314, 0.314, 0.353, 1.0));
    }

    pub(crate) fn draw_item_icon(&self, item_id: &str, x: f32, y: f32, slot_width: f32, slot_height: f32, state: &GameState) {
        if let Some(texture) = self.item_sprites.get(item_id) {
            let icon_width = texture.width();
            let icon_height = texture.height();
            let offset_x = (slot_width - icon_width) / 2.0;
            let offset_y = (slot_height - icon_height) / 2.0;

            draw_texture_ex(
                texture,
                x + offset_x,
                y + offset_y,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            let item_def = state.item_registry.get_or_placeholder(item_id);
            let color = item_def.category_color();
            let icon_width = 32.0;
            let icon_height = 32.0;
            let offset_x = (slot_width - icon_width) / 2.0;
            let offset_y = (slot_height - icon_height) / 2.0;
            draw_rectangle(x + offset_x, y + offset_y, icon_width, icon_height, color);
        }
    }

    pub(crate) fn render_dragged_item(&self, drag: &DragState, state: &GameState) {
        let (mx, my) = mouse_position();
        let slot_size = INV_SLOT_SIZE;
        let x = mx - slot_size / 2.0;
        let y = my - slot_size / 2.0;

        // Drop shadow
        draw_rectangle(x + 3.0, y + 3.0, slot_size, slot_size, Color::new(0.0, 0.0, 0.0, 0.4));

        // Outer border (gold glow effect)
        draw_rectangle(x - 2.0, y - 2.0, slot_size + 4.0, slot_size + 4.0, SLOT_SELECTED_BORDER);

        // Background
        draw_rectangle(x, y, slot_size, slot_size, SLOT_HOVER_BG);

        // Inner bevel effect
        draw_line(x + 1.0, y + 1.0, x + slot_size - 1.0, y + 1.0, 2.0, SLOT_INNER_SHADOW);
        draw_line(x + 1.0, y + 1.0, x + 1.0, y + slot_size - 1.0, 2.0, SLOT_INNER_SHADOW);

        // Draw the item icon centered on cursor
        self.draw_item_icon(&drag.item_id, x, y, slot_size, slot_size, state);

        // Draw quantity if > 1 (with shadow)
        if drag.quantity > 1 {
            let qty_text = drag.quantity.to_string();
            self.draw_text_sharp(&qty_text, x + 3.0, y + slot_size - 2.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.8));
            self.draw_text_sharp(&qty_text, x + 2.0, y + slot_size - 3.0, 16.0, TEXT_NORMAL);
        }
    }
}
