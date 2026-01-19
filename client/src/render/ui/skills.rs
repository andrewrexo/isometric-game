//! Skills panel rendering - compact 3x3 grid showing combat skill levels
//! 2 active skills (Hitpoints, Combat), 6 locked placeholder slots

use macroquad::prelude::*;
use crate::game::{GameState, SkillType};
use crate::ui::{UiElementId, UiLayout};
use super::super::Renderer;
use super::common::*;

/// Skills panel dimensions
const SKILLS_PANEL_WIDTH: f32 = 200.0;
const SKILLS_PANEL_HEIGHT: f32 = 175.0;

/// Compact header for skills panel
const SKILLS_HEADER_HEIGHT: f32 = 28.0;

/// Skill slot dimensions
const SKILL_SLOT_SIZE: f32 = 40.0;
const SKILL_SLOT_SPACING: f32 = 4.0;
const SKILL_GRID_COLS: usize = 3;
const SKILL_GRID_ROWS: usize = 3;
const TOTAL_SKILL_SLOTS: usize = 8;

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

        let screen_w = screen_width();
        let screen_h = screen_height();

        // Position panel on right side, above bottom bar
        let exp_bar_top = screen_h - EXP_BAR_HEIGHT;
        let panel_x = screen_w - SKILLS_PANEL_WIDTH - 16.0;
        let panel_y = exp_bar_top - EXP_BAR_GAP - SKILLS_PANEL_HEIGHT - MENU_BUTTON_SIZE - 8.0;

        // Draw panel frame
        self.draw_panel_frame(panel_x, panel_y, SKILLS_PANEL_WIDTH, SKILLS_PANEL_HEIGHT);
        self.draw_corner_accents(panel_x, panel_y, SKILLS_PANEL_WIDTH, SKILLS_PANEL_HEIGHT);

        // Header (compact)
        let header_x = panel_x + FRAME_THICKNESS;
        let header_y = panel_y + FRAME_THICKNESS;
        let header_w = SKILLS_PANEL_WIDTH - FRAME_THICKNESS * 2.0;

        draw_rectangle(header_x, header_y, header_w, SKILLS_HEADER_HEIGHT, HEADER_BG);
        draw_line(
            header_x + 10.0,
            header_y + SKILLS_HEADER_HEIGHT,
            header_x + header_w - 10.0,
            header_y + SKILLS_HEADER_HEIGHT,
            2.0,
            HEADER_BORDER,
        );

        // Header text with combat level
        let combat_level = state.get_local_player()
            .map(|p| p.combat_level())
            .unwrap_or(3);
        let header_text = format!("Skills (Cmb: {})", combat_level);
        let text_dims = self.measure_text_sharp(&header_text, 16.0);
        let text_x = header_x + (header_w - text_dims.width) / 2.0;
        self.draw_text_sharp(&header_text, text_x, header_y + 20.0, 16.0, TEXT_TITLE);

        // Grid area
        let grid_x = panel_x + FRAME_THICKNESS + GRID_PADDING;
        let grid_y = header_y + SKILLS_HEADER_HEIGHT + 8.0;

        // Draw skill slots (8 total in 3x3 grid, last slot empty)
        for slot_index in 0..TOTAL_SKILL_SLOTS {
            let row = slot_index / SKILL_GRID_COLS;
            let col = slot_index % SKILL_GRID_COLS;
            let slot_x = grid_x + col as f32 * (SKILL_SLOT_SIZE + SKILL_SLOT_SPACING);
            let slot_y = grid_y + row as f32 * (SKILL_SLOT_SIZE + SKILL_SLOT_SPACING);

            // Register bounds for hit detection
            let bounds = Rect::new(slot_x, slot_y, SKILL_SLOT_SIZE, SKILL_SLOT_SIZE);
            layout.add(UiElementId::SkillSlot(slot_index), bounds);

            let is_hovered = matches!(hovered, Some(UiElementId::SkillSlot(i)) if *i == slot_index);

            if slot_index < ACTIVE_SKILLS.len() {
                // Active skill slot
                let skill_type = ACTIVE_SKILLS[slot_index];
                let skill = state.get_local_player()
                    .map(|p| p.skills.get(skill_type).clone())
                    .unwrap_or_default();

                self.draw_skill_slot(slot_x, slot_y, skill_type, skill.level, is_hovered);
            } else {
                // Locked placeholder slot
                self.draw_locked_skill_slot(slot_x, slot_y);
            }
        }
    }

    /// Draw an active skill slot with icon and level
    fn draw_skill_slot(&self, x: f32, y: f32, skill_type: SkillType, level: i32, is_hovered: bool) {
        let size = SKILL_SLOT_SIZE;

        // Background
        let bg_color = if is_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
        let border_color = if is_hovered { SLOT_HOVER_BORDER } else { SLOT_BORDER };

        draw_rectangle(x, y, size, size, border_color);
        draw_rectangle(x + 1.0, y + 1.0, size - 2.0, size - 2.0, bg_color);

        // Inner shadow
        draw_line(x + 2.0, y + 2.0, x + size - 2.0, y + 2.0, 2.0, SLOT_INNER_SHADOW);
        draw_line(x + 2.0, y + 2.0, x + 2.0, y + size - 2.0, 2.0, SLOT_INNER_SHADOW);

        // Draw skill letter (placeholder until icons are added)
        let letter = match skill_type {
            SkillType::Hitpoints => "H",
            SkillType::Combat => "C",
        };
        let icon_color = self.get_skill_icon_color(skill_type);
        let letter_dims = self.measure_text_sharp(letter, 20.0);
        let letter_x = x + (size - letter_dims.width) / 2.0;
        let letter_y = y + size / 2.0 + 4.0; // Center vertically, slightly up for level
        self.draw_text_sharp(letter, letter_x, letter_y, 20.0, icon_color);

        // Level number in bottom-right corner
        let level_text = format!("{}", level);
        let text_dims = self.measure_text_sharp(&level_text, 16.0);
        let level_x = x + size - text_dims.width - 4.0;
        let level_y = y + size - 4.0;

        // Text shadow
        self.draw_text_sharp(&level_text, level_x + 1.0, level_y + 1.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.8));
        // Text
        self.draw_text_sharp(&level_text, level_x, level_y, 16.0, TEXT_NORMAL);
    }

    /// Draw a locked placeholder slot
    fn draw_locked_skill_slot(&self, x: f32, y: f32) {
        let size = SKILL_SLOT_SIZE;

        // Darker background for locked slots
        let bg_color = Color::new(0.055, 0.055, 0.075, 1.0);
        let border_color = Color::new(0.15, 0.14, 0.13, 1.0);

        draw_rectangle(x, y, size, size, border_color);
        draw_rectangle(x + 1.0, y + 1.0, size - 2.0, size - 2.0, bg_color);

        // Draw lock icon (simple padlock shape)
        let center_x = x + size / 2.0;
        let center_y = y + size / 2.0;
        let lock_color = Color::new(0.3, 0.28, 0.25, 1.0);

        // Lock body (rectangle)
        draw_rectangle(center_x - 6.0, center_y - 2.0, 12.0, 10.0, lock_color);

        // Lock shackle (arch)
        draw_rectangle(center_x - 4.0, center_y - 8.0, 2.0, 8.0, lock_color);
        draw_rectangle(center_x + 2.0, center_y - 8.0, 2.0, 8.0, lock_color);
        draw_rectangle(center_x - 4.0, center_y - 10.0, 8.0, 3.0, lock_color);
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
        let tooltip_x = (mouse_x + 16.0).min(screen_width() - tooltip_width - 8.0);
        let tooltip_y = (mouse_y + 16.0).min(screen_height() - tooltip_height - 8.0);

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
