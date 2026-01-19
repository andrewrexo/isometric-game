//! Item tooltip rendering

use macroquad::prelude::*;
use crate::game::GameState;
use crate::ui::UiElementId;
use super::super::Renderer;
use super::common::*;

impl Renderer {
    /// Render tooltip for hovered inventory/quick slot items
    pub(crate) fn render_item_tooltip(&self, state: &GameState) {
        // Check if we're hovering over an inventory, quick slot, or equipment slot
        let (item_id, quantity) = match &state.ui_state.hovered_element {
            Some(UiElementId::InventorySlot(idx)) if state.ui_state.inventory_open => {
                if let Some(slot) = state.inventory.slots.get(*idx).and_then(|s| s.as_ref()) {
                    (slot.item_id.clone(), slot.quantity)
                } else {
                    return;
                }
            }
            Some(UiElementId::QuickSlot(idx)) => {
                if let Some(slot) = state.inventory.slots.get(*idx).and_then(|s| s.as_ref()) {
                    (slot.item_id.clone(), slot.quantity)
                } else {
                    return;
                }
            }
            Some(UiElementId::EquipmentSlot(slot_type)) if state.ui_state.inventory_open => {
                let equipped_item = state.get_local_player().and_then(|p| {
                    match slot_type.as_str() {
                        "head" => p.equipped_head.clone(),
                        "body" => p.equipped_body.clone(),
                        "weapon" => p.equipped_weapon.clone(),
                        "back" => p.equipped_back.clone(),
                        "feet" => p.equipped_feet.clone(),
                        "ring" => p.equipped_ring.clone(),
                        "gloves" => p.equipped_gloves.clone(),
                        "necklace" => p.equipped_necklace.clone(),
                        "belt" => p.equipped_belt.clone(),
                        _ => None,
                    }
                });
                if let Some(id) = equipped_item {
                    (id, 1) // Equipment always has quantity 1
                } else {
                    return;
                }
            }
            _ => return,
        };

        // Get item definition from registry
        let item_def = state.item_registry.get_or_placeholder(&item_id);

        // Get player combat level for requirement checking (single combat skill)
        let player_combat_level = state.get_local_player()
            .map(|p| p.skills.combat.level)
            .unwrap_or(1);

        // Get mouse position for tooltip placement
        let (mouse_x, mouse_y) = mouse_position();

        // Enhanced tooltip styling - use 16pt throughout for crisp rendering
        let padding = 10.0;
        let line_height = 20.0;
        let font_size = 16.0;
        let small_font_size = 16.0;  // Use 16pt for all text (native size)
        let max_tooltip_width = 200.0;
        let text_width_limit = max_tooltip_width - padding * 2.0;

        // Prepare text strings for measurement
        let name_text = if quantity > 1 {
            format!("{} x{}", item_def.display_name, quantity)
        } else {
            item_def.display_name.clone()
        };

        // Word-wrap the description
        let desc_lines = if !item_def.description.is_empty() {
            self.wrap_text(&item_def.description, text_width_limit, small_font_size)
        } else {
            vec![]
        };

        // Calculate tooltip width based on longest line
        let mut max_w = self.measure_text_sharp(&name_text, font_size).width;
        for line in &desc_lines {
            max_w = max_w.max(self.measure_text_sharp(line, small_font_size).width);
        }

        if let Some(ref equip) = item_def.equipment {
            if equip.attack_bonus != 0 {
                let attack_text = if equip.attack_bonus > 0 {
                    format!("+{} Attack", equip.attack_bonus)
                } else {
                    format!("{} Attack", equip.attack_bonus)
                };
                max_w = max_w.max(self.measure_text_sharp(&attack_text, small_font_size).width);
            }
            if equip.strength_bonus != 0 {
                let strength_text = if equip.strength_bonus > 0 {
                    format!("+{} Strength", equip.strength_bonus)
                } else {
                    format!("{} Strength", equip.strength_bonus)
                };
                max_w = max_w.max(self.measure_text_sharp(&strength_text, small_font_size).width);
            }
            if equip.defence_bonus != 0 {
                let defence_text = if equip.defence_bonus > 0 {
                    format!("+{} Defence", equip.defence_bonus)
                } else {
                    format!("{} Defence", equip.defence_bonus)
                };
                max_w = max_w.max(self.measure_text_sharp(&defence_text, small_font_size).width);
            }
            // Measure requirement text
            let is_weapon = equip.slot_type == "weapon";
            let req_text = if is_weapon && equip.attack_level_required > 1 {
                format!("Requires {} Attack", equip.attack_level_required)
            } else if !is_weapon && equip.defence_level_required > 1 {
                format!("Requires {} Defence", equip.defence_level_required)
            } else {
                String::new()
            };
            if !req_text.is_empty() {
                max_w = max_w.max(self.measure_text_sharp(&req_text, small_font_size).width);
            }
        }

        let tooltip_width = (max_w + padding * 2.0).ceil().min(max_tooltip_width);

        // Calculate tooltip height based on actual lines drawn
        let mut total_h = padding * 2.0;
        total_h += line_height; // Name
        total_h += line_height; // Category badge

        let has_description = !desc_lines.is_empty();
        let has_equipment = item_def.equipment.is_some();

        if has_description {
            total_h += 2.0; // Small gap
            total_h += desc_lines.len() as f32 * line_height;
        }
        if has_equipment {
            total_h += 2.0; // Small gap
            if let Some(ref equip) = item_def.equipment {
                if equip.attack_bonus != 0 {
                    total_h += line_height;
                }
                if equip.strength_bonus != 0 {
                    total_h += line_height;
                }
                if equip.defence_bonus != 0 {
                    total_h += line_height;
                }
                // Level requirement line (only if > 1)
                let is_weapon = equip.slot_type == "weapon";
                if (is_weapon && equip.attack_level_required > 1) || (!is_weapon && equip.defence_level_required > 1) {
                    total_h += line_height;
                }
            }
        }

        let tooltip_height = total_h.ceil();

        // Position tooltip near cursor, but keep on screen
        let mut tooltip_x = (mouse_x + 16.0).floor();
        let mut tooltip_y = (mouse_y + 16.0).floor();

        // Clamp to screen bounds
        if tooltip_x + tooltip_width > screen_width() {
            tooltip_x = (mouse_x - tooltip_width - 8.0).floor();
        }
        if tooltip_y + tooltip_height > screen_height() {
            tooltip_y = (mouse_y - tooltip_height - 8.0).floor();
        }

        // Draw tooltip frame (3-layer)
        // Shadow
        draw_rectangle(tooltip_x + 2.0, tooltip_y + 2.0, tooltip_width, tooltip_height,
                       Color::new(0.0, 0.0, 0.0, 0.4));
        // Frame
        draw_rectangle(tooltip_x - 1.0, tooltip_y - 1.0, tooltip_width + 2.0, tooltip_height + 2.0,
                       TOOLTIP_FRAME);
        // Background
        draw_rectangle(tooltip_x, tooltip_y, tooltip_width, tooltip_height, TOOLTIP_BG);

        // Inner highlight (top edge)
        draw_line(tooltip_x + 1.0, tooltip_y + 1.0, tooltip_x + tooltip_width - 1.0, tooltip_y + 1.0,
                  1.0, Color::new(0.227, 0.227, 0.267, 1.0));

        let mut y = tooltip_y + padding + 12.0;

        // Item name (white, bold-ish)
        self.draw_text_sharp(&name_text, tooltip_x + padding, y, font_size, TEXT_NORMAL);
        y += line_height;

        // Category badge
        let category_color = self.get_category_color(&item_def.category);
        let category_text = item_def.category.to_uppercase();
        let badge_w = self.measure_text_sharp(&category_text, 16.0).width + 10.0;
        let badge_h = 20.0;
        let badge_x = tooltip_x + padding;
        let badge_y = y - 14.0;

        // Badge background (tinted)
        let badge_bg = Color::new(category_color.r, category_color.g, category_color.b, 0.2);
        draw_rectangle(badge_x, badge_y, badge_w, badge_h, badge_bg);
        draw_rectangle_lines(badge_x, badge_y, badge_w, badge_h, 1.0, category_color);
        self.draw_text_sharp(&category_text, badge_x + 5.0, y, 16.0, category_color);
        y += line_height;

        // Description section (if any)
        if has_description {
            y += 2.0;
            // Description text
            for line in &desc_lines {
                self.draw_text_sharp(line, tooltip_x + padding, y, small_font_size, TEXT_DIM);
                y += line_height;
            }
        }

        // Equipment stats section
        if let Some(ref equip) = item_def.equipment {
            y += 2.0;
            // Stat colors
            let stat_green = Color::new(0.392, 0.784, 0.392, 1.0);  // rgba(100, 200, 100)
            let stat_red = Color::new(1.0, 0.392, 0.392, 1.0);      // rgba(255, 100, 100)

            // Attack bonus
            if equip.attack_bonus != 0 {
                let attack_text = if equip.attack_bonus > 0 {
                    format!("+{} Attack", equip.attack_bonus)
                } else {
                    format!("{} Attack", equip.attack_bonus)
                };
                let attack_color = if equip.attack_bonus > 0 { stat_green } else { stat_red };
                self.draw_text_sharp(&attack_text, tooltip_x + padding, y, small_font_size, attack_color);
                y += line_height;
            }

            // Strength bonus
            if equip.strength_bonus != 0 {
                let strength_text = if equip.strength_bonus > 0 {
                    format!("+{} Strength", equip.strength_bonus)
                } else {
                    format!("{} Strength", equip.strength_bonus)
                };
                let strength_color = if equip.strength_bonus > 0 { stat_green } else { stat_red };
                self.draw_text_sharp(&strength_text, tooltip_x + padding, y, small_font_size, strength_color);
                y += line_height;
            }

            // Defence bonus
            if equip.defence_bonus != 0 {
                let defence_text = if equip.defence_bonus > 0 {
                    format!("+{} Defence", equip.defence_bonus)
                } else {
                    format!("{} Defence", equip.defence_bonus)
                };
                let defence_color = if equip.defence_bonus > 0 { stat_green } else { stat_red };
                self.draw_text_sharp(&defence_text, tooltip_x + padding, y, small_font_size, defence_color);
                y += line_height;
            }

            // Level requirements - check highest requirement against combat level
            let level_required = equip.attack_level_required.max(equip.defence_level_required);
            if level_required > 1 {
                let meets_req = player_combat_level >= level_required;
                let req_color = if meets_req { stat_green } else { stat_red };
                let req_text = format!("Requires {} Combat", level_required);
                self.draw_text_sharp(&req_text, tooltip_x + padding, y, small_font_size, req_color);
            }
        }
    }

    /// Get enhanced category color for tooltips
    pub(crate) fn get_category_color(&self, category: &str) -> Color {
        match category.to_lowercase().as_str() {
            "equipment" => CATEGORY_EQUIPMENT,
            "consumable" => CATEGORY_CONSUMABLE,
            "material" => CATEGORY_MATERIAL,
            "quest" => CATEGORY_QUEST,
            _ => TEXT_NORMAL,
        }
    }
}
