//! Skills panel rendering - compact 3x3 grid showing combat skill levels
//! 2 active skills (Hitpoints, Combat), 6 locked placeholder slots

use macroquad::prelude::*;
use crate::game::{GameState, SkillType};
use crate::ui::{UiElementId, UiLayout};
use crate::util::virtual_screen_size;
use super::super::Renderer;
use super::common::*;

/// Skills panel dimensions (compact: just fits the 3x3 grid with padding)
const SKILLS_PANEL_PADDING: f32 = 8.0;
const SKILLS_GRID_WIDTH: f32 = 3.0 * SKILL_SLOT_SIZE + 2.0 * SKILL_SLOT_SPACING; // 128
const SKILLS_PANEL_WIDTH: f32 = SKILLS_GRID_WIDTH + SKILLS_PANEL_PADDING * 2.0 + FRAME_THICKNESS * 2.0; // 152
const SKILLS_HEADER_HEIGHT: f32 = 24.0;
const SKILLS_PANEL_HEIGHT: f32 = FRAME_THICKNESS * 2.0 + SKILLS_HEADER_HEIGHT + SKILLS_PANEL_PADDING + SKILLS_GRID_WIDTH + SKILLS_PANEL_PADDING; // 176

/// Skill slot dimensions
const SKILL_SLOT_SIZE: f32 = 40.0;
const SKILL_SLOT_SPACING: f32 = 4.0;
const SKILL_GRID_COLS: usize = 3;
const SKILL_GRID_ROWS: usize = 3;
const TOTAL_SKILL_SLOTS: usize = 8;

/// UI icons sprite sheet: 24x24 icons in 10 columns
const UI_ICON_SIZE: f32 = 24.0;
const UI_ICON_COLS: usize = 10;

/// Active skills in display order (consolidated combat system)
const ACTIVE_SKILLS: [SkillType; 2] = [
    SkillType::Hitpoints,
    SkillType::Combat,
];

impl Renderer {
    /// Render the skills panel when open
    pub(crate) fn render_skills_panel(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        if !state.ui_state.skills_open {
            return;
        }

        let (screen_w, screen_h) = virtual_screen_size();
        let scale = state.ui_state.ui_scale;

        // Scaled dimensions
        let panel_width = SKILLS_PANEL_WIDTH * scale;
        let panel_height = SKILLS_PANEL_HEIGHT * scale;
        let frame_thickness = FRAME_THICKNESS * scale;
        let header_height = SKILLS_HEADER_HEIGHT * scale;
        let panel_padding = SKILLS_PANEL_PADDING * scale;
        let slot_size = SKILL_SLOT_SIZE * scale;
        let slot_spacing = SKILL_SLOT_SPACING * scale;
        let button_size = MENU_BUTTON_SIZE * scale;
        let exp_bar_gap = EXP_BAR_GAP * scale;

        // Position panel on right side, above the menu buttons (align with button right edge)
        let panel_x = screen_w - panel_width - 8.0;
        let button_area_height = button_size + exp_bar_gap;
        let panel_y = screen_h - button_area_height - panel_height - 8.0;

        // Draw panel frame
        self.draw_panel_frame(panel_x, panel_y, panel_width, panel_height);
        self.draw_corner_accents(panel_x, panel_y, panel_width, panel_height);

        // Header (compact)
        let header_x = panel_x + frame_thickness;
        let header_y = panel_y + frame_thickness;
        let header_w = panel_width - frame_thickness * 2.0;

        draw_rectangle(header_x, header_y, header_w, header_height, HEADER_BG);
        draw_line(
            header_x + 6.0 * scale,
            header_y + header_height,
            header_x + header_w - 6.0 * scale,
            header_y + header_height,
            1.0,
            HEADER_BORDER,
        );

        // Header text with combat level (native font size for crisp rendering)
        let combat_level = state.get_local_player()
            .map(|p| p.combat_level())
            .unwrap_or(3);
        let header_text = format!("Skills (Cmb: {})", combat_level);
        let text_dims = self.measure_text_sharp(&header_text, 16.0);
        let text_x = header_x + (header_w - text_dims.width) / 2.0;
        self.draw_text_sharp(&header_text, text_x, header_y + (header_height + 12.0) / 2.0, 16.0, TEXT_TITLE);

        // Grid area
        let grid_x = panel_x + frame_thickness + panel_padding;
        let grid_y = header_y + header_height + panel_padding;

        // Draw skill slots (8 total in 3x3 grid, last slot empty)
        for slot_index in 0..TOTAL_SKILL_SLOTS {
            let row = slot_index / SKILL_GRID_COLS;
            let col = slot_index % SKILL_GRID_COLS;
            let slot_x = grid_x + col as f32 * (slot_size + slot_spacing);
            let slot_y = grid_y + row as f32 * (slot_size + slot_spacing);

            // Register bounds for hit detection
            let bounds = Rect::new(slot_x, slot_y, slot_size, slot_size);
            layout.add(UiElementId::SkillSlot(slot_index), bounds);

            let is_hovered = matches!(hovered, Some(UiElementId::SkillSlot(i)) if *i == slot_index);

            if slot_index < ACTIVE_SKILLS.len() {
                // Active skill slot
                let skill_type = ACTIVE_SKILLS[slot_index];
                let skill = state.get_local_player()
                    .map(|p| p.skills.get(skill_type).clone())
                    .unwrap_or_default();

                self.draw_skill_slot_scaled(slot_x, slot_y, slot_size, skill_type, skill.level, is_hovered, scale);
            } else {
                // Locked placeholder slot
                self.draw_locked_skill_slot_scaled(slot_x, slot_y, slot_size, scale);
            }
        }
    }

    /// Draw an active skill slot with icon and level (scaled)
    fn draw_skill_slot_scaled(&self, x: f32, y: f32, size: f32, skill_type: SkillType, level: i32, is_hovered: bool, scale: f32) {
        // Background
        let bg_color = if is_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
        let border_color = if is_hovered { SLOT_HOVER_BORDER } else { SLOT_BORDER };

        draw_rectangle(x, y, size, size, border_color);
        draw_rectangle(x + 1.0, y + 1.0, size - 2.0, size - 2.0, bg_color);

        // Inner shadow
        draw_line(x + 2.0, y + 2.0, x + size - 2.0, y + 2.0, 2.0, SLOT_INNER_SHADOW);
        draw_line(x + 2.0, y + 2.0, x + 2.0, y + size - 2.0, 2.0, SLOT_INNER_SHADOW);

        // Draw skill icon from ui_icons.png spritesheet (scaled)
        let (icon_col, icon_row) = match skill_type {
            SkillType::Hitpoints => (0, 6),
            SkillType::Combat => (2, 6),
        };

        let icon_size = UI_ICON_SIZE * scale;
        if let Some(ref texture) = self.ui_icons {
            let src_x = icon_col as f32 * UI_ICON_SIZE;
            let src_y = icon_row as f32 * UI_ICON_SIZE;
            let src_rect = Rect::new(src_x, src_y, UI_ICON_SIZE, UI_ICON_SIZE);

            // Center the icon in the slot, slightly up to leave room for level
            let icon_x = x + (size - icon_size) / 2.0;
            let icon_y = y + (size - icon_size) / 2.0 - 2.0 * scale;

            draw_texture_ex(
                texture,
                icon_x,
                icon_y,
                WHITE,
                DrawTextureParams {
                    source: Some(src_rect),
                    dest_size: Some(Vec2::new(icon_size, icon_size)),
                    ..Default::default()
                },
            );
        } else {
            // Fallback to letter if texture not loaded (native font size)
            let letter = match skill_type {
                SkillType::Hitpoints => "H",
                SkillType::Combat => "C",
            };
            let icon_color = self.get_skill_icon_color(skill_type);
            let letter_dims = self.measure_text_sharp(letter, 16.0);
            let letter_x = x + (size - letter_dims.width) / 2.0;
            let letter_y = y + size / 2.0 + 4.0;
            self.draw_text_sharp(letter, letter_x, letter_y, 16.0, icon_color);
        }

        // Level number in bottom-right corner (native font size)
        let level_text = format!("{}", level);
        let text_dims = self.measure_text_sharp(&level_text, 16.0);
        let level_x = x + size - text_dims.width - 4.0;
        let level_y = y + size - 4.0;

        // Text shadow
        self.draw_text_sharp(&level_text, level_x + 1.0, level_y + 1.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.8));
        // Text
        self.draw_text_sharp(&level_text, level_x, level_y, 16.0, TEXT_NORMAL);
    }

    /// Draw a locked placeholder slot (scaled)
    fn draw_locked_skill_slot_scaled(&self, x: f32, y: f32, size: f32, scale: f32) {
        // Darker background for locked slots
        let bg_color = Color::new(0.055, 0.055, 0.075, 1.0);
        let border_color = Color::new(0.15, 0.14, 0.13, 1.0);

        draw_rectangle(x, y, size, size, border_color);
        draw_rectangle(x + 1.0, y + 1.0, size - 2.0, size - 2.0, bg_color);

        // Draw lock icon (simple padlock shape, scaled)
        let center_x = x + size / 2.0;
        let center_y = y + size / 2.0;
        let lock_color = Color::new(0.3, 0.28, 0.25, 1.0);

        // Lock body (rectangle)
        draw_rectangle(center_x - 6.0 * scale, center_y - 2.0 * scale, 12.0 * scale, 10.0 * scale, lock_color);

        // Lock shackle (arch)
        draw_rectangle(center_x - 4.0 * scale, center_y - 8.0 * scale, 2.0 * scale, 8.0 * scale, lock_color);
        draw_rectangle(center_x + 2.0 * scale, center_y - 8.0 * scale, 2.0 * scale, 8.0 * scale, lock_color);
        draw_rectangle(center_x - 4.0 * scale, center_y - 10.0 * scale, 8.0 * scale, 3.0 * scale, lock_color);
    }

    /// Get the icon color for a skill type
    fn get_skill_icon_color(&self, skill_type: SkillType) -> Color {
        match skill_type {
            SkillType::Hitpoints => Color::new(0.8, 0.2, 0.2, 1.0),  // Red
            SkillType::Combat => Color::new(0.85, 0.65, 0.15, 1.0), // Gold/orange
        }
    }

    /// Render skill tooltip when hovering a skill slot
    pub(crate) fn render_skill_tooltip(&self, state: &GameState, hovered: &Option<UiElementId>) {
        let slot_index = match hovered {
            Some(UiElementId::SkillSlot(i)) if *i < ACTIVE_SKILLS.len() => *i,
            _ => return,
        };

        let skill_type = ACTIVE_SKILLS[slot_index];
        let player = match state.get_local_player() {
            Some(p) => p,
            None => return,
        };

        let skill = player.skills.get(skill_type);
        let (mouse_x, mouse_y) = mouse_position();

        // Tooltip content
        let name = skill_type.display_name();
        let level_text = format!("Level: {}", skill.level);
        let xp_text = format!("XP: {} / {}",
            Self::format_number(skill.xp),
            Self::format_number(crate::game::skills::total_xp_for_level(skill.level + 1))
        );
        let remaining_text = format!("To next: {} XP", Self::format_number(skill.xp_to_next_level()));

        // Calculate tooltip size
        let padding = 8.0;
        let line_height = 20.0;
        let font_size = 16.0;

        let name_dims = self.measure_text_sharp(name, font_size);
        let level_dims = self.measure_text_sharp(&level_text, font_size);
        let xp_dims = self.measure_text_sharp(&xp_text, font_size);
        let remaining_dims = self.measure_text_sharp(&remaining_text, font_size);

        let max_width = name_dims.width
            .max(level_dims.width)
            .max(xp_dims.width)
            .max(remaining_dims.width);

        let tooltip_width = max_width + padding * 2.0;
        let tooltip_height = padding * 2.0 + line_height * 4.0; // 4 lines of text

        // Position tooltip (offset from cursor, keep on screen)
        let (sw, sh) = virtual_screen_size();
        let tooltip_x = (mouse_x + 16.0).min(sw - tooltip_width - 8.0);
        let tooltip_y = (mouse_y + 16.0).min(sh - tooltip_height - 8.0);

        // Draw tooltip background
        draw_rectangle(tooltip_x - 1.0, tooltip_y - 1.0, tooltip_width + 2.0, tooltip_height + 2.0, TOOLTIP_FRAME);
        draw_rectangle(tooltip_x, tooltip_y, tooltip_width, tooltip_height, TOOLTIP_BG);

        // Draw text
        let mut text_y = tooltip_y + padding + 14.0;

        // Skill name (gold)
        self.draw_text_sharp(name, tooltip_x + padding, text_y, font_size, TEXT_GOLD);
        text_y += line_height;

        // Level
        self.draw_text_sharp(&level_text, tooltip_x + padding, text_y, font_size, TEXT_NORMAL);
        text_y += line_height;

        // XP
        self.draw_text_sharp(&xp_text, tooltip_x + padding, text_y, font_size, TEXT_NORMAL);
        text_y += line_height;

        // Remaining
        self.draw_text_sharp(&remaining_text, tooltip_x + padding, text_y, font_size, TEXT_DIM);
    }

    /// Format a number with commas (e.g., 1234567 -> "1,234,567")
    fn format_number(n: i64) -> String {
        let s = n.to_string();
        let mut result = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push(',');
            }
            result.push(c);
        }
        result.chars().rev().collect()
    }
}
