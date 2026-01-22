//! Gear panel rendering - separate equipment slots panel

use macroquad::prelude::*;
use crate::game::{GameState, DragSource};
use crate::ui::{UiElementId, UiLayout};
use super::super::Renderer;
use super::common::*;

/// Gear panel dimensions
const GEAR_PANEL_PADDING: f32 = 12.0;
const GEAR_HEADER_HEIGHT: f32 = 24.0;
const GEAR_GRID_WIDTH: f32 = 3.0 * EQUIP_SLOT_SIZE + 2.0 * EQUIP_SLOT_SPACING; // 122
const GEAR_GRID_HEIGHT: f32 = 5.0 * EQUIP_SLOT_SIZE + 4.0 * EQUIP_SLOT_SPACING; // 206
const GEAR_PANEL_WIDTH: f32 = 240.0; // Unified width to match inventory and other UI panels
const GEAR_PANEL_HEIGHT: f32 = FRAME_THICKNESS * 2.0 + GEAR_HEADER_HEIGHT + GEAR_PANEL_PADDING + GEAR_GRID_HEIGHT + GEAR_PANEL_PADDING; // ~262
const STATS_SECTION_GAP: f32 = 8.0; // Gap between equipment grid and stats

impl Renderer {
    /// Render the gear panel when open
    pub(crate) fn render_gear_panel(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        if !state.ui_state.gear_panel_open {
            return;
        }

        let screen_w = screen_width();
        let screen_h = screen_height();

        // Position panel on right side, above the menu buttons (align with button right edge)
        let panel_x = screen_w - GEAR_PANEL_WIDTH - 8.0;
        let button_area_height = MENU_BUTTON_SIZE + EXP_BAR_GAP;
        let panel_y = screen_h - button_area_height - GEAR_PANEL_HEIGHT - 8.0;

        // Draw panel frame
        self.draw_panel_frame(panel_x, panel_y, GEAR_PANEL_WIDTH, GEAR_PANEL_HEIGHT);
        self.draw_corner_accents(panel_x, panel_y, GEAR_PANEL_WIDTH, GEAR_PANEL_HEIGHT);

        // Header
        let header_x = panel_x + FRAME_THICKNESS;
        let header_y = panel_y + FRAME_THICKNESS;
        let header_w = GEAR_PANEL_WIDTH - FRAME_THICKNESS * 2.0;

        draw_rectangle(header_x, header_y, header_w, GEAR_HEADER_HEIGHT, HEADER_BG);
        draw_line(
            header_x + 6.0,
            header_y + GEAR_HEADER_HEIGHT,
            header_x + header_w - 6.0,
            header_y + GEAR_HEADER_HEIGHT,
            1.0,
            HEADER_BORDER,
        );

        // Header text
        let header_text = "CHARACTER";
        let text_dims = self.measure_text_sharp(header_text, 16.0);
        let text_x = header_x + (header_w - text_dims.width) / 2.0;
        self.draw_text_sharp(header_text, text_x, header_y + 17.0, 16.0, TEXT_TITLE);

        // Grid area
        let grid_x = panel_x + FRAME_THICKNESS + GEAR_PANEL_PADDING;
        let grid_y = header_y + GEAR_HEADER_HEIGHT + GEAR_PANEL_PADDING;

        let slot_step = EQUIP_SLOT_SIZE + EQUIP_SLOT_SPACING;

        // Equipment slots - arranged in body-shaped layout
        // Same layout as was in inventory.rs
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
            let slot_x = grid_x + (*col as f32) * slot_step;
            let slot_y = grid_y + (*row as f32) * slot_step;

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
                        self.draw_item_icon(id, slot_x, slot_y, EQUIP_SLOT_SIZE, EQUIP_SLOT_SIZE, state, false);
                    }
                }
            }
        }

        // Stats section - to the right of equipment grid
        let stats_x = grid_x + GEAR_GRID_WIDTH + STATS_SECTION_GAP;
        let stats_y = grid_y;

        // Get player stats
        if let Some(player) = state.get_local_player() {
            let line_height = 24.0;
            let text_x = stats_x + 4.0;
            let mut text_y = stats_y + 18.0;

            // Equipment bonuses
            let atk_bonus = player.attack_bonus(&state.item_registry);
            let str_bonus = player.strength_bonus(&state.item_registry);
            let def_bonus = player.defence_bonus(&state.item_registry);

            // Stats list
            let hp_text = format!("HP  {}/{}", player.hp, player.max_hp);
            self.draw_text_sharp(&hp_text, text_x, text_y, 16.0, HEALTH_GREEN_MID);
            text_y += line_height;

            let combat_text = format!("Lv  {}", player.combat_level());
            self.draw_text_sharp(&combat_text, text_x, text_y, 16.0, TEXT_NORMAL);
            text_y += line_height;

            let atk_text = format!("ATK +{}", atk_bonus);
            self.draw_text_sharp(&atk_text, text_x, text_y, 16.0, CATEGORY_EQUIPMENT);
            text_y += line_height;

            let str_text = format!("STR +{}", str_bonus);
            self.draw_text_sharp(&str_text, text_x, text_y, 16.0, CATEGORY_CONSUMABLE);
            text_y += line_height;

            let def_text = format!("DEF +{}", def_bonus);
            self.draw_text_sharp(&def_text, text_x, text_y, 16.0, CATEGORY_MATERIAL);
        }
    }
}
