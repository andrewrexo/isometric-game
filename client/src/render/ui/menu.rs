//! Escape menu rendering

use macroquad::prelude::*;
use crate::game::GameState;
use crate::ui::{UiElementId, UiLayout};
use crate::util::virtual_screen_size;
use super::super::Renderer;
use super::common::*;

impl Renderer {
    /// Render the escape menu (settings and disconnect)
    pub(crate) fn render_escape_menu(&self, state: &GameState, layout: &mut UiLayout) {
        let (sw, sh) = virtual_screen_size();

        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

        // Compact menu sizing - fits on mobile
        let menu_width = 240.0;
        let menu_height = (sh - 40.0).min(340.0); // Cap height, leave margin
        let menu_x = ((sw - menu_width) / 2.0).floor();
        let menu_y = ((sh - menu_height) / 2.0).floor();

        // ===== PANEL FRAME =====
        self.draw_panel_frame(menu_x, menu_y, menu_width, menu_height);
        self.draw_corner_accents(menu_x, menu_y, menu_width, menu_height);

        // ===== HEADER =====
        let header_height = 24.0;
        draw_rectangle(menu_x + FRAME_THICKNESS, menu_y + FRAME_THICKNESS,
                      menu_width - FRAME_THICKNESS * 2.0, header_height, HEADER_BG);
        draw_line(menu_x + FRAME_THICKNESS, menu_y + FRAME_THICKNESS + header_height,
                 menu_x + menu_width - FRAME_THICKNESS, menu_y + FRAME_THICKNESS + header_height, 1.0, HEADER_BORDER);

        // Title centered in header
        let title = "SETTINGS";
        let title_width = self.measure_text_sharp(title, 16.0).width;
        self.draw_text_sharp(title, (menu_x + (menu_width - title_width) / 2.0).floor(),
                            (menu_y + FRAME_THICKNESS + 17.0).floor(), 16.0, TEXT_TITLE);

        // Get current mouse position for hover detection
        let (mouse_x, mouse_y) = mouse_position();

        // ===== CONTENT AREA =====
        let content_x = menu_x + FRAME_THICKNESS + 8.0;
        let mut y = menu_y + FRAME_THICKNESS + header_height + 8.0;

        // Shared dimensions
        let row_height = 26.0;
        let btn_height = 24.0;
        let slider_height = 16.0;
        let inner_width = menu_width - FRAME_THICKNESS * 2.0 - 16.0;

        // Helper to draw themed button
        let draw_button = |btn_x: f32, btn_y: f32, btn_w: f32, btn_h: f32, text: &str, is_selected: bool, is_hovered: bool, renderer: &Self| {
            let (bg_color, border_color) = if is_selected {
                (Color::new(0.180, 0.200, 0.145, 1.0), FRAME_ACCENT)
            } else if is_hovered {
                (SLOT_HOVER_BG, SLOT_BORDER)
            } else {
                (SLOT_BG_EMPTY, SLOT_BORDER)
            };

            draw_rectangle(btn_x, btn_y, btn_w, btn_h, border_color);
            draw_rectangle(btn_x + 1.0, btn_y + 1.0, btn_w - 2.0, btn_h - 2.0, bg_color);

            if is_selected || is_hovered {
                draw_line(btn_x + 2.0, btn_y + 2.0, btn_x + btn_w - 2.0, btn_y + 2.0, 1.0, FRAME_INNER);
            }

            let text_width = renderer.measure_text_sharp(text, 16.0).width;
            let text_color = if is_selected { TEXT_TITLE } else { TEXT_NORMAL };
            renderer.draw_text_sharp(text, (btn_x + (btn_w - text_width) / 2.0).floor(),
                                    (btn_y + 17.0).floor(), 16.0, text_color);
        };

        // Helper to check hover
        let is_hovered = |bounds: Rect| -> bool {
            mouse_x >= bounds.x && mouse_x <= bounds.x + bounds.w
                && mouse_y >= bounds.y && mouse_y <= bounds.y + bounds.h
        };

        // ===== ZOOM ROW =====
        let zoom_btn_w = (inner_width - 12.0) / 3.0;
        let zoom_05x_bounds = Rect::new(content_x, y, zoom_btn_w, btn_height);
        let zoom_1x_bounds = Rect::new(content_x + zoom_btn_w + 6.0, y, zoom_btn_w, btn_height);
        let zoom_2x_bounds = Rect::new(content_x + (zoom_btn_w + 6.0) * 2.0, y, zoom_btn_w, btn_height);
        layout.add(UiElementId::EscapeMenuZoom05x, zoom_05x_bounds);
        layout.add(UiElementId::EscapeMenuZoom1x, zoom_1x_bounds);
        layout.add(UiElementId::EscapeMenuZoom2x, zoom_2x_bounds);

        let is_05x_selected = (state.camera.zoom - 0.5).abs() < 0.1;
        let is_1x_selected = (state.camera.zoom - 1.0).abs() < 0.1;
        let is_2x_selected = (state.camera.zoom - 2.0).abs() < 0.1;
        draw_button(zoom_05x_bounds.x, zoom_05x_bounds.y, zoom_btn_w, btn_height, "0.5x", is_05x_selected, is_hovered(zoom_05x_bounds), self);
        draw_button(zoom_1x_bounds.x, zoom_1x_bounds.y, zoom_btn_w, btn_height, "1x", is_1x_selected, is_hovered(zoom_1x_bounds), self);
        draw_button(zoom_2x_bounds.x, zoom_2x_bounds.y, zoom_btn_w, btn_height, "2x", is_2x_selected, is_hovered(zoom_2x_bounds), self);
        y += row_height + 4.0;

        // ===== AUDIO SLIDERS =====
        let slider_width = inner_width - 50.0;
        let slider_x = content_x + 42.0;

        // Music slider
        let music_bounds = Rect::new(slider_x, y, slider_width, slider_height);
        layout.add(UiElementId::EscapeMenuMusicSlider, music_bounds);
        self.draw_compact_slider("Music", slider_x, y, slider_width, slider_height,
                                 state.ui_state.audio_volume, state.ui_state.audio_muted, is_hovered(music_bounds));
        y += row_height - 4.0;

        // SFX slider
        let sfx_bounds = Rect::new(slider_x, y, slider_width, slider_height);
        layout.add(UiElementId::EscapeMenuSfxSlider, sfx_bounds);
        self.draw_compact_slider("SFX", slider_x, y, slider_width, slider_height,
                                 state.ui_state.audio_sfx_volume, state.ui_state.audio_muted, is_hovered(sfx_bounds));
        y += row_height - 4.0;

        // UI Scale slider
        let scale_bounds = Rect::new(slider_x, y, slider_width, slider_height);
        layout.add(UiElementId::EscapeMenuUiScaleSlider, scale_bounds);
        let scale_normalized = (state.ui_state.ui_scale - 0.75) / 0.5; // 0.75-1.25 range
        self.draw_compact_slider("Scale", slider_x, y, slider_width, slider_height,
                                 scale_normalized, false, is_hovered(scale_bounds));
        y += row_height;

        // ===== TOGGLE BUTTONS (2 per row) =====
        let toggle_w = (inner_width - 6.0) / 2.0;

        // Row 1: Mute + Shift-Drop
        let mute_bounds = Rect::new(content_x, y, toggle_w, btn_height);
        let shift_drop_bounds = Rect::new(content_x + toggle_w + 6.0, y, toggle_w, btn_height);
        layout.add(UiElementId::EscapeMenuMuteToggle, mute_bounds);
        layout.add(UiElementId::EscapeMenuShiftDropToggle, shift_drop_bounds);

        let mute_text = if state.ui_state.audio_muted { "Muted" } else { "Mute" };
        draw_button(mute_bounds.x, mute_bounds.y, toggle_w, btn_height, mute_text, state.ui_state.audio_muted, is_hovered(mute_bounds), self);
        let shift_text = if state.ui_state.shift_drop_enabled { "ShftDrp" } else { "ShftDrp" };
        draw_button(shift_drop_bounds.x, shift_drop_bounds.y, toggle_w, btn_height, shift_text, state.ui_state.shift_drop_enabled, is_hovered(shift_drop_bounds), self);
        y += row_height;

        // Row 2: Chat Log + Tap Walk
        let chat_bounds = Rect::new(content_x, y, toggle_w, btn_height);
        let tap_walk_bounds = Rect::new(content_x + toggle_w + 6.0, y, toggle_w, btn_height);
        layout.add(UiElementId::EscapeMenuChatLogToggle, chat_bounds);
        layout.add(UiElementId::EscapeMenuTapPathfindToggle, tap_walk_bounds);

        let chat_text = if state.ui_state.chat_log_visible { "Chat" } else { "Chat" };
        draw_button(chat_bounds.x, chat_bounds.y, toggle_w, btn_height, chat_text, state.ui_state.chat_log_visible, is_hovered(chat_bounds), self);
        let tap_text = if state.ui_state.tap_to_pathfind { "TapWalk" } else { "TapWalk" };
        draw_button(tap_walk_bounds.x, tap_walk_bounds.y, toggle_w, btn_height, tap_text, state.ui_state.tap_to_pathfind, is_hovered(tap_walk_bounds), self);
        y += row_height + 8.0;

        // ===== DISCONNECT BUTTON =====
        let disconnect_width = inner_width;
        let disconnect_height = 28.0;
        let disconnect_x = content_x;
        let disconnect_y = y;
        let disconnect_bounds = Rect::new(disconnect_x, disconnect_y, disconnect_width, disconnect_height);
        layout.add(UiElementId::EscapeMenuDisconnect, disconnect_bounds);

        let disconnect_hovered = is_hovered(disconnect_bounds);
        let disconnect_bg = if disconnect_hovered {
            Color::new(0.35, 0.15, 0.15, 1.0)
        } else {
            Color::new(0.25, 0.12, 0.12, 1.0)
        };
        let disconnect_border = Color::new(0.5, 0.2, 0.2, 1.0);

        draw_rectangle(disconnect_x, disconnect_y, disconnect_width, disconnect_height, disconnect_border);
        draw_rectangle(disconnect_x + 1.0, disconnect_y + 1.0, disconnect_width - 2.0, disconnect_height - 2.0, disconnect_bg);

        if disconnect_hovered {
            draw_line(disconnect_x + 2.0, disconnect_y + 2.0, disconnect_x + disconnect_width - 2.0, disconnect_y + 2.0, 1.0,
                     Color::new(0.6, 0.3, 0.3, 1.0));
        }

        let disconnect_text = "Disconnect";
        let disconnect_text_width = self.measure_text_sharp(disconnect_text, 16.0).width;
        let disconnect_text_color = if disconnect_hovered { Color::new(1.0, 0.8, 0.8, 1.0) } else { Color::new(0.85, 0.7, 0.7, 1.0) };
        self.draw_text_sharp(disconnect_text, (disconnect_x + (disconnect_width - disconnect_text_width) / 2.0).floor(),
                            (disconnect_y + 19.0).floor(), 16.0, disconnect_text_color);

        // ===== FOOTER HINT =====
        let hint = "[Esc] Close";
        let hint_width = self.measure_text_sharp(hint, 16.0).width;
        self.draw_text_sharp(hint, (menu_x + (menu_width - hint_width) / 2.0).floor(),
                            (menu_y + menu_height - FRAME_THICKNESS - 6.0).floor(), 16.0, TEXT_DIM);
    }

    /// Draw a compact slider with label on left
    fn draw_compact_slider(&self, label: &str, x: f32, y: f32, width: f32, height: f32, value: f32, muted: bool, hovered: bool) {
        // Label to the left
        let label_width = self.measure_text_sharp(label, 16.0).width;
        self.draw_text_sharp(label, (x - label_width - 6.0).floor(), (y + 12.0).floor(), 16.0, TEXT_DIM);

        // Track
        let track_color = if hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
        draw_rectangle(x, y, width, height, SLOT_BORDER);
        draw_rectangle(x + 1.0, y + 1.0, width - 2.0, height - 2.0, track_color);

        // Fill
        let fill_width = (width - 4.0) * value;
        let fill_color = if muted {
            Color::new(0.3, 0.3, 0.35, 1.0)
        } else {
            Color::new(0.4, 0.55, 0.3, 1.0)
        };
        draw_rectangle(x + 2.0, y + 2.0, fill_width, height - 4.0, fill_color);

        // Handle
        let handle_x = x + 2.0 + fill_width - 3.0;
        let handle_color = if muted { Color::new(0.5, 0.5, 0.55, 1.0) } else { FRAME_ACCENT };
        draw_rectangle(handle_x.max(x + 2.0), y + 2.0, 6.0, height - 4.0, handle_color);
    }
}
