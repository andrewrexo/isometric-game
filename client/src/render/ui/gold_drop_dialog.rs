//! Gold drop dialog rendering

use macroquad::prelude::*;
use crate::game::GoldDropDialog;
use crate::ui::{UiElementId, UiLayout};
use crate::util::virtual_screen_size;
use super::super::Renderer;
use super::common::*;

impl Renderer {
    /// Render the gold drop amount dialog
    pub(crate) fn render_gold_drop_dialog(&self, dialog: &GoldDropDialog, player_gold: i32, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let (sw, sh) = virtual_screen_size();

        // Semi-transparent overlay to focus attention
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.45));

        let box_width = 280.0;
        let box_height = 140.0;
        let box_x = (sw - box_width) / 2.0;
        let box_y = (sh - box_height) / 2.0;

        // Draw themed panel frame with corner accents
        self.draw_panel_frame(box_x, box_y, box_width, box_height);
        self.draw_corner_accents(box_x, box_y, box_width, box_height);

        // ===== TITLE TAB =====
        let title_text = "DROP GOLD";
        let title_width = self.measure_text_sharp(title_text, 16.0).width + 28.0;
        let title_x = box_x + (box_width - title_width) / 2.0;
        let title_y = box_y - 8.0;
        let title_h = 26.0;

        // Title tab with beveled effect
        draw_rectangle(title_x - 1.0, title_y - 1.0, title_width + 2.0, title_h + 2.0, FRAME_OUTER);
        draw_rectangle(title_x, title_y, title_width, title_h, HEADER_BG);
        draw_rectangle(title_x + 1.0, title_y + 1.0, title_width - 2.0, title_h - 2.0, Color::new(0.165, 0.149, 0.188, 1.0));

        // Title tab inner highlight
        draw_line(title_x + 2.0, title_y + 2.0, title_x + title_width - 2.0, title_y + 2.0, 1.0, FRAME_INNER);

        // Title text in gold
        self.draw_text_sharp(title_text, title_x + 14.0, title_y + 18.0, 16.0, TEXT_TITLE);

        // Small decorative accent on title tab corners
        draw_rectangle(title_x, title_y, 3.0, 1.0, FRAME_ACCENT);
        draw_rectangle(title_x + title_width - 3.0, title_y, 3.0, 1.0, FRAME_ACCENT);

        // ===== CONTENT AREA =====
        let content_x = box_x + FRAME_THICKNESS + 12.0;
        let content_y = box_y + FRAME_THICKNESS + 16.0;
        let content_width = box_width - FRAME_THICKNESS * 2.0 - 24.0;

        // Current gold display
        let gold_text = format!("Available: {}g", player_gold);
        self.draw_text_sharp(&gold_text, content_x, content_y + 16.0, 16.0, TEXT_GOLD);

        // ===== INPUT FIELD =====
        let input_y = content_y + 36.0;
        let input_height = 28.0;
        let input_width = content_width;

        // Input field background
        draw_rectangle(content_x, input_y, input_width, input_height, SLOT_BORDER);
        draw_rectangle(content_x + 1.0, input_y + 1.0, input_width - 2.0, input_height - 2.0, SLOT_BG_EMPTY);

        // Inner shadow
        draw_line(content_x + 2.0, input_y + 2.0, content_x + input_width - 2.0, input_y + 2.0, 1.0, SLOT_INNER_SHADOW);
        draw_line(content_x + 2.0, input_y + 2.0, content_x + 2.0, input_y + input_height - 2.0, 1.0, SLOT_INNER_SHADOW);

        // Input text
        let input_text_x = content_x + 8.0;
        let input_text_y = input_y + 19.0;

        if dialog.input.is_empty() {
            self.draw_text_sharp("Enter amount...", input_text_x, input_text_y, 16.0, TEXT_DIM);
        } else {
            self.draw_text_sharp(&dialog.input, input_text_x, input_text_y, 16.0, TEXT_NORMAL);
        }

        // Blinking cursor
        let cursor_visible = (macroquad::time::get_time() * 2.0) as i32 % 2 == 0;
        if cursor_visible {
            let text_before_cursor: String = dialog.input.chars().take(dialog.cursor).collect();
            let cursor_x = input_text_x + self.measure_text_sharp(&text_before_cursor, 16.0).width;
            draw_rectangle(cursor_x, input_y + 6.0, 2.0, input_height - 12.0, TEXT_NORMAL);
        }

        // ===== BUTTONS =====
        let button_y = input_y + input_height + 12.0;
        let button_width = (content_width - 12.0) / 2.0;
        let button_height = 28.0;

        // Confirm button
        let confirm_x = content_x;
        let confirm_bounds = Rect::new(confirm_x, button_y, button_width, button_height);
        layout.add(UiElementId::GoldDropConfirm, confirm_bounds);

        let confirm_hovered = matches!(hovered, Some(UiElementId::GoldDropConfirm));
        let (confirm_bg, confirm_border) = if confirm_hovered {
            (Color::new(0.235, 0.204, 0.141, 1.0), FRAME_ACCENT)
        } else {
            (Color::new(0.157, 0.141, 0.110, 1.0), FRAME_MID)
        };

        draw_rectangle(confirm_x, button_y, button_width, button_height, confirm_border);
        draw_rectangle(confirm_x + 1.0, button_y + 1.0, button_width - 2.0, button_height - 2.0, confirm_bg);

        if confirm_hovered {
            draw_line(confirm_x + 2.0, button_y + 2.0, confirm_x + button_width - 2.0, button_y + 2.0, 1.0, FRAME_INNER);
        }

        let confirm_text_color = if confirm_hovered { TEXT_TITLE } else { TEXT_NORMAL };
        let confirm_text = "Confirm";
        let confirm_text_width = self.measure_text_sharp(confirm_text, 16.0).width;
        self.draw_text_sharp(confirm_text, confirm_x + (button_width - confirm_text_width) / 2.0, button_y + 19.0, 16.0, confirm_text_color);

        // Cancel button
        let cancel_x = content_x + button_width + 12.0;
        let cancel_bounds = Rect::new(cancel_x, button_y, button_width, button_height);
        layout.add(UiElementId::GoldDropCancel, cancel_bounds);

        let cancel_hovered = matches!(hovered, Some(UiElementId::GoldDropCancel));
        let (cancel_bg, cancel_border) = if cancel_hovered {
            (Color::new(0.235, 0.141, 0.141, 1.0), Color::new(0.8, 0.4, 0.4, 1.0))
        } else {
            (Color::new(0.157, 0.110, 0.110, 1.0), FRAME_MID)
        };

        draw_rectangle(cancel_x, button_y, button_width, button_height, cancel_border);
        draw_rectangle(cancel_x + 1.0, button_y + 1.0, button_width - 2.0, button_height - 2.0, cancel_bg);

        if cancel_hovered {
            draw_line(cancel_x + 2.0, button_y + 2.0, cancel_x + button_width - 2.0, button_y + 2.0, 1.0, FRAME_INNER);
        }

        let cancel_text_color = if cancel_hovered { TEXT_TITLE } else { TEXT_NORMAL };
        let cancel_text = "Cancel";
        let cancel_text_width = self.measure_text_sharp(cancel_text, 16.0).width;
        self.draw_text_sharp(cancel_text, cancel_x + (button_width - cancel_text_width) / 2.0, button_y + 19.0, 16.0, cancel_text_color);
    }
}
