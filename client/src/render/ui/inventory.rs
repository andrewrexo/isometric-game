//! Inventory panel rendering

use macroquad::prelude::*;
use macroquad::window::get_internal_gl;
use crate::game::{GameState, DragState, DragSource};
use crate::ui::{UiElementId, UiLayout};
use crate::util::virtual_screen_size;
use super::super::Renderer;
use super::common::*;

impl Renderer {
    /// Draw equipment slot with silhouette icon when empty
    pub(crate) fn draw_equipment_slot(&self, x: f32, y: f32, size: f32, slot_type: &str, has_item: bool, is_hovered: bool, is_dragging: bool) {
        // Outer border (purple accent for equipment)
        let border_color = if is_dragging {
            SLOT_SELECTED_BORDER
        } else if is_hovered {
            EQUIP_ACCENT
        } else {
            SLOT_BORDER
        };
        draw_rectangle(x, y, size, size, border_color);

        // Inner background
        let bg = if is_dragging {
            SLOT_DRAG_SOURCE
        } else if is_hovered {
            SLOT_HOVER_BG
        } else {
            EQUIP_SLOT_EMPTY
        };
        draw_rectangle(x + 1.0, y + 1.0, size - 2.0, size - 2.0, bg);

        // Inner bevel effect
        draw_line(x + 2.0, y + 2.0, x + size - 2.0, y + 2.0, 2.0, SLOT_INNER_SHADOW);
        draw_line(x + 2.0, y + 2.0, x + 2.0, y + size - 2.0, 2.0, SLOT_INNER_SHADOW);

        // Draw silhouette if empty (and not dragging)
        if !has_item && !is_dragging {
            let center_x = x + size / 2.0;
            let center_y = y + size / 2.0;
            let icon_color = Color::new(0.188, 0.188, 0.227, 1.0); // rgba(48, 48, 58, 255)

            match slot_type {
                "head" => {
                    // Helmet silhouette (rounded head shape)
                    draw_rectangle(center_x - 8.0, center_y - 8.0, 16.0, 14.0, icon_color);
                    draw_rectangle(center_x - 10.0, center_y - 4.0, 20.0, 8.0, icon_color);
                    draw_rectangle(center_x - 6.0, center_y - 12.0, 12.0, 6.0, icon_color);
                },
                "body" => {
                    // Armor silhouette (torso shape)
                    draw_rectangle(center_x - 8.0, center_y - 10.0, 16.0, 20.0, icon_color);
                    draw_rectangle(center_x - 12.0, center_y - 6.0, 5.0, 12.0, icon_color);
                    draw_rectangle(center_x + 7.0, center_y - 6.0, 5.0, 12.0, icon_color);
                },
                "weapon" => {
                    // Sword silhouette
                    draw_rectangle(center_x - 2.0, center_y - 14.0, 4.0, 24.0, icon_color);
                    draw_rectangle(center_x - 8.0, center_y + 4.0, 16.0, 4.0, icon_color);
                    draw_rectangle(center_x - 3.0, center_y + 8.0, 6.0, 4.0, icon_color);
                },
                "back" => {
                    // Cape/backpack silhouette
                    draw_rectangle(center_x - 10.0, center_y - 10.0, 20.0, 6.0, icon_color);
                    draw_rectangle(center_x - 8.0, center_y - 4.0, 16.0, 16.0, icon_color);
                    draw_rectangle(center_x - 6.0, center_y + 10.0, 12.0, 4.0, icon_color);
                },
                "feet" => {
                    // Boots silhouette
                    draw_rectangle(center_x - 8.0, center_y - 4.0, 6.0, 12.0, icon_color);
                    draw_rectangle(center_x + 2.0, center_y - 4.0, 6.0, 12.0, icon_color);
                    draw_rectangle(center_x - 10.0, center_y + 6.0, 9.0, 4.0, icon_color);
                    draw_rectangle(center_x + 1.0, center_y + 6.0, 9.0, 4.0, icon_color);
                },
                "ring" => {
                    // Ring silhouette (circular band)
                    draw_rectangle(center_x - 6.0, center_y - 8.0, 12.0, 4.0, icon_color);
                    draw_rectangle(center_x - 8.0, center_y - 4.0, 4.0, 8.0, icon_color);
                    draw_rectangle(center_x + 4.0, center_y - 4.0, 4.0, 8.0, icon_color);
                    draw_rectangle(center_x - 6.0, center_y + 4.0, 12.0, 4.0, icon_color);
                    // Gem on top
                    draw_rectangle(center_x - 3.0, center_y - 12.0, 6.0, 6.0, icon_color);
                },
                "gloves" => {
                    // Glove silhouette (hand shape)
                    draw_rectangle(center_x - 8.0, center_y - 2.0, 16.0, 12.0, icon_color);
                    // Fingers
                    draw_rectangle(center_x - 8.0, center_y - 10.0, 3.0, 10.0, icon_color);
                    draw_rectangle(center_x - 4.0, center_y - 12.0, 3.0, 12.0, icon_color);
                    draw_rectangle(center_x, center_y - 12.0, 3.0, 12.0, icon_color);
                    draw_rectangle(center_x + 4.0, center_y - 10.0, 3.0, 10.0, icon_color);
                    // Thumb
                    draw_rectangle(center_x + 8.0, center_y - 4.0, 4.0, 8.0, icon_color);
                },
                "necklace" => {
                    // Necklace silhouette (pendant on chain)
                    // Chain part (U shape)
                    draw_rectangle(center_x - 8.0, center_y - 10.0, 3.0, 8.0, icon_color);
                    draw_rectangle(center_x + 5.0, center_y - 10.0, 3.0, 8.0, icon_color);
                    draw_rectangle(center_x - 6.0, center_y - 2.0, 12.0, 3.0, icon_color);
                    // Pendant (diamond shape)
                    draw_rectangle(center_x - 4.0, center_y + 1.0, 8.0, 8.0, icon_color);
                    draw_rectangle(center_x - 2.0, center_y + 9.0, 4.0, 4.0, icon_color);
                },
                "belt" => {
                    // Belt silhouette (horizontal band with buckle)
                    draw_rectangle(center_x - 12.0, center_y - 3.0, 24.0, 6.0, icon_color);
                    // Buckle (square with center)
                    draw_rectangle(center_x - 5.0, center_y - 6.0, 10.0, 12.0, icon_color);
                    draw_rectangle(center_x - 2.0, center_y - 3.0, 4.0, 6.0, EQUIP_SLOT_EMPTY);
                },
                _ => {}
            }
        }
    }

    pub(crate) fn render_inventory(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let (screen_w, screen_h) = virtual_screen_size();
        let scale = state.ui_state.ui_scale;

        // Scaled dimensions (keep font at native size for crisp rendering)
        let inv_width = INV_WIDTH * scale;
        let inv_height_full = INV_HEIGHT * scale;
        let frame_thickness = FRAME_THICKNESS * scale;
        let header_height = HEADER_HEIGHT * scale;
        let button_size = MENU_BUTTON_SIZE * scale;
        let exp_bar_gap = EXP_BAR_GAP * scale;

        // Position panel on right side, above the menu buttons (align with button right edge)
        let inv_x = screen_w - inv_width - 8.0;
        let button_area_height = button_size + exp_bar_gap;

        // Calculate the minimum Y the panel can reach (below XP bar)
        let min_panel_y = EXP_BAR_HEIGHT + 4.0; // XP bar height + small margin
        let max_available_height = screen_h - button_area_height - 8.0 - min_panel_y;

        // Clamp panel height if it would overlap the XP bar
        let inv_height = inv_height_full.min(max_available_height);
        let inv_y = screen_h - button_area_height - inv_height - 8.0;

        // Draw panel frame with corner accents
        self.draw_panel_frame(inv_x, inv_y, inv_width, inv_height);
        self.draw_corner_accents(inv_x, inv_y, inv_width, inv_height);

        // ===== HEADER SECTION =====
        let header_x = inv_x + frame_thickness;
        let header_y = inv_y + frame_thickness;
        let header_w = inv_width - frame_thickness * 2.0;

        // Header background
        draw_rectangle(header_x, header_y, header_w, header_height, HEADER_BG);

        // Header bottom separator
        draw_line(header_x + 10.0 * scale, header_y + header_height, header_x + header_w - 10.0 * scale, header_y + header_height, 2.0, HEADER_BORDER);

        // Decorative dots on separator
        let dot_spacing = 50.0 * scale;
        let num_dots = ((header_w - 40.0 * scale) / dot_spacing).max(1.0) as i32;
        let start_dot_x = header_x + 20.0 * scale;
        for i in 0..num_dots {
            let dot_x = start_dot_x + i as f32 * dot_spacing;
            draw_rectangle(dot_x - 1.5, header_y + header_height - 1.5, 3.0, 3.0, FRAME_ACCENT);
        }

        // Title text (native font size for crisp rendering)
        self.draw_text_sharp("INVENTORY", header_x + 8.0, header_y + (header_height + 12.0) / 2.0, 16.0, TEXT_TITLE);

        // Gold display (right side)
        let gold_text = format!("{}g", state.inventory.gold);
        let gold_width = self.measure_text_sharp(&gold_text, 16.0).width;

        // Nugget icon size and spacing
        let icon_size = 12.0 * scale;
        let icon_margin = 4.0 * scale;
        let coin_x = header_x + header_w - 8.0 - gold_width - icon_size - icon_margin;

        // Gold nugget icon
        if let Some(texture) = &self.gold_nugget_texture {
            draw_texture_ex(
                texture,
                coin_x,
                header_y + (header_height - icon_size) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(icon_size, icon_size)),
                    ..Default::default()
                },
            );
        }

        self.draw_text_sharp(&gold_text, coin_x + icon_size + icon_margin, header_y + (header_height + 12.0) / 2.0, 16.0, TEXT_GOLD);

        // Register gold display bounds for right-click context menu
        let gold_bounds = Rect::new(coin_x, header_y, icon_size + icon_margin + gold_width + 8.0, header_height);
        layout.add(UiElementId::GoldDisplay, gold_bounds);

        // ===== INVENTORY GRID =====
        let grid_padding = GRID_PADDING * scale;
        let slot_size = (INV_SLOT_SIZE * scale).max(MIN_SLOT_SIZE); // Ensure icons fit
        let slot_spacing = SLOT_SPACING * scale;
        let content_y = inv_y + frame_thickness + header_height + 10.0 * scale;
        let grid_x = inv_x + grid_padding;
        let grid_y = content_y;
        let slots_per_row = 4;
        let total_rows = 5;
        let row_height = slot_size + slot_spacing;

        // Calculate visible grid area (panel bottom minus grid top, with bottom padding)
        let grid_bottom = inv_y + inv_height - frame_thickness - 4.0 * scale;
        let visible_grid_height = grid_bottom - grid_y;
        let total_grid_height = total_rows as f32 * row_height;
        let needs_scroll = total_grid_height > visible_grid_height + 1.0;

        // Scroll offset (clamped)
        let max_scroll = (total_grid_height - visible_grid_height).max(0.0);
        let scroll_offset = state.ui_state.inventory_scroll_offset.clamp(0.0, max_scroll);

        // Register grid area for scroll input detection
        if needs_scroll {
            let grid_area = Rect::new(inv_x, grid_y, inv_width, visible_grid_height);
            layout.add(UiElementId::InventoryGridArea, grid_area);
        }

        // Set up scissor clipping for the grid area
        // Convert virtual coordinates to physical screen pixels for scissor
        let physical_w = screen_width();
        let physical_h = screen_height();
        let scale_x = physical_w / screen_w;
        let scale_y = physical_h / screen_h;

        if needs_scroll {
            let mut gl = unsafe { get_internal_gl() };
            gl.flush();
            let scissor_x = (grid_x * scale_x) as i32;
            let scissor_y = (grid_y * scale_y) as i32;
            let scissor_w = ((inv_width - grid_padding) * scale_x) as i32;
            let scissor_h = (visible_grid_height * scale_y) as i32;
            gl.quad_gl.scissor(Some((scissor_x, scissor_y, scissor_w, scissor_h)));
        }

        for i in 0..20 {
            let row = i / slots_per_row;
            let col = i % slots_per_row;
            let x = grid_x + col as f32 * (slot_size + slot_spacing);
            let y = grid_y + row as f32 * row_height - scroll_offset;

            // Skip slots fully outside the visible grid area
            if needs_scroll && (y + slot_size < grid_y || y > grid_bottom) {
                continue;
            }

            // Register slot bounds for hit detection (only if visible)
            if !needs_scroll || (y >= grid_y - 1.0 && y + slot_size <= grid_bottom + 1.0) {
                let bounds = Rect::new(x, y, slot_size, slot_size);
                layout.add(UiElementId::InventorySlot(i), bounds);
            }

            // Check if this slot is hovered
            let is_hovered = matches!(hovered, Some(UiElementId::InventorySlot(idx)) if *idx == i);

            // Check if this slot is being dragged
            let is_dragging = matches!(&state.ui_state.drag_state, Some(drag) if matches!(&drag.source, DragSource::Inventory(idx) if *idx == i));

            // Determine slot state
            let slot_state = if is_dragging {
                SlotState::Dragging
            } else if is_hovered {
                SlotState::Hovered
            } else {
                SlotState::Normal
            };

            // Draw the slot with bevel effect
            let has_item = state.inventory.slots[i].is_some();
            self.draw_inventory_slot(x, y, slot_size, has_item, slot_state);

            // Draw item if present (hide if being dragged)
            if let Some(slot) = &state.inventory.slots[i] {
                if !is_dragging {
                    self.draw_item_icon(&slot.item_id, x, y, slot_size, slot_size, state, false);

                    // Quantity badge (bottom-left with shadow, native font size)
                    if slot.quantity > 1 {
                        let qty_text = slot.quantity.to_string();
                        // Shadow
                        self.draw_text_sharp(&qty_text, x + 3.0, y + slot_size - 4.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.8));
                        // Text
                        self.draw_text_sharp(&qty_text, x + 2.0, y + slot_size - 5.0, 16.0, TEXT_NORMAL);
                    }
                }
            }

            // Show slot number badge for first 5 (quick slots)
            if i < 5 {
                let num_text = (i + 1).to_string();
                let text_w = self.measure_text_sharp(&num_text, 16.0).width;
                let badge_w = text_w + 2.0;
                let badge_h = 13.0;
                let num_x = x + slot_size - badge_w - 1.0;
                let num_y = y + 1.0;
                draw_rectangle(num_x, num_y, badge_w, badge_h, Color::new(0.0, 0.0, 0.0, 0.5));
                self.draw_text_sharp(&num_text, num_x + 1.0, num_y + 11.0, 16.0, TEXT_DIM);
            }

            // Shift-drop indicator overlay
            let shift_held = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            if shift_held && state.ui_state.shift_drop_enabled && has_item && is_hovered {
                // Red-tinted overlay
                draw_rectangle(x + 2.0, y + 2.0, slot_size - 4.0, slot_size - 4.0, Color::new(0.8, 0.2, 0.2, 0.35));
                // Red border highlight
                draw_rectangle_lines(x + 1.0, y + 1.0, slot_size - 2.0, slot_size - 2.0, 2.0, Color::new(0.9, 0.3, 0.3, 0.9));
            }
        }

        // Disable scissor clipping
        if needs_scroll {
            let mut gl = unsafe { get_internal_gl() };
            gl.flush();
            gl.quad_gl.scissor(None);

            // Draw scroll indicator - subtle gradient at bottom when more content below
            if scroll_offset < max_scroll - 1.0 {
                let fade_h = 12.0 * scale;
                let fade_y = grid_bottom - fade_h;
                for j in 0..4 {
                    let t = j as f32 / 4.0;
                    let alpha = t * 0.6;
                    draw_rectangle(
                        grid_x, fade_y + t * fade_h,
                        inv_width - grid_padding * 2.0, fade_h / 4.0,
                        Color::new(PANEL_BG_DARK.r, PANEL_BG_DARK.g, PANEL_BG_DARK.b, alpha),
                    );
                }
            }

            // Draw scroll indicator at top when scrolled down
            if scroll_offset > 1.0 {
                let fade_h = 12.0 * scale;
                for j in 0..4 {
                    let t = 1.0 - j as f32 / 4.0;
                    let alpha = t * 0.6;
                    draw_rectangle(
                        grid_x, grid_y + (j as f32) * fade_h / 4.0,
                        inv_width - grid_padding * 2.0, fade_h / 4.0,
                        Color::new(PANEL_BG_DARK.r, PANEL_BG_DARK.g, PANEL_BG_DARK.b, alpha),
                    );
                }
            }

            // Scrollbar track and thumb
            let scrollbar_w: f32 = if cfg!(target_os = "android") { 12.0 } else { 8.0 };
            let track_x = inv_x + inv_width - frame_thickness - scrollbar_w - 2.0;
            let track_y = grid_y;
            let track_h = visible_grid_height;

            layout.add(UiElementId::InventoryScrollbar, Rect::new(track_x, track_y, scrollbar_w, track_h));

            // Track background
            draw_rectangle(track_x, track_y, scrollbar_w, track_h, Color::new(0.1, 0.09, 0.12, 0.6));

            // Thumb
            let thumb_ratio = visible_grid_height / total_grid_height;
            let thumb_h = (track_h * thumb_ratio).max(16.0);
            let scroll_ratio = if max_scroll > 0.0 { scroll_offset / max_scroll } else { 0.0 };
            let thumb_y = track_y + scroll_ratio * (track_h - thumb_h);

            let thumb_color = if state.ui_state.inventory_scrollbar_dragging {
                FRAME_ACCENT
            } else if matches!(hovered, Some(UiElementId::InventoryScrollbar)) {
                FRAME_MID
            } else {
                Color::new(0.3, 0.27, 0.35, 0.8)
            };
            draw_rectangle(track_x + 1.0, thumb_y, scrollbar_w - 2.0, thumb_h, thumb_color);
        }

    }

    pub(crate) fn draw_item_icon(&self, item_id: &str, x: f32, y: f32, slot_width: f32, slot_height: f32, state: &GameState, with_backdrop: bool) {
        // Draw circular stone backdrop if requested
        if with_backdrop {
            if let Some(backdrop) = &self.circular_stone_texture {
                let backdrop_width = backdrop.width();
                let backdrop_height = backdrop.height();
                let backdrop_offset_x = (slot_width - backdrop_width) / 2.0;
                let backdrop_offset_y = (slot_height - backdrop_height) / 2.0;

                draw_texture_ex(
                    backdrop,
                    x + backdrop_offset_x,
                    y + backdrop_offset_y,
                    WHITE,
                    DrawTextureParams::default(),
                );
            }
        }

        if let Some((texture, source_rect)) = self.item_sprites.get(item_id) {
            let (icon_width, icon_height) = if let Some(r) = source_rect {
                (r.w, r.h)
            } else {
                (texture.width(), texture.height())
            };
            let offset_x = (slot_width - icon_width) / 2.0;
            // If drawing with backdrop, bring up Y by 1px
            let y_draw = if with_backdrop { y - 1.0 } else { y };
            let offset_y = (slot_height - icon_height) / 2.0;

            draw_texture_ex(
                texture,
                x + offset_x,
                y_draw + offset_y,
                WHITE,
                DrawTextureParams {
                    source: source_rect,
                    ..Default::default()
                },
            );
        } else {
            let item_def = state.item_registry.get_or_placeholder(item_id);
            let color = item_def.category_color();
            let icon_width = 32.0;
            let icon_height = 32.0;
            let offset_x = (slot_width - icon_width) / 2.0;
            let offset_y = (slot_height - icon_height) / 2.0;
            draw_rectangle(x + offset_x, y + offset_y, icon_width, icon_height, color);
        }
    }

    pub(crate) fn render_dragged_item(&self, drag: &DragState, state: &GameState) {
        let (mx, my) = mouse_position();

        // Get the item texture to determine its size
        if let Some((texture, source_rect)) = self.item_sprites.get(&drag.item_id) {
            let (icon_width, icon_height) = if let Some(r) = source_rect {
                (r.w, r.h)
            } else {
                (texture.width(), texture.height())
            };

            // Center the item on cursor
            let x = mx - icon_width / 2.0;
            let y = my - icon_height / 2.0;

            // Draw just the item sprite, semi-transparent (70% opacity)
            draw_texture_ex(
                texture,
                x,
                y,
                Color::new(1.0, 1.0, 1.0, 0.7),
                DrawTextureParams {
                    source: source_rect,
                    ..Default::default()
                },
            );
        } else {
            // Fallback for items without textures - draw colored placeholder
            let item_def = state.item_registry.get_or_placeholder(&drag.item_id);
            let mut color = item_def.category_color();
            color.a = 0.7; // Semi-transparent
            let icon_size = 32.0;
            let x = mx - icon_size / 2.0;
            let y = my - icon_size / 2.0;
            draw_rectangle(x, y, icon_size, icon_size, color);
        }
    }
}
