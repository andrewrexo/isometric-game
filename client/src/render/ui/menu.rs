//! Escape menu rendering

use macroquad::prelude::*;
use crate::game::GameState;
use crate::ui::{UiElementId, UiLayout};
use super::super::Renderer;
use super::common::*;

impl Renderer {
    /// Render the escape menu (settings and disconnect)
    pub(crate) fn render_escape_menu(&self, state: &GameState, layout: &mut UiLayout) {
        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.5));

        let menu_width = 260.0;
        let menu_height = 200.0;
        let menu_x = ((screen_width() - menu_width) / 2.0).floor();
        let menu_y = ((screen_height() - menu_height) / 2.0).floor();

        // ===== PANEL FRAME =====
        self.draw_panel_frame(menu_x, menu_y, menu_width, menu_height);
        self.draw_corner_accents(menu_x, menu_y, menu_width, menu_height);

        // ===== HEADER =====
        let header_height = 32.0;
        draw_rectangle(menu_x + FRAME_THICKNESS, menu_y + FRAME_THICKNESS,
                      menu_width - FRAME_THICKNESS * 2.0, header_height, HEADER_BG);
        draw_line(menu_x + FRAME_THICKNESS, menu_y + FRAME_THICKNESS + header_height,
                 menu_x + menu_width - FRAME_THICKNESS, menu_y + FRAME_THICKNESS + header_height, 1.0, HEADER_BORDER);

        // Title centered in header
        let title = "MENU";
        let title_width = self.measure_text_sharp(title, 16.0).width;
        self.draw_text_sharp(title, (menu_x + (menu_width - title_width) / 2.0).floor(),
                            (menu_y + FRAME_THICKNESS + 22.0).floor(), 16.0, TEXT_TITLE);

        // Decorative dots
        draw_rectangle(menu_x + FRAME_THICKNESS + 10.0, menu_y + FRAME_THICKNESS + 14.0, 3.0, 3.0, FRAME_ACCENT);
        draw_rectangle(menu_x + FRAME_THICKNESS + 16.0, menu_y + FRAME_THICKNESS + 14.0, 3.0, 3.0, FRAME_ACCENT);
        draw_rectangle(menu_x + menu_width - FRAME_THICKNESS - 13.0, menu_y + FRAME_THICKNESS + 14.0, 3.0, 3.0, FRAME_ACCENT);
        draw_rectangle(menu_x + menu_width - FRAME_THICKNESS - 19.0, menu_y + FRAME_THICKNESS + 14.0, 3.0, 3.0, FRAME_ACCENT);

        // ===== CONTENT AREA =====
        let content_x = menu_x + FRAME_THICKNESS + 12.0;
        let content_y = menu_y + FRAME_THICKNESS + header_height + 12.0;

        // Camera Zoom label
        self.draw_text_sharp("Camera Zoom", content_x.floor(), (content_y + 12.0).floor(), 16.0, TEXT_DIM);

        // Get current mouse position for hover detection
        let (mouse_x, mouse_y) = mouse_position();

        // Zoom buttons
        let button_width = 90.0;
        let button_height = 30.0;
        let button_y = content_y + 22.0;
        let button_spacing = 16.0;
        let buttons_total_width = button_width * 2.0 + button_spacing;
        let buttons_start_x = menu_x + (menu_width - buttons_total_width) / 2.0;

        // Helper to draw themed button
        let draw_zoom_button = |btn_x: f32, btn_y: f32, text: &str, is_selected: bool, is_hovered: bool, renderer: &Self| {
            let (bg_color, border_color) = if is_selected {
                (Color::new(0.180, 0.200, 0.145, 1.0), FRAME_ACCENT) // Selected: greenish tint with gold border
            } else if is_hovered {
                (SLOT_HOVER_BG, SLOT_BORDER)
            } else {
                (SLOT_BG_EMPTY, SLOT_BORDER)
            };

            // Button border and background
            draw_rectangle(btn_x, btn_y, button_width, button_height, border_color);
            draw_rectangle(btn_x + 1.0, btn_y + 1.0, button_width - 2.0, button_height - 2.0, bg_color);

            // Inner highlight
            if is_selected || is_hovered {
                draw_line(btn_x + 2.0, btn_y + 2.0, btn_x + button_width - 2.0, btn_y + 2.0, 1.0, FRAME_INNER);
            }

            // Text centered
            let text_width = renderer.measure_text_sharp(text, 16.0).width;
            let text_color = if is_selected { TEXT_TITLE } else { TEXT_NORMAL };
            renderer.draw_text_sharp(text, (btn_x + (button_width - text_width) / 2.0).floor(),
                                    (btn_y + 20.0).floor(), 16.0, text_color);
        };

        // 1x Zoom button
        let zoom_1x_bounds = Rect::new(buttons_start_x.floor(), button_y.floor(), button_width, button_height);
        layout.add(UiElementId::EscapeMenuZoom1x, zoom_1x_bounds);
        let is_1x_hovered = mouse_x >= zoom_1x_bounds.x && mouse_x <= zoom_1x_bounds.x + zoom_1x_bounds.w
            && mouse_y >= zoom_1x_bounds.y && mouse_y <= zoom_1x_bounds.y + zoom_1x_bounds.h;
        let is_1x_selected = (state.camera.zoom - 1.0).abs() < 0.1;
        draw_zoom_button(zoom_1x_bounds.x, zoom_1x_bounds.y, "1x Zoom", is_1x_selected, is_1x_hovered, self);

        // 2x Zoom button
        let zoom_2x_bounds = Rect::new((buttons_start_x + button_width + button_spacing).floor(), button_y.floor(), button_width, button_height);
        layout.add(UiElementId::EscapeMenuZoom2x, zoom_2x_bounds);
        let is_2x_hovered = mouse_x >= zoom_2x_bounds.x && mouse_x <= zoom_2x_bounds.x + zoom_2x_bounds.w
            && mouse_y >= zoom_2x_bounds.y && mouse_y <= zoom_2x_bounds.y + zoom_2x_bounds.h;
        let is_2x_selected = (state.camera.zoom - 2.0).abs() < 0.1;
        draw_zoom_button(zoom_2x_bounds.x, zoom_2x_bounds.y, "2x Zoom", is_2x_selected, is_2x_hovered, self);

        // ===== DISCONNECT BUTTON =====
        let disconnect_width = 160.0;
        let disconnect_height = 32.0;
        let disconnect_x = (menu_x + (menu_width - disconnect_width) / 2.0).floor();
        let disconnect_y = (menu_y + menu_height - FRAME_THICKNESS - disconnect_height - 28.0).floor();
        let disconnect_bounds = Rect::new(disconnect_x, disconnect_y, disconnect_width, disconnect_height);
        layout.add(UiElementId::EscapeMenuDisconnect, disconnect_bounds);
        let is_disconnect_hovered = mouse_x >= disconnect_bounds.x && mouse_x <= disconnect_bounds.x + disconnect_bounds.w
            && mouse_y >= disconnect_bounds.y && mouse_y <= disconnect_bounds.y + disconnect_bounds.h;

        // Red-tinted disconnect button
        let disconnect_bg = if is_disconnect_hovered {
            Color::new(0.35, 0.15, 0.15, 1.0)
        } else {
            Color::new(0.25, 0.12, 0.12, 1.0)
        };
        let disconnect_border = Color::new(0.5, 0.2, 0.2, 1.0);

        draw_rectangle(disconnect_x, disconnect_y, disconnect_width, disconnect_height, disconnect_border);
        draw_rectangle(disconnect_x + 1.0, disconnect_y + 1.0, disconnect_width - 2.0, disconnect_height - 2.0, disconnect_bg);

        if is_disconnect_hovered {
            draw_line(disconnect_x + 2.0, disconnect_y + 2.0, disconnect_x + disconnect_width - 2.0, disconnect_y + 2.0, 1.0,
                     Color::new(0.6, 0.3, 0.3, 1.0));
        }

        let disconnect_text = "Disconnect";
        let disconnect_text_width = self.measure_text_sharp(disconnect_text, 16.0).width;
        let disconnect_text_color = if is_disconnect_hovered { Color::new(1.0, 0.8, 0.8, 1.0) } else { Color::new(0.85, 0.7, 0.7, 1.0) };
        self.draw_text_sharp(disconnect_text, (disconnect_x + (disconnect_width - disconnect_text_width) / 2.0).floor(),
                            (disconnect_y + 21.0).floor(), 16.0, disconnect_text_color);

        // ===== FOOTER HINT =====
        let hint = "[Esc] Close";
        let hint_width = self.measure_text_sharp(hint, 16.0).width;
        self.draw_text_sharp(hint, (menu_x + (menu_width - hint_width) / 2.0).floor(),
                            (menu_y + menu_height - FRAME_THICKNESS - 8.0).floor(), 16.0, TEXT_DIM);
    }
}
