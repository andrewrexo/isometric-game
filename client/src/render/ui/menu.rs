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

        let menu_width = 260.0;
        let menu_height = 530.0;
        let menu_x = ((sw - menu_width) / 2.0).floor();
        let menu_y = ((sh - menu_height) / 2.0).floor();

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

        // Get current mouse position for hover detection
        let (mouse_x, mouse_y) = mouse_position();

        // Camera Zoom label
        self.draw_text_sharp("Camera Zoom", content_x.floor(), (content_y + 12.0).floor(), 16.0, TEXT_DIM);

        // Zoom buttons
        let button_width = 90.0;
        let button_height = 30.0;
        let button_y = content_y + 22.0;
        let button_spacing = 16.0;
        let buttons_total_width = button_width * 2.0 + button_spacing;
        let buttons_start_x = menu_x + (menu_width - buttons_total_width) / 2.0;

        // Helper to draw themed button
        let draw_button = |btn_x: f32, btn_y: f32, btn_w: f32, btn_h: f32, text: &str, is_selected: bool, is_hovered: bool, renderer: &Self| {
            let (bg_color, border_color) = if is_selected {
                (Color::new(0.180, 0.200, 0.145, 1.0), FRAME_ACCENT) // Selected: greenish tint with gold border
            } else if is_hovered {
                (SLOT_HOVER_BG, SLOT_BORDER)
            } else {
                (SLOT_BG_EMPTY, SLOT_BORDER)
            };

            // Button border and background
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, border_color);
            draw_rectangle(btn_x + 1.0, btn_y + 1.0, btn_w - 2.0, btn_h - 2.0, bg_color);

            // Inner highlight
            if is_selected || is_hovered {
                draw_line(btn_x + 2.0, btn_y + 2.0, btn_x + btn_w - 2.0, btn_y + 2.0, 1.0, FRAME_INNER);
            }

            // Text centered
            let text_width = renderer.measure_text_sharp(text, 16.0).width;
            let text_color = if is_selected { TEXT_TITLE } else { TEXT_NORMAL };
            renderer.draw_text_sharp(text, (btn_x + (btn_w - text_width) / 2.0).floor(),
                                    (btn_y + 20.0).floor(), 16.0, text_color);
        };

        // Helper to draw a volume slider
        let draw_slider = |label: &str, slider_x: f32, slider_y: f32, slider_width: f32, slider_height: f32,
                          volume: f32, muted: bool, renderer: &Self| {
            // Label
            let label_width = renderer.measure_text_sharp(label, 14.0).width;
            renderer.draw_text_sharp(label, (slider_x - label_width - 8.0).floor(), (slider_y + 14.0).floor(), 14.0, TEXT_DIM);

            // Slider track
            let is_hovered = mouse_x >= slider_x && mouse_x <= slider_x + slider_width
                && mouse_y >= slider_y && mouse_y <= slider_y + slider_height;
            let track_color = if is_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
            draw_rectangle(slider_x, slider_y, slider_width, slider_height, SLOT_BORDER);
            draw_rectangle(slider_x + 1.0, slider_y + 1.0, slider_width - 2.0, slider_height - 2.0, track_color);

            // Filled portion
            let fill_width = (slider_width - 4.0) * volume;
            let fill_color = if muted {
                Color::new(0.3, 0.3, 0.35, 1.0)
            } else {
                Color::new(0.4, 0.55, 0.3, 1.0)
            };
            draw_rectangle(slider_x + 2.0, slider_y + 2.0, fill_width, slider_height - 4.0, fill_color);

            // Handle
            let handle_x = slider_x + 2.0 + fill_width - 3.0;
            let handle_color = if muted {
                Color::new(0.5, 0.5, 0.55, 1.0)
            } else {
                FRAME_ACCENT
            };
            draw_rectangle(handle_x.max(slider_x + 2.0), slider_y + 2.0, 6.0, slider_height - 4.0, handle_color);

            // Percentage text
            let pct = (volume * 100.0).round() as i32;
            let pct_text = format!("{}%", pct);
            let text_color = if muted { TEXT_DIM } else { TEXT_NORMAL };
            renderer.draw_text_sharp(&pct_text, (slider_x + slider_width + 8.0).floor(), (slider_y + 14.0).floor(), 14.0, text_color);
        };

        // 1x Zoom button
        let zoom_1x_bounds = Rect::new(buttons_start_x.floor(), button_y.floor(), button_width, button_height);
        layout.add(UiElementId::EscapeMenuZoom1x, zoom_1x_bounds);
        let is_1x_hovered = mouse_x >= zoom_1x_bounds.x && mouse_x <= zoom_1x_bounds.x + zoom_1x_bounds.w
            && mouse_y >= zoom_1x_bounds.y && mouse_y <= zoom_1x_bounds.y + zoom_1x_bounds.h;
        let is_1x_selected = (state.camera.zoom - 1.0).abs() < 0.1;
        draw_button(zoom_1x_bounds.x, zoom_1x_bounds.y, button_width, button_height, "1x Zoom", is_1x_selected, is_1x_hovered, self);

        // 2x Zoom button
        let zoom_2x_bounds = Rect::new((buttons_start_x + button_width + button_spacing).floor(), button_y.floor(), button_width, button_height);
        layout.add(UiElementId::EscapeMenuZoom2x, zoom_2x_bounds);
        let is_2x_hovered = mouse_x >= zoom_2x_bounds.x && mouse_x <= zoom_2x_bounds.x + zoom_2x_bounds.w
            && mouse_y >= zoom_2x_bounds.y && mouse_y <= zoom_2x_bounds.y + zoom_2x_bounds.h;
        let is_2x_selected = (state.camera.zoom - 2.0).abs() < 0.1;
        draw_button(zoom_2x_bounds.x, zoom_2x_bounds.y, button_width, button_height, "2x Zoom", is_2x_selected, is_2x_hovered, self);

        // ===== AUDIO SECTION =====
        let audio_y = button_y + button_height + 20.0;
        self.draw_text_sharp("Audio", content_x.floor(), (audio_y + 12.0).floor(), 16.0, TEXT_DIM);

        let slider_width = 120.0;
        let slider_height = 18.0;
        let slider_x = (menu_x + 70.0).floor();

        // Music volume slider
        let music_slider_y = (audio_y + 28.0).floor();
        let music_slider_bounds = Rect::new(slider_x, music_slider_y, slider_width, slider_height);
        layout.add(UiElementId::EscapeMenuMusicSlider, music_slider_bounds);
        draw_slider("Music", slider_x, music_slider_y, slider_width, slider_height,
                   state.ui_state.audio_volume, state.ui_state.audio_muted, self);

        // SFX volume slider
        let sfx_slider_y = (music_slider_y + slider_height + 8.0).floor();
        let sfx_slider_bounds = Rect::new(slider_x, sfx_slider_y, slider_width, slider_height);
        layout.add(UiElementId::EscapeMenuSfxSlider, sfx_slider_bounds);
        draw_slider("SFX", slider_x, sfx_slider_y, slider_width, slider_height,
                   state.ui_state.audio_sfx_volume, state.ui_state.audio_muted, self);

        // Mute toggle button
        let mute_btn_width = 100.0;
        let mute_btn_height = 28.0;
        let mute_btn_x = (menu_x + (menu_width - mute_btn_width) / 2.0).floor();
        let mute_btn_y = (sfx_slider_y + slider_height + 10.0).floor();

        let mute_bounds = Rect::new(mute_btn_x, mute_btn_y, mute_btn_width, mute_btn_height);
        layout.add(UiElementId::EscapeMenuMuteToggle, mute_bounds);

        let is_mute_hovered = mouse_x >= mute_bounds.x && mouse_x <= mute_bounds.x + mute_bounds.w
            && mouse_y >= mute_bounds.y && mouse_y <= mute_bounds.y + mute_bounds.h;

        let mute_text = if state.ui_state.audio_muted { "Unmute" } else { "Mute" };
        draw_button(mute_btn_x, mute_btn_y, mute_btn_width, mute_btn_height, mute_text, state.ui_state.audio_muted, is_mute_hovered, self);

        // ===== UI SCALE SECTION =====
        let ui_scale_y = mute_btn_y + mute_btn_height + 16.0;
        self.draw_text_sharp("UI Scale", content_x.floor(), (ui_scale_y + 12.0).floor(), 16.0, TEXT_DIM);

        // UI Scale slider (0.5x to 1.5x)
        let ui_scale_slider_y = (ui_scale_y + 28.0).floor();
        let ui_scale_slider_bounds = Rect::new(slider_x, ui_scale_slider_y, slider_width, slider_height);
        layout.add(UiElementId::EscapeMenuUiScaleSlider, ui_scale_slider_bounds);

        // Draw UI scale slider (convert 0.5-1.5 range to 0.0-1.0 for display)
        let scale_normalized = (state.ui_state.ui_scale - 0.5) / 1.0; // 0.5->0.0, 1.5->1.0
        let is_scale_hovered = mouse_x >= slider_x && mouse_x <= slider_x + slider_width
            && mouse_y >= ui_scale_slider_y && mouse_y <= ui_scale_slider_y + slider_height;

        // Custom slider for UI scale (similar to audio slider but different colors)
        {
            let track_color = if is_scale_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
            draw_rectangle(slider_x, ui_scale_slider_y, slider_width, slider_height, SLOT_BORDER);
            draw_rectangle(slider_x + 1.0, ui_scale_slider_y + 1.0, slider_width - 2.0, slider_height - 2.0, track_color);

            // Filled portion
            let fill_width = (slider_width - 4.0) * scale_normalized;
            let fill_color = Color::new(0.3, 0.45, 0.6, 1.0); // Blue tint for scale
            draw_rectangle(slider_x + 2.0, ui_scale_slider_y + 2.0, fill_width, slider_height - 4.0, fill_color);

            // Handle
            let handle_x = slider_x + 2.0 + fill_width - 3.0;
            draw_rectangle(handle_x.max(slider_x + 2.0), ui_scale_slider_y + 2.0, 6.0, slider_height - 4.0, FRAME_ACCENT);

            // Scale text (show as multiplier)
            let scale_text = format!("{:.1}x", state.ui_state.ui_scale);
            self.draw_text_sharp(&scale_text, (slider_x + slider_width + 8.0).floor(), (ui_scale_slider_y + 14.0).floor(), 14.0, TEXT_NORMAL);

            // Label
            let label = "Scale";
            let label_width = self.measure_text_sharp(label, 14.0).width;
            self.draw_text_sharp(label, (slider_x - label_width - 8.0).floor(), (ui_scale_slider_y + 14.0).floor(), 14.0, TEXT_DIM);
        }

        // ===== INVENTORY SECTION =====
        let inventory_y = ui_scale_slider_y + slider_height + 16.0;
        self.draw_text_sharp("Inventory", content_x.floor(), (inventory_y + 12.0).floor(), 16.0, TEXT_DIM);

        // Shift-Drop toggle button
        let shift_drop_btn_width = 140.0;
        let shift_drop_btn_height = 28.0;
        let shift_drop_btn_x = (menu_x + (menu_width - shift_drop_btn_width) / 2.0).floor();
        let shift_drop_btn_y = (inventory_y + 22.0).floor();

        let shift_drop_bounds = Rect::new(shift_drop_btn_x, shift_drop_btn_y, shift_drop_btn_width, shift_drop_btn_height);
        layout.add(UiElementId::EscapeMenuShiftDropToggle, shift_drop_bounds);

        let is_shift_drop_hovered = mouse_x >= shift_drop_bounds.x && mouse_x <= shift_drop_bounds.x + shift_drop_bounds.w
            && mouse_y >= shift_drop_bounds.y && mouse_y <= shift_drop_bounds.y + shift_drop_bounds.h;

        let shift_drop_text = if state.ui_state.shift_drop_enabled { "Shift-Drop: ON" } else { "Shift-Drop: OFF" };
        draw_button(shift_drop_btn_x, shift_drop_btn_y, shift_drop_btn_width, shift_drop_btn_height, shift_drop_text, state.ui_state.shift_drop_enabled, is_shift_drop_hovered, self);

        // Chat Log toggle button
        let chat_log_btn_y = (shift_drop_btn_y + shift_drop_btn_height + 6.0).floor();
        let chat_log_bounds = Rect::new(shift_drop_btn_x, chat_log_btn_y, shift_drop_btn_width, shift_drop_btn_height);
        layout.add(UiElementId::EscapeMenuChatLogToggle, chat_log_bounds);

        let is_chat_log_hovered = mouse_x >= chat_log_bounds.x && mouse_x <= chat_log_bounds.x + chat_log_bounds.w
            && mouse_y >= chat_log_bounds.y && mouse_y <= chat_log_bounds.y + chat_log_bounds.h;

        let chat_log_text = if state.ui_state.chat_log_visible { "Chat Log: ON" } else { "Chat Log: OFF" };
        draw_button(shift_drop_btn_x, chat_log_btn_y, shift_drop_btn_width, shift_drop_btn_height, chat_log_text, state.ui_state.chat_log_visible, is_chat_log_hovered, self);

        // Tap-to-Pathfind toggle button
        let tap_path_btn_y = (chat_log_btn_y + shift_drop_btn_height + 6.0).floor();
        let tap_path_bounds = Rect::new(shift_drop_btn_x, tap_path_btn_y, shift_drop_btn_width, shift_drop_btn_height);
        layout.add(UiElementId::EscapeMenuTapPathfindToggle, tap_path_bounds);

        let is_tap_path_hovered = mouse_x >= tap_path_bounds.x && mouse_x <= tap_path_bounds.x + tap_path_bounds.w
            && mouse_y >= tap_path_bounds.y && mouse_y <= tap_path_bounds.y + tap_path_bounds.h;

        let tap_path_text = if state.ui_state.tap_to_pathfind { "Tap Walk: ON" } else { "Tap Walk: OFF" };
        draw_button(shift_drop_btn_x, tap_path_btn_y, shift_drop_btn_width, shift_drop_btn_height, tap_path_text, state.ui_state.tap_to_pathfind, is_tap_path_hovered, self);

        // ===== CONTROLS SECTION =====
        let controls_y = tap_path_btn_y + shift_drop_btn_height + 16.0;
        self.draw_text_sharp("Controls", content_x.floor(), (controls_y + 12.0).floor(), 16.0, TEXT_DIM);

        let controls_text_y = controls_y + 28.0;
        self.draw_text_sharp("WASD: Move", content_x.floor(), (controls_text_y).floor(), 16.0, TEXT_NORMAL);
        self.draw_text_sharp("Space: Attack", content_x.floor(), (controls_text_y + 16.0).floor(), 16.0, TEXT_NORMAL);
        self.draw_text_sharp("I: Inventory  E: Interact", content_x.floor(), (controls_text_y + 32.0).floor(), 16.0, TEXT_NORMAL);
        self.draw_text_sharp("Q: Quests  F: Pickup", content_x.floor(), (controls_text_y + 48.0).floor(), 16.0, TEXT_NORMAL);
        self.draw_text_sharp("F3: Debug", content_x.floor(), (controls_text_y + 64.0).floor(), 16.0, TEXT_NORMAL);

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
