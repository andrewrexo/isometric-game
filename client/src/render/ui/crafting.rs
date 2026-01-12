//! Crafting panel rendering

use macroquad::prelude::*;
use crate::game::{GameState, RecipeDefinition};
use crate::ui::{UiElementId, UiLayout};
use super::super::Renderer;
use super::common::*;

impl Renderer {
    pub(crate) fn render_crafting(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let panel_width = 650.0;
        let panel_height = 450.0;
        let panel_x = (screen_width() - panel_width) / 2.0;
        let panel_y = (screen_height() - panel_height) / 2.0;

        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.588));

        // Draw themed panel frame with corner accents
        self.draw_panel_frame(panel_x, panel_y, panel_width, panel_height);
        self.draw_corner_accents(panel_x, panel_y, panel_width, panel_height);

        // ===== HEADER SECTION =====
        let header_x = panel_x + FRAME_THICKNESS;
        let header_y = panel_y + FRAME_THICKNESS;
        let header_w = panel_width - FRAME_THICKNESS * 2.0;

        draw_rectangle(header_x, header_y, header_w, HEADER_HEIGHT, HEADER_BG);

        draw_line(header_x + 10.0, header_y + HEADER_HEIGHT, header_x + header_w - 10.0, header_y + HEADER_HEIGHT, 2.0, HEADER_BORDER);

        let dot_spacing = 60.0;
        let num_dots = ((header_w - 40.0) / dot_spacing) as i32;
        let start_dot_x = header_x + 20.0;
        for i in 0..num_dots {
            let dot_x = start_dot_x + i as f32 * dot_spacing;
            draw_rectangle(dot_x - 1.5, header_y + HEADER_HEIGHT - 1.5, 3.0, 3.0, FRAME_ACCENT);
        }

        self.draw_text_sharp("CRAFTING", header_x + 12.0, header_y + 26.0, 16.0, TEXT_TITLE);
        self.draw_text_sharp("[E] Close", header_x + header_w - 80.0, header_y + 26.0, 16.0, TEXT_DIM);

        // ===== CONTENT AREA =====
        let content_y = panel_y + FRAME_THICKNESS + HEADER_HEIGHT + 8.0;
        let content_height = panel_height - FRAME_THICKNESS * 2.0 - HEADER_HEIGHT - FOOTER_HEIGHT - 16.0;

        let categories: Vec<&str> = {
            let mut cats: Vec<&str> = state.recipe_definitions.iter()
                .map(|r| r.category.as_str())
                .collect();
            cats.sort();
            cats.dedup();
            cats
        };

        if categories.is_empty() {
            self.draw_text_sharp("No recipes available", panel_x + 20.0, content_y + 40.0, 16.0, TEXT_DIM);
            return;
        }

        // ===== CATEGORY TABS =====
        let tab_y = content_y;
        let tab_height = 28.0;
        let mut tab_x = panel_x + FRAME_THICKNESS + 10.0;

        for (i, category) in categories.iter().enumerate() {
            let is_selected = i == state.ui_state.crafting_selected_category;
            let tab_width = self.measure_text_sharp(category, 16.0).width + 24.0;

            let bounds = Rect::new(tab_x, tab_y, tab_width, tab_height);
            layout.add(UiElementId::CraftingCategoryTab(i), bounds);

            let is_hovered = matches!(hovered, Some(UiElementId::CraftingCategoryTab(idx)) if *idx == i);

            let (bg_color, border_color) = if is_selected {
                (SLOT_HOVER_BG, SLOT_SELECTED_BORDER)
            } else if is_hovered {
                (Color::new(0.141, 0.141, 0.188, 1.0), SLOT_HOVER_BORDER)
            } else {
                (SLOT_BG_EMPTY, SLOT_BORDER)
            };

            draw_rectangle(tab_x, tab_y, tab_width, tab_height, border_color);
            draw_rectangle(tab_x + 1.0, tab_y + 1.0, tab_width - 2.0, tab_height - 2.0, bg_color);

            if is_selected {
                draw_line(tab_x + 2.0, tab_y + 2.0, tab_x + tab_width - 2.0, tab_y + 2.0, 1.0, FRAME_INNER);
                draw_line(tab_x + 2.0, tab_y + 2.0, tab_x + 2.0, tab_y + tab_height - 2.0, 1.0, FRAME_INNER);
            }

            let display_name: String = category.chars().enumerate()
                .map(|(idx, c)| if idx == 0 { c.to_ascii_uppercase() } else { c })
                .collect();

            let text_color = if is_selected { TEXT_TITLE } else if is_hovered { TEXT_NORMAL } else { TEXT_DIM };
            self.draw_text_sharp(&display_name, tab_x + 12.0, tab_y + 19.0, 16.0, text_color);

            tab_x += tab_width + 4.0;
        }

        let current_category = categories.get(state.ui_state.crafting_selected_category).copied().unwrap_or("consumables");
        let recipes: Vec<&RecipeDefinition> = state.recipe_definitions.iter()
            .filter(|r| r.category == current_category)
            .collect();

        // ===== RECIPE LIST (left side) =====
        let list_width = 220.0;
        let list_x = panel_x + FRAME_THICKNESS + 10.0;
        let list_y = tab_y + tab_height + 10.0;
        let list_height = content_height - tab_height - 18.0;

        draw_rectangle(list_x, list_y, list_width, list_height, SLOT_BORDER);
        draw_rectangle(list_x + 1.0, list_y + 1.0, list_width - 2.0, list_height - 2.0, SLOT_BG_EMPTY);

        draw_line(list_x + 2.0, list_y + 2.0, list_x + list_width - 2.0, list_y + 2.0, 2.0, SLOT_INNER_SHADOW);
        draw_line(list_x + 2.0, list_y + 2.0, list_x + 2.0, list_y + list_height - 2.0, 2.0, SLOT_INNER_SHADOW);

        self.draw_text_sharp("Recipes", list_x + 8.0, list_y + 18.0, 16.0, TEXT_TITLE);
        draw_line(list_x + 6.0, list_y + 24.0, list_x + list_width - 6.0, list_y + 24.0, 1.0, HEADER_BORDER);

        let line_height = 28.0;
        let mut y = list_y + 32.0;

        for (i, recipe) in recipes.iter().enumerate() {
            if y > list_y + list_height - line_height {
                break;
            }

            let is_selected = i == state.ui_state.crafting_selected_recipe;

            let item_bounds = Rect::new(list_x + 4.0, y, list_width - 8.0, line_height - 2.0);
            layout.add(UiElementId::CraftingRecipeItem(i), item_bounds);

            let is_hovered = matches!(hovered, Some(UiElementId::CraftingRecipeItem(idx)) if *idx == i);

            if is_selected {
                draw_rectangle(list_x + 4.0, y, list_width - 8.0, line_height - 2.0, SLOT_HOVER_BG);
            } else if is_hovered {
                draw_rectangle(list_x + 4.0, y, list_width - 8.0, line_height - 2.0, Color::new(0.125, 0.125, 0.173, 1.0));
            }

            let text_color = if is_selected { TEXT_TITLE } else if is_hovered { TEXT_NORMAL } else { TEXT_DIM };

            let prefix = if is_selected { "> " } else { "  " };
            self.draw_text_sharp(&format!("{}{}", prefix, recipe.display_name), list_x + 8.0, y + 19.0, 16.0, text_color);

            if recipe.level_required > 1 {
                let level_text = format!("Lv{}", recipe.level_required);
                let level_width = self.measure_text_sharp(&level_text, 16.0).width;
                self.draw_text_sharp(&level_text, list_x + list_width - level_width - 12.0, y + 17.0, 16.0, FRAME_MID);
            }

            y += line_height;
        }

        // ===== DETAIL PANEL (right side) =====
        let detail_x = list_x + list_width + 12.0;
        let detail_width = panel_width - list_width - FRAME_THICKNESS * 2.0 - 32.0;
        let detail_y = list_y;
        let detail_height = list_height;

        draw_rectangle(detail_x, detail_y, detail_width, detail_height, SLOT_BORDER);
        draw_rectangle(detail_x + 1.0, detail_y + 1.0, detail_width - 2.0, detail_height - 2.0, Color::new(0.094, 0.094, 0.125, 1.0));

        draw_line(detail_x + 2.0, detail_y + 2.0, detail_x + detail_width - 2.0, detail_y + 2.0, 2.0, SLOT_INNER_SHADOW);
        draw_line(detail_x + 2.0, detail_y + 2.0, detail_x + 2.0, detail_y + detail_height - 2.0, 2.0, SLOT_INNER_SHADOW);

        if let Some(recipe) = recipes.get(state.ui_state.crafting_selected_recipe) {
            self.draw_text_sharp(&recipe.display_name, detail_x + 12.0, detail_y + 24.0, 16.0, TEXT_TITLE);

            draw_line(detail_x + 10.0, detail_y + 32.0, detail_x + detail_width - 10.0, detail_y + 32.0, 1.0, HEADER_BORDER);

            let desc_height = self.draw_text_wrapped(
                &recipe.description,
                detail_x + 12.0,
                detail_y + 48.0,
                16.0,
                TEXT_NORMAL,
                detail_width - 24.0,
                20.0,
            );

            let mut section_y = detail_y + 48.0 + desc_height + 10.0;

            if recipe.level_required > 1 {
                let (level_color, level_icon) = if let Some(player) = state.get_local_player() {
                    if player.level >= recipe.level_required {
                        (Color::new(0.392, 0.784, 0.392, 1.0), "[OK]")
                    } else {
                        (Color::new(0.784, 0.314, 0.314, 1.0), "[!!]")
                    }
                } else {
                    (TEXT_DIM, "[??]")
                };
                self.draw_text_sharp(&format!("{} Requires Level {}", level_icon, recipe.level_required), detail_x + 12.0, section_y, 16.0, level_color);
                section_y += 25.0;
            }

            self.draw_text_sharp("Materials Required:", detail_x + 12.0, section_y, 16.0, FRAME_INNER);
            section_y += 22.0;

            let mut can_craft = true;

            for ingredient in &recipe.ingredients {
                let have_count = state.inventory.count_item_by_id(&ingredient.item_id);
                let need_count = ingredient.count;
                let has_enough = have_count >= need_count;

                if !has_enough {
                    can_craft = false;
                }

                let (marker, color) = if has_enough {
                    ("[+]", Color::new(0.392, 0.784, 0.392, 1.0))
                } else {
                    ("[-]", Color::new(0.784, 0.314, 0.314, 1.0))
                };

                let display_name = state.item_registry.get_display_name(&ingredient.item_id);
                let text = format!("{} {} ({}/{})", marker, display_name, have_count, need_count);
                self.draw_text_sharp(&text, detail_x + 20.0, section_y, 16.0, color);
                section_y += 20.0;
            }

            section_y += 12.0;
            self.draw_text_sharp("Creates:", detail_x + 12.0, section_y, 16.0, FRAME_INNER);
            section_y += 22.0;

            for result in &recipe.results {
                let display_name = state.item_registry.get_display_name(&result.item_id);
                let text = format!("  {} x{}", display_name, result.count);
                self.draw_text_sharp(&text, detail_x + 20.0, section_y, 16.0, CATEGORY_EQUIPMENT);
                section_y += 20.0;
            }

            // ===== CRAFT BUTTON =====
            let btn_y = detail_y + detail_height - 38.0;
            let btn_width = 140.0;
            let btn_x = detail_x + (detail_width - btn_width) / 2.0;

            if can_craft {
                let bounds = Rect::new(btn_x, btn_y, btn_width, 28.0);
                layout.add(UiElementId::CraftingButton, bounds);
            }

            let is_btn_hovered = can_craft && matches!(hovered, Some(UiElementId::CraftingButton));

            if can_craft {
                let (btn_bg, btn_border) = if is_btn_hovered {
                    (Color::new(0.282, 0.235, 0.157, 1.0), FRAME_ACCENT)
                } else {
                    (Color::new(0.188, 0.157, 0.110, 1.0), FRAME_MID)
                };

                draw_rectangle(btn_x, btn_y, btn_width, 28.0, btn_border);
                draw_rectangle(btn_x + 1.0, btn_y + 1.0, btn_width - 2.0, 26.0, btn_bg);

                draw_line(btn_x + 2.0, btn_y + 2.0, btn_x + btn_width - 2.0, btn_y + 2.0, 1.0, FRAME_INNER);
                draw_line(btn_x + 2.0, btn_y + 2.0, btn_x + 2.0, btn_y + 26.0, 1.0, FRAME_INNER);

                let craft_text = "[ CRAFT ]";
                let text_w = self.measure_text_sharp(craft_text, 16.0).width;
                self.draw_text_sharp(craft_text, btn_x + (btn_width - text_w) / 2.0, btn_y + 19.0, 16.0, TEXT_TITLE);
            } else {
                draw_rectangle(btn_x, btn_y, btn_width, 28.0, SLOT_BORDER);
                draw_rectangle(btn_x + 1.0, btn_y + 1.0, btn_width - 2.0, 26.0, Color::new(0.125, 0.094, 0.094, 1.0));

                let text = "Missing Materials";
                let text_w = self.measure_text_sharp(text, 16.0).width;
                self.draw_text_sharp(text, btn_x + (btn_width - text_w) / 2.0, btn_y + 19.0, 16.0, Color::new(0.502, 0.314, 0.314, 1.0));
            }
        } else {
            self.draw_text_sharp("Select a recipe", detail_x + 12.0, detail_y + 24.0, 16.0, TEXT_DIM);
        }

        // ===== FOOTER SECTION =====
        let footer_x = panel_x + FRAME_THICKNESS;
        let footer_y = panel_y + panel_height - FRAME_THICKNESS - FOOTER_HEIGHT;
        let footer_w = panel_width - FRAME_THICKNESS * 2.0;

        draw_rectangle(footer_x, footer_y, footer_w, FOOTER_HEIGHT, FOOTER_BG);
        draw_line(footer_x + 10.0, footer_y, footer_x + footer_w - 10.0, footer_y, 1.0, HEADER_BORDER);

        self.draw_text_sharp("[A/D] Category", footer_x + 10.0, footer_y + 20.0, 16.0, TEXT_DIM);
        self.draw_text_sharp("[W/S] Select", footer_x + 130.0, footer_y + 20.0, 16.0, TEXT_DIM);
        self.draw_text_sharp("[C] Craft", footer_x + 250.0, footer_y + 20.0, 16.0, TEXT_DIM);
        self.draw_text_sharp("[E] Close", footer_x + footer_w - 80.0, footer_y + 20.0, 16.0, TEXT_DIM);
    }
}
