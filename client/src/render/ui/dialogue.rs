//! NPC dialogue panel rendering

use macroquad::prelude::*;
use crate::game::ActiveDialogue;
use crate::ui::{UiElementId, UiLayout};
use crate::util::virtual_screen_size;
use super::super::Renderer;
use super::common::*;

impl Renderer {
    pub(crate) fn render_dialogue(&self, dialogue: &ActiveDialogue, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let (sw, sh) = virtual_screen_size();

        // Semi-transparent overlay to focus attention
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.45));

        let box_width = 620.0;
        let choice_area_height = if dialogue.choices.is_empty() {
            0.0
        } else {
            dialogue.choices.len() as f32 * 32.0 + 36.0
        };
        let box_height = 120.0 + choice_area_height;
        let box_x = (sw - box_width) / 2.0;
        let box_y = sh - box_height - 60.0;

        // Draw themed panel frame with corner accents
        self.draw_panel_frame(box_x, box_y, box_width, box_height);
        self.draw_corner_accents(box_x, box_y, box_width, box_height);

        // ===== SPEAKER NAME TAB =====
        let speaker_text = dialogue.speaker.to_uppercase();
        let speaker_width = self.measure_text_sharp(&speaker_text, 16.0).width + 28.0;
        let speaker_x = box_x + 20.0;
        let speaker_y = box_y - 8.0;
        let speaker_h = 26.0;

        // Speaker tab with beveled effect
        draw_rectangle(speaker_x - 1.0, speaker_y - 1.0, speaker_width + 2.0, speaker_h + 2.0, FRAME_OUTER);
        draw_rectangle(speaker_x, speaker_y, speaker_width, speaker_h, HEADER_BG);
        draw_rectangle(speaker_x + 1.0, speaker_y + 1.0, speaker_width - 2.0, speaker_h - 2.0, Color::new(0.165, 0.149, 0.188, 1.0));

        // Speaker tab inner highlight
        draw_line(speaker_x + 2.0, speaker_y + 2.0, speaker_x + speaker_width - 2.0, speaker_y + 2.0, 1.0, FRAME_INNER);

        // Speaker name in gold
        self.draw_text_sharp(&speaker_text, speaker_x + 14.0, speaker_y + 18.0, 16.0, TEXT_TITLE);

        // Small decorative accent on speaker tab corners
        draw_rectangle(speaker_x, speaker_y, 3.0, 1.0, FRAME_ACCENT);
        draw_rectangle(speaker_x + speaker_width - 3.0, speaker_y, 3.0, 1.0, FRAME_ACCENT);

        // ===== DIALOGUE CONTENT AREA =====
        let content_x = box_x + FRAME_THICKNESS + 12.0;
        let content_y = box_y + FRAME_THICKNESS + 20.0;
        let content_width = box_width - FRAME_THICKNESS * 2.0 - 24.0;

        // Decorative line under speaker area
        draw_line(content_x, content_y, content_x + content_width, content_y, 1.0, HEADER_BORDER);

        // Dialogue text with word wrap
        let text_x = content_x;
        let text_y = content_y + 24.0;
        let max_line_width = content_width;

        let words: Vec<&str> = dialogue.text.split_whitespace().collect();
        let mut current_line = String::new();
        let mut line_y = text_y;

        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            let line_width = self.measure_text_sharp(&test_line, 16.0).width;
            if line_width > max_line_width && !current_line.is_empty() {
                self.draw_text_sharp(&current_line, text_x, line_y, 16.0, TEXT_NORMAL);
                line_y += 22.0;
                current_line = word.to_string();
            } else {
                current_line = test_line;
            }
        }
        if !current_line.is_empty() {
            self.draw_text_sharp(&current_line, text_x, line_y, 16.0, TEXT_NORMAL);
        }

        // ===== CHOICES / CONTINUE =====
        if dialogue.choices.is_empty() {
            let hint = "[ Continue ]";
            let hint_width = self.measure_text_sharp(hint, 16.0).width + 20.0;
            let hint_x = box_x + box_width - hint_width - FRAME_THICKNESS - 15.0;
            let hint_y = box_y + box_height - FRAME_THICKNESS - 32.0;

            let bounds = Rect::new(hint_x, hint_y, hint_width, 24.0);
            layout.add(UiElementId::DialogueContinue, bounds);

            let is_hovered = matches!(hovered, Some(UiElementId::DialogueContinue));

            let (btn_bg, btn_border) = if is_hovered {
                (Color::new(0.235, 0.204, 0.141, 1.0), FRAME_ACCENT)
            } else {
                (Color::new(0.157, 0.141, 0.110, 1.0), FRAME_MID)
            };

            draw_rectangle(hint_x, hint_y, hint_width, 24.0, btn_border);
            draw_rectangle(hint_x + 1.0, hint_y + 1.0, hint_width - 2.0, 22.0, btn_bg);

            if is_hovered {
                draw_line(hint_x + 2.0, hint_y + 2.0, hint_x + hint_width - 2.0, hint_y + 2.0, 1.0, FRAME_INNER);
            }

            let text_color = if is_hovered { TEXT_TITLE } else { TEXT_NORMAL };
            self.draw_text_sharp(hint, hint_x + 10.0, hint_y + 17.0, 16.0, text_color);

            self.draw_text_sharp("[Enter]", box_x + FRAME_THICKNESS + 15.0, hint_y + 17.0, 16.0, TEXT_DIM);
        } else {
            // ===== CHOICE BUTTONS =====
            let choice_start_y = box_y + FRAME_THICKNESS + 70.0;
            let choice_btn_height = 26.0;
            let choice_spacing = 32.0;

            for (i, choice) in dialogue.choices.iter().enumerate() {
                let choice_y = choice_start_y + (i as f32 * choice_spacing);
                let choice_width = content_width;
                let choice_x = content_x;

                let bounds = Rect::new(choice_x, choice_y, choice_width, choice_btn_height);
                layout.add(UiElementId::DialogueChoice(i), bounds);

                let is_hovered = matches!(hovered, Some(UiElementId::DialogueChoice(idx)) if *idx == i);

                let (bg_color, border_color) = if is_hovered {
                    (SLOT_HOVER_BG, SLOT_SELECTED_BORDER)
                } else {
                    (SLOT_BG_EMPTY, SLOT_BORDER)
                };

                draw_rectangle(choice_x, choice_y, choice_width, choice_btn_height, border_color);
                draw_rectangle(choice_x + 1.0, choice_y + 1.0, choice_width - 2.0, choice_btn_height - 2.0, bg_color);

                if is_hovered {
                    draw_line(choice_x + 2.0, choice_y + 2.0, choice_x + choice_width - 2.0, choice_y + 2.0, 1.0, FRAME_INNER);
                    draw_line(choice_x + 2.0, choice_y + 2.0, choice_x + 2.0, choice_y + choice_btn_height - 2.0, 1.0, FRAME_INNER);
                }

                let num_text = format!("[{}]", i + 1);
                let num_color = if is_hovered { TEXT_GOLD } else { FRAME_MID };
                self.draw_text_sharp(&num_text, choice_x + 8.0, choice_y + 18.0, 16.0, num_color);

                let text_color = if is_hovered { TEXT_TITLE } else { TEXT_NORMAL };
                self.draw_text_sharp(&choice.text, choice_x + 40.0, choice_y + 18.0, 16.0, text_color);
            }

            let hint_y = box_y + box_height - FRAME_THICKNESS - 10.0;
            self.draw_text_sharp("[1-4] Select", content_x, hint_y, 16.0, TEXT_DIM);
            self.draw_text_sharp("[Esc] Close", content_x + content_width - 75.0, hint_y, 16.0, TEXT_DIM);
        }
    }
}
