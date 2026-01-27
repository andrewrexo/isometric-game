//! NPC dialogue panel rendering

use macroquad::prelude::*;
use macroquad::window::get_internal_gl;
use crate::game::ActiveDialogue;
use crate::ui::{UiElementId, UiLayout};
use crate::util::virtual_screen_size;
use super::super::Renderer;
use super::common::*;

impl Renderer {
    pub(crate) fn render_dialogue(&self, dialogue: &ActiveDialogue, hovered: &Option<UiElementId>, layout: &mut UiLayout, scroll_offset: f32, scrollbar_dragging: bool) {
        let (sw, sh) = virtual_screen_size();

        let is_mobile = cfg!(target_os = "android");

        // Semi-transparent overlay to focus attention
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.45));

        // Responsive width: cap at 620, with 10px margin each side
        let box_width = sw.min(620.0 + 20.0) - 20.0;

        // Mobile-aware sizing
        let (choice_btn_height, choice_spacing) = if is_mobile {
            (30.0, 38.0)
        } else {
            (26.0, 32.0)
        };

        let bottom_margin = if is_mobile { 20.0 } else { 60.0 };

        let choice_area_height = if dialogue.choices.is_empty() {
            0.0
        } else {
            dialogue.choices.len() as f32 * choice_spacing + 36.0
        };
        let text_margin_bottom = 12.0;
        let ideal_box_height = 120.0 + text_margin_bottom + choice_area_height;

        // Clamp height to screen bounds (leave 40px top margin minimum)
        let max_box_height = sh - 40.0 - bottom_margin;
        let box_height = ideal_box_height.min(max_box_height);
        let is_clamped = ideal_box_height > max_box_height;

        let box_x = (sw - box_width) / 2.0;
        let box_y = sh - box_height - bottom_margin;

        // Draw themed panel frame with corner accents
        self.draw_panel_frame(box_x, box_y, box_width, box_height);
        self.draw_corner_accents(box_x, box_y, box_width, box_height);

        // ===== CLOSE BUTTON (top-right corner) =====
        if !dialogue.choices.is_empty() {
            let close_size = if is_mobile { 32.0 } else { 24.0 };
            let close_x = box_x + box_width - close_size - FRAME_THICKNESS - 4.0;
            let close_y = box_y + FRAME_THICKNESS + 4.0;

            let bounds = Rect::new(close_x, close_y, close_size, close_size);
            layout.add(UiElementId::DialogueClose, bounds);

            let is_hovered = matches!(hovered, Some(UiElementId::DialogueClose));
            let (btn_bg, btn_border) = if is_hovered {
                (Color::new(0.4, 0.15, 0.15, 1.0), Color::new(0.6, 0.2, 0.2, 1.0))
            } else {
                (Color::new(0.2, 0.1, 0.1, 1.0), FRAME_MID)
            };

            draw_rectangle(close_x, close_y, close_size, close_size, btn_border);
            draw_rectangle(close_x + 1.0, close_y + 1.0, close_size - 2.0, close_size - 2.0, btn_bg);

            let cx = close_x + close_size / 2.0;
            let cy = close_y + close_size / 2.0;
            let cross = close_size * 0.25;
            let cross_color = if is_hovered { TEXT_TITLE } else { TEXT_DIM };
            draw_line(cx - cross, cy - cross, cx + cross, cy + cross, 2.0, cross_color);
            draw_line(cx + cross, cy - cross, cx - cross, cy + cross, 2.0, cross_color);
        }

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

        // Decorative line under speaker area (shortened when close button is present)
        let line_end = if !dialogue.choices.is_empty() {
            let close_size = if is_mobile { 32.0 } else { 24.0 };
            box_x + box_width - close_size - FRAME_THICKNESS - 4.0 - 8.0
        } else {
            content_x + content_width
        };
        draw_line(content_x, content_y, line_end, content_y, 1.0, HEADER_BORDER);

        // Dialogue text with word wrap
        let text_x = content_x;
        let text_y = content_y + 28.0;
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
            let choice_start_y = box_y + FRAME_THICKNESS + 70.0 + text_margin_bottom;

            // Calculate visible area for choices when clamped
            let choice_area_top = choice_start_y;
            let choice_area_bottom = box_y + box_height - FRAME_THICKNESS - 20.0;
            let visible_choice_height = choice_area_bottom - choice_area_top;

            // Calculate max scroll
            let total_choice_content = dialogue.choices.len() as f32 * choice_spacing;
            let max_scroll = (total_choice_content - visible_choice_height).max(0.0);
            let needs_scroll = max_scroll > 0.0;
            let clamped_scroll = scroll_offset.clamp(0.0, max_scroll);

            // Scrollbar margin
            let scrollbar_width: f32 = if is_mobile { 20.0 } else { 14.0 };

            // Apply scissor clipping when choices overflow the visible area
            if needs_scroll {
                let physical_w = screen_width();
                let physical_h = screen_height();
                let scale_x = physical_w / sw;
                let scale_y = physical_h / sh;
                let mut gl = unsafe { get_internal_gl() };
                gl.flush();
                let clip_width = content_width - scrollbar_width - 4.0;
                gl.quad_gl.scissor(Some((
                    (content_x * scale_x) as i32,
                    (choice_area_top * scale_y) as i32,
                    (clip_width * scale_x) as i32,
                    (visible_choice_height * scale_y) as i32,
                )));
            }

            for (i, choice) in dialogue.choices.iter().enumerate() {
                let choice_y = choice_start_y + (i as f32 * choice_spacing) - clamped_scroll;

                // Skip rendering if outside visible area
                if needs_scroll && (choice_y + choice_btn_height < choice_area_top || choice_y > choice_area_bottom) {
                    continue;
                }

                let choice_width = if needs_scroll { content_width - scrollbar_width - 4.0 } else { content_width };
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
                self.draw_text_sharp(&num_text, choice_x + 8.0, choice_y + choice_btn_height * 0.65, 16.0, num_color);

                let text_color = if is_hovered { TEXT_TITLE } else { TEXT_NORMAL };
                self.draw_text_sharp(&choice.text, choice_x + 40.0, choice_y + choice_btn_height * 0.65, 16.0, text_color);
            }

            if needs_scroll {
                let mut gl = unsafe { get_internal_gl() };
                gl.flush();
                gl.quad_gl.scissor(None);

                // Scrollbar track and thumb
                let track_x = content_x + content_width - scrollbar_width;
                let track_y = choice_area_top;
                let track_h = visible_choice_height;

                // Register scrollbar hit area
                layout.add(UiElementId::DialogueScrollbar, Rect::new(track_x, track_y, scrollbar_width, track_h));

                // Draw track background
                draw_rectangle(track_x, track_y, scrollbar_width, track_h, Color::new(0.1, 0.09, 0.12, 0.6));

                // Draw thumb
                let thumb_ratio = visible_choice_height / total_choice_content;
                let thumb_h = (track_h * thumb_ratio).max(20.0);
                let scroll_ratio = if max_scroll > 0.0 { clamped_scroll / max_scroll } else { 0.0 };
                let thumb_y = track_y + scroll_ratio * (track_h - thumb_h);

                let thumb_color = if scrollbar_dragging {
                    FRAME_ACCENT
                } else if matches!(hovered, Some(UiElementId::DialogueScrollbar)) {
                    FRAME_MID
                } else {
                    Color::new(0.3, 0.27, 0.35, 0.8)
                };
                draw_rectangle(track_x + 2.0, thumb_y, scrollbar_width - 4.0, thumb_h, thumb_color);
            }

            let hint_y = box_y + box_height - FRAME_THICKNESS - 10.0;
            self.draw_text_sharp("[1-4] Select", content_x, hint_y, 16.0, TEXT_DIM);
            self.draw_text_sharp("[Esc] Close", content_x + content_width - 75.0, hint_y, 16.0, TEXT_DIM);
        }
    }
}
