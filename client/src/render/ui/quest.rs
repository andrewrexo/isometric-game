//! Quest UI rendering (quest log, tracker, completion notifications)

use macroquad::prelude::*;
use crate::game::GameState;
use crate::ui::{UiElementId, UiLayout};
use crate::util::virtual_screen_size;
use super::super::Renderer;
use super::common::*;

impl Renderer {
    pub(crate) fn render_quest_log(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let (sw, sh) = virtual_screen_size();

        let panel_width = 380.0;
        let panel_height = 420.0;
        let panel_x = (sw - panel_width) / 2.0;
        let panel_y = (sh - panel_height) / 2.0;

        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.588));

        // Draw themed panel frame with corner accents
        self.draw_panel_frame(panel_x, panel_y, panel_width, panel_height);
        self.draw_corner_accents(panel_x, panel_y, panel_width, panel_height);

        // ===== HEADER SECTION =====
        let header_x = panel_x + FRAME_THICKNESS;
        let header_y = panel_y + FRAME_THICKNESS;
        let header_w = panel_width - FRAME_THICKNESS * 2.0;

        // Header background
        draw_rectangle(header_x, header_y, header_w, HEADER_HEIGHT, HEADER_BG);

        // Header bottom separator with decorative dots
        draw_line(header_x + 10.0, header_y + HEADER_HEIGHT, header_x + header_w - 10.0, header_y + HEADER_HEIGHT, 2.0, HEADER_BORDER);

        let dot_spacing = 50.0;
        let num_dots = ((header_w - 40.0) / dot_spacing) as i32;
        let start_dot_x = header_x + 20.0;
        for i in 0..num_dots {
            let dot_x = start_dot_x + i as f32 * dot_spacing;
            draw_rectangle(dot_x - 1.5, header_y + HEADER_HEIGHT - 1.5, 3.0, 3.0, FRAME_ACCENT);
        }

        // Title
        self.draw_text_sharp("QUEST LOG", header_x + 12.0, header_y + 26.0, 16.0, TEXT_TITLE);

        // Close hint (right side)
        self.draw_text_sharp("[Q] Close", header_x + header_w - 80.0, header_y + 26.0, 16.0, TEXT_DIM);

        // ===== CONTENT AREA =====
        let content_x = panel_x + FRAME_THICKNESS + 8.0;
        let content_y = panel_y + FRAME_THICKNESS + HEADER_HEIGHT + 8.0;
        let content_w = panel_width - FRAME_THICKNESS * 2.0 - 16.0;
        let content_h = panel_height - FRAME_THICKNESS * 2.0 - HEADER_HEIGHT - FOOTER_HEIGHT - 16.0;

        // Quest list panel with inset effect
        draw_rectangle(content_x, content_y, content_w, content_h, SLOT_BORDER);
        draw_rectangle(content_x + 1.0, content_y + 1.0, content_w - 2.0, content_h - 2.0, SLOT_BG_EMPTY);

        // Inner shadow (top/left)
        draw_line(content_x + 2.0, content_y + 2.0, content_x + content_w - 2.0, content_y + 2.0, 2.0, SLOT_INNER_SHADOW);
        draw_line(content_x + 2.0, content_y + 2.0, content_x + 2.0, content_y + content_h - 2.0, 2.0, SLOT_INNER_SHADOW);

        let mut y = content_y + 10.0;
        let line_height = 17.0;
        let objective_spacing = 16.0;

        if state.ui_state.active_quests.is_empty() {
            // Empty state with themed styling
            self.draw_text_sharp("No Active Quests", content_x + 12.0, y + 10.0, 16.0, TEXT_DIM);
            y += line_height + 8.0;
            self.draw_text_sharp("Talk to NPCs with", content_x + 12.0, y + 10.0, 16.0, Color::new(0.392, 0.392, 0.431, 1.0));
            self.draw_text_sharp("!", content_x + 140.0, y + 10.0, 16.0, TEXT_GOLD);
            self.draw_text_sharp("above their heads", content_x + 155.0, y + 10.0, 16.0, Color::new(0.392, 0.392, 0.431, 1.0));
        } else {
            for (quest_idx, quest) in state.ui_state.active_quests.iter().enumerate() {
                // Calculate entry height
                let entry_padding = 6.0;
                let title_height = line_height;
                let objectives_height = quest.objectives.len() as f32 * objective_spacing;
                let entry_height = entry_padding + title_height + 2.0 + objectives_height + entry_padding;

                let entry_start_y = y;

                // Check if we're about to overflow the panel
                if y + entry_height > content_y + content_h - 20.0 {
                    let remaining = state.ui_state.active_quests.len() - quest_idx;
                    if remaining > 0 {
                        self.draw_text_sharp(&format!("...and {} more quests", remaining), content_x + 12.0, y, 16.0, TEXT_DIM);
                    }
                    break;
                }

                // Register quest entry bounds for hover detection
                let bounds = Rect::new(content_x + 4.0, entry_start_y, content_w - 8.0, entry_height);
                layout.add(UiElementId::QuestLogEntry(quest_idx), bounds);

                // Check if this quest is hovered
                let is_hovered = matches!(hovered, Some(UiElementId::QuestLogEntry(idx)) if *idx == quest_idx);

                // Draw quest entry background with slot-like styling
                if is_hovered {
                    draw_rectangle(content_x + 4.0, entry_start_y, content_w - 8.0, entry_height, SLOT_HOVER_BORDER);
                    draw_rectangle(content_x + 5.0, entry_start_y + 1.0, content_w - 10.0, entry_height - 2.0, SLOT_HOVER_BG);
                }

                // Move y inside the entry box with padding
                y += entry_padding;

                // Quest name with star icon
                let name_color = if is_hovered { TEXT_TITLE } else { TEXT_NORMAL };
                self.draw_text_sharp("*", content_x + 12.0, y + 12.0, 16.0, TEXT_GOLD);
                self.draw_text_sharp(&quest.name, content_x + 28.0, y + 12.0, 16.0, name_color);
                y += title_height + 4.0;

                // Objectives with styled checkmarks
                for obj in &quest.objectives {
                    let (check_icon, status_color) = if obj.completed {
                        ("[+]", Color::new(0.392, 0.784, 0.392, 1.0))
                    } else {
                        ("[ ]", Color::new(0.502, 0.502, 0.541, 1.0))
                    };

                    self.draw_text_sharp(check_icon, content_x + 20.0, y + 12.0, 16.0, status_color);

                    let obj_text = format!("{} ({}/{})", obj.description, obj.current, obj.target);
                    let text_color = if obj.completed {
                        Color::new(0.392, 0.627, 0.392, 1.0)
                    } else {
                        TEXT_DIM
                    };
                    self.draw_text_sharp(&obj_text, content_x + 52.0, y + 12.0, 16.0, text_color);
                    y += objective_spacing;
                }

                // Move past bottom padding
                y += entry_padding;

                // Decorative separator between quests
                if quest_idx < state.ui_state.active_quests.len() - 1 {
                    draw_line(content_x + 20.0, y + 2.0, content_x + content_w - 20.0, y + 2.0, 1.0, SLOT_BORDER);
                    y += 8.0;
                }
            }
        }

        // ===== FOOTER SECTION =====
        let footer_x = panel_x + FRAME_THICKNESS;
        let footer_y = panel_y + panel_height - FRAME_THICKNESS - FOOTER_HEIGHT;
        let footer_w = panel_width - FRAME_THICKNESS * 2.0;

        draw_rectangle(footer_x, footer_y, footer_w, FOOTER_HEIGHT, FOOTER_BG);
        draw_line(footer_x + 10.0, footer_y, footer_x + footer_w - 10.0, footer_y, 1.0, HEADER_BORDER);

        self.draw_text_sharp("[Q] Close", footer_x + 10.0, footer_y + 20.0, 16.0, TEXT_DIM);

        let quest_count = state.ui_state.active_quests.len();
        let count_text = format!("{} Active", quest_count);
        let count_width = self.measure_text_sharp(&count_text, 16.0).width;
        self.draw_text_sharp(&count_text, footer_x + footer_w - count_width - 10.0, footer_y + 20.0, 16.0, FRAME_MID);
    }

    pub(crate) fn render_quest_tracker(&self, state: &GameState) {
        if state.ui_state.active_quests.is_empty() {
            return;
        }

        let tracker_x = 10.0;
        let tracker_y = if state.debug_mode { 460.0 } else { 20.0 };
        let line_height = 18.0;

        let mut y = tracker_y;

        // Header
        self.draw_text_sharp("QUESTS", tracker_x, y, 16.0, Color::from_rgba(255, 220, 100, 255));
        y += line_height + 5.0;

        // Only show first 2 active quests
        for quest in state.ui_state.active_quests.iter().take(2) {
            self.draw_text_sharp(&quest.name, tracker_x, y, 16.0, WHITE);
            y += line_height;

            for obj in &quest.objectives {
                let status_color = if obj.completed {
                    Color::from_rgba(100, 255, 100, 255)
                } else {
                    Color::from_rgba(200, 200, 200, 255)
                };

                let check = if obj.completed { "[x]" } else { "[ ]" };
                let obj_text = format!("{} {} ({}/{})", check, obj.description, obj.current, obj.target);
                self.draw_text_sharp(&obj_text, tracker_x + 10.0, y, 16.0, status_color);
                y += line_height - 2.0;
            }

            y += 8.0;
        }

        if state.ui_state.active_quests.len() > 2 {
            let more = format!("...and {} more (Q to view)", state.ui_state.active_quests.len() - 2);
            self.draw_text_sharp(&more, tracker_x, y, 16.0, GRAY);
        }
    }

    pub(crate) fn render_quest_completed(&self, state: &GameState) {
        let current_time = macroquad::time::get_time();
        let (sw, _sh) = virtual_screen_size();

        for event in &state.ui_state.quest_completed_events {
            let age = (current_time - event.time) as f32;
            if age > 4.0 {
                continue;
            }

            let alpha = if age > 3.0 {
                ((4.0 - age) * 255.0) as u8
            } else {
                255
            };

            let scale = if age < 0.3 {
                let t = age / 0.3;
                1.3 - 0.3 * t * t
            } else {
                1.0
            };

            let float_offset = (age * 10.0).min(30.0);
            let base_y = 120.0 - float_offset;

            if let Some(texture) = &self.quest_complete_texture {
                let tex_width = texture.width() * scale;
                let tex_height = texture.height() * scale;
                let x = (sw - tex_width) / 2.0;
                let y = base_y - tex_height / 2.0;

                draw_texture_ex(
                    texture,
                    x,
                    y,
                    Color::from_rgba(255, 255, 255, alpha),
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(tex_width, tex_height)),
                        ..Default::default()
                    },
                );

                let name_width = self.measure_text_sharp(&event.quest_name, 16.0).width;
                self.draw_text_sharp(
                    &event.quest_name,
                    (sw - name_width) / 2.0,
                    y + tex_height + 8.0,
                    16.0,
                    Color::from_rgba(255, 255, 255, alpha),
                );

                let rewards = format!("+{} EXP  +{} Gold", event.exp_reward, event.gold_reward);
                let rewards_width = self.measure_text_sharp(&rewards, 16.0).width;
                self.draw_text_sharp(
                    &rewards,
                    (sw - rewards_width) / 2.0,
                    y + tex_height + 28.0,
                    16.0,
                    Color::from_rgba(100, 255, 100, alpha),
                );
            } else {
                let title = "QUEST COMPLETE!";
                let title_width = self.measure_text_sharp(title, 32.0).width;
                let x = (sw - title_width) / 2.0;

                let outline_color = Color::from_rgba(0, 0, 0, alpha);
                for ox in [-2.0, 2.0] {
                    for oy in [-2.0, 2.0] {
                        self.draw_text_sharp(title, x + ox, base_y + oy, 32.0, outline_color);
                    }
                }

                self.draw_text_sharp(title, x, base_y, 32.0, Color::from_rgba(255, 215, 0, alpha));

                let name_width = self.measure_text_sharp(&event.quest_name, 16.0).width;
                self.draw_text_sharp(
                    &event.quest_name,
                    (sw - name_width) / 2.0,
                    base_y + 25.0,
                    16.0,
                    Color::from_rgba(255, 255, 255, alpha),
                );

                let rewards = format!("+{} EXP  +{} Gold", event.exp_reward, event.gold_reward);
                let rewards_width = self.measure_text_sharp(&rewards, 16.0).width;
                self.draw_text_sharp(
                    &rewards,
                    (sw - rewards_width) / 2.0,
                    base_y + 45.0,
                    16.0,
                    Color::from_rgba(100, 255, 100, alpha),
                );
            }
        }
    }
}
