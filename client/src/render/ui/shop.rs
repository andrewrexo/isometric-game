//! Shop panel rendering - side-by-side Buy/Sell layout

use macroquad::prelude::*;
use crate::game::GameState;
use crate::ui::{UiElementId, UiLayout, ScrollableListConfig, draw_scrollbar};
use super::super::Renderer;
use super::common::*;

/// Constants for shop list rendering
const SHOP_ITEM_HEIGHT: f32 = 48.0;
const SHOP_ITEM_SPACING: f32 = 4.0;
const SCROLLBAR_WIDTH: f32 = 8.0;
const COLUMN_GAP: f32 = 10.0;
const HEADER_HEIGHT: f32 = 24.0;
const TRANSACTION_HEIGHT: f32 = 80.0;

impl Renderer {
    pub(crate) fn render_shop_tab(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout, panel_x: f32, content_y: f32, content_width: f32, content_height: f32) {
        let shop_data = match &state.ui_state.shop_data {
            Some(data) => data,
            None => {
                self.draw_text_sharp("Loading shop...", panel_x + 20.0, content_y + 40.0, 16.0, TEXT_DIM);
                return;
            }
        };

        // Calculate column dimensions
        let usable_width = content_width - 20.0; // Account for padding
        let column_width = (usable_width - COLUMN_GAP) / 2.0;
        let left_x = panel_x + FRAME_THICKNESS + 10.0;
        let right_x = left_x + column_width + COLUMN_GAP;

        let header_y = content_y + 2.0;
        let list_y = content_y + HEADER_HEIGHT + 10.0;
        let list_height = content_height - HEADER_HEIGHT - TRANSACTION_HEIGHT - 26.0;
        let bar_y = list_y + list_height + 8.0;

        // STEP 1: Render lists FIRST (they may overflow)
        self.render_buy_column(state, hovered, layout, left_x, list_y, column_width, list_height, shop_data);
        self.render_sell_column(state, hovered, layout, right_x, list_y, column_width, list_height, shop_data);

        // STEP 2: Draw headers ON TOP to cover any list overflow at the top
        // BUY header
        draw_rectangle(left_x, header_y, column_width, HEADER_HEIGHT + 8.0, PANEL_BG_MID); // Cover gap too
        draw_rectangle(left_x, header_y, column_width, HEADER_HEIGHT, SLOT_BORDER);
        draw_rectangle(left_x + 1.0, header_y + 1.0, column_width - 2.0, HEADER_HEIGHT - 2.0, PANEL_BG_MID);
        let buy_header_dims = self.measure_text_sharp("BUY", 16.0);
        let buy_header_x = left_x + (column_width - buy_header_dims.width) / 2.0;
        self.draw_text_sharp("BUY", buy_header_x, header_y + 17.0, 16.0, TEXT_TITLE);

        // SELL header
        draw_rectangle(right_x, header_y, column_width, HEADER_HEIGHT + 8.0, PANEL_BG_MID); // Cover gap too
        draw_rectangle(right_x, header_y, column_width, HEADER_HEIGHT, SLOT_BORDER);
        draw_rectangle(right_x + 1.0, header_y + 1.0, column_width - 2.0, HEADER_HEIGHT - 2.0, PANEL_BG_MID);
        let sell_header_dims = self.measure_text_sharp("SELL", 16.0);
        let sell_header_x = right_x + (column_width - sell_header_dims.width) / 2.0;
        self.draw_text_sharp("SELL", sell_header_x, header_y + 17.0, 16.0, TEXT_TITLE);

        // STEP 3: Render transaction bars ON TOP to cover any list overflow at the bottom
        self.render_buy_transaction(state, hovered, layout, left_x, bar_y, column_width, shop_data);
        self.render_sell_transaction(state, hovered, layout, right_x, bar_y, column_width, shop_data);
    }

    fn render_buy_column(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout, x: f32, list_y: f32, width: f32, list_height: f32, shop_data: &crate::game::ShopData) {
        // Calculate scroll state first to know if scrollbar is needed
        let scroll_config = ScrollableListConfig {
            visible_height: list_height,
            item_height: SHOP_ITEM_HEIGHT,
            item_spacing: SHOP_ITEM_SPACING,
            total_items: shop_data.stock.len(),
            scroll_offset: state.ui_state.shop_buy_scroll,
        };
        let scroll_state = scroll_config.calculate();

        // Only reserve scrollbar space if scrolling is needed
        let item_width = if scroll_state.show_scrollbar {
            width - SCROLLBAR_WIDTH - 2.0
        } else {
            width
        };

        // Store the scroll area bounds
        let clip_rect = Rect::new(x, list_y, width, list_height);
        layout.add(UiElementId::ShopBuyScrollArea, clip_rect);

        // Render visible items (overflow will be covered by headers/transaction bars drawn later)
        for i in scroll_state.first_visible..scroll_state.last_visible {
            if let Some(stock_item) = shop_data.stock.get(i) {
                let relative_idx = i - scroll_state.first_visible;
                let item_y = list_y + scroll_state.first_item_offset + (relative_idx as f32) * (SHOP_ITEM_HEIGHT + SHOP_ITEM_SPACING);

                // Skip items completely outside visible area
                if item_y + SHOP_ITEM_HEIGHT < list_y || item_y > list_y + list_height {
                    continue;
                }

                let is_selected = i == state.ui_state.shop_selected_buy_index;
                let bounds = Rect::new(x, item_y, item_width, SHOP_ITEM_HEIGHT);
                layout.add(UiElementId::ShopBuyItem(i), bounds);

                let is_hovered = matches!(hovered, Some(UiElementId::ShopBuyItem(idx)) if *idx == i);

                let (bg_color, border_color) = if is_selected {
                    (SLOT_HOVER_BG, SLOT_SELECTED_BORDER)
                } else if is_hovered {
                    (Color::new(0.141, 0.141, 0.188, 1.0), SLOT_HOVER_BORDER)
                } else {
                    (SLOT_BG_EMPTY, SLOT_BORDER)
                };

                draw_rectangle(x, item_y, item_width, SHOP_ITEM_HEIGHT, border_color);
                draw_rectangle(x + 1.0, item_y + 1.0, item_width - 2.0, SHOP_ITEM_HEIGHT - 2.0, bg_color);

                // Item sprite
                let sprite_x = x + 13.0;
                let sprite_y = item_y + 8.0;
                self.draw_item_icon(&stock_item.item_id, sprite_x, sprite_y, 32.0, 32.0, state, true);

                // Item name
                let name = state.item_registry.get(&stock_item.item_id)
                    .map(|def| def.display_name.as_str())
                    .unwrap_or(&stock_item.item_id);
                self.draw_text_sharp(name, sprite_x + ITEM_TEXT_OFFSET, item_y + 18.0, 16.0, TEXT_NORMAL);

                // Price
                let price_text = format!("{}g", stock_item.price);
                let text_x = sprite_x + ITEM_TEXT_OFFSET;
                let text_y = item_y + 32.0;
                
                // Nugget icon
                let icon_size = 12.0;
                let icon_margin = 4.0;
                if let Some(texture) = &self.gold_nugget_texture {
                    draw_texture_ex(
                        texture,
                        text_x,
                        text_y - 11.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(icon_size, icon_size)),
                            ..Default::default()
                        },
                    );
                }
                
                self.draw_text_sharp(&price_text, text_x + icon_size + icon_margin, text_y, 16.0, TEXT_GOLD);

                // Stock (right side)
                let stock_text = format!("x{}", stock_item.quantity);
                let stock_color = if stock_item.quantity > 0 { TEXT_DIM } else { Color::new(0.8, 0.3, 0.3, 1.0) };
                self.draw_text_sharp(&stock_text, x + item_width - 40.0, item_y + 26.0, 16.0, stock_color);
            }
        }

        // Scrollbar
        if scroll_state.show_scrollbar {
            let scrollbar_x = x + item_width + 2.0;
            draw_scrollbar(
                scrollbar_x,
                list_y,
                SCROLLBAR_WIDTH - 4.0,
                list_height,
                scroll_state.scrollbar_position,
                scroll_state.scrollbar_size,
                Color::new(0.15, 0.15, 0.2, 1.0),
                Color::new(0.4, 0.4, 0.5, 1.0),
            );
        }
    }

    fn render_sell_column(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout, x: f32, list_y: f32, width: f32, list_height: f32, shop_data: &crate::game::ShopData) {
        // Collect inventory items
        let inventory_items: Vec<(usize, &crate::game::InventorySlot)> = state.inventory.slots.iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|s| (i, s)))
            .collect();

        if inventory_items.is_empty() {
            self.draw_text_sharp("No items", x + 10.0, list_y + 20.0, 16.0, TEXT_DIM);
            return;
        }

        // Calculate scroll state first to know if scrollbar is needed
        let scroll_config = ScrollableListConfig {
            visible_height: list_height,
            item_height: SHOP_ITEM_HEIGHT,
            item_spacing: SHOP_ITEM_SPACING,
            total_items: inventory_items.len(),
            scroll_offset: state.ui_state.shop_sell_scroll,
        };
        let scroll_state = scroll_config.calculate();

        // Only reserve scrollbar space if scrolling is needed
        let item_width = if scroll_state.show_scrollbar {
            width - SCROLLBAR_WIDTH - 4.0
        } else {
            width - 1.0
        };

        // Store scroll area bounds
        let clip_rect = Rect::new(x, list_y, width, list_height);
        layout.add(UiElementId::ShopSellScrollArea, clip_rect);

        // Render visible items (overflow will be covered by headers/transaction bars drawn later)
        for i in scroll_state.first_visible..scroll_state.last_visible {
            if let Some((_slot_idx, inv_slot)) = inventory_items.get(i) {
                let relative_idx = i - scroll_state.first_visible;
                let item_y = list_y + scroll_state.first_item_offset + (relative_idx as f32) * (SHOP_ITEM_HEIGHT + SHOP_ITEM_SPACING);

                // Skip items completely outside visible area
                if item_y + SHOP_ITEM_HEIGHT < list_y || item_y > list_y + list_height {
                    continue;
                }

                let is_selected = i == state.ui_state.shop_selected_sell_index;
                let bounds = Rect::new(x, item_y, item_width, SHOP_ITEM_HEIGHT);
                layout.add(UiElementId::ShopSellItem(i), bounds);

                let is_hovered = matches!(hovered, Some(UiElementId::ShopSellItem(idx)) if *idx == i);

                let (bg_color, border_color) = if is_selected {
                    (SLOT_HOVER_BG, SLOT_SELECTED_BORDER)
                } else if is_hovered {
                    (Color::new(0.141, 0.141, 0.188, 1.0), SLOT_HOVER_BORDER)
                } else {
                    (SLOT_BG_EMPTY, SLOT_BORDER)
                };

                draw_rectangle(x, item_y, item_width, SHOP_ITEM_HEIGHT, border_color);
                draw_rectangle(x + 1.0, item_y + 1.0, item_width - 2.0, SHOP_ITEM_HEIGHT - 2.0, bg_color);

                // Item sprite
                let sprite_x = x + 13.0;
                let sprite_y = item_y + 8.0;
                self.draw_item_icon(&inv_slot.item_id, sprite_x, sprite_y, 32.0, 32.0, state, true);

                // Item name
                let name = state.item_registry.get(&inv_slot.item_id)
                    .map(|def| def.display_name.as_str())
                    .unwrap_or(&inv_slot.item_id);
                self.draw_text_sharp(name, sprite_x + ITEM_TEXT_OFFSET, item_y + 18.0, 16.0, TEXT_NORMAL);

                // Quantity owned
                let qty_text = format!("x{}", inv_slot.quantity);
                self.draw_text_sharp(&qty_text, sprite_x + ITEM_TEXT_OFFSET, item_y + 34.0, 16.0, TEXT_DIM);

                // Sell price (right side)
                if let Some(item_def) = state.item_registry.get(&inv_slot.item_id) {
                    if item_def.sellable {
                        let sell_price = (item_def.base_price as f32 * shop_data.buy_multiplier) as i32;
                        let price_text = format!("{}g", sell_price);
                        let price_width = self.measure_text_sharp(&price_text, 16.0).width;
                        
                        let icon_size = 12.0;
                        let icon_margin = 4.0;
                        let total_width = icon_size + icon_margin + price_width;
                        let price_x = x + item_width - total_width - 8.0;
                        let price_y = item_y + 24.0;

                        if let Some(texture) = &self.gold_nugget_texture {
                            draw_texture_ex(
                                texture,
                                price_x,
                                price_y - 11.0,
                                WHITE,
                                DrawTextureParams {
                                    dest_size: Some(vec2(icon_size, icon_size)),
                                    ..Default::default()
                                },
                            );
                        }
                        self.draw_text_sharp(&price_text, price_x + icon_size + icon_margin, price_y, 16.0, TEXT_GOLD);
                    } else {
                        self.draw_text_sharp("---", x + item_width - 40.0, item_y + 24.0, 16.0, Color::new(0.5, 0.5, 0.5, 1.0));
                    }
                }
            }
        }

        // Scrollbar
        if scroll_state.show_scrollbar {
            let scrollbar_x = x + item_width + 2.0;
            draw_scrollbar(
                scrollbar_x,
                list_y,
                SCROLLBAR_WIDTH - 4.0,
                list_height,
                scroll_state.scrollbar_position,
                scroll_state.scrollbar_size,
                Color::new(0.15, 0.15, 0.2, 1.0),
                Color::new(0.4, 0.4, 0.5, 1.0),
            );
        }
    }

    fn render_buy_transaction(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout, x: f32, bar_y: f32, width: f32, shop_data: &crate::game::ShopData) {
        // Background
        draw_rectangle(x, bar_y, width, TRANSACTION_HEIGHT, SLOT_BORDER);
        draw_rectangle(x + 1.0, bar_y + 1.0, width - 2.0, TRANSACTION_HEIGHT - 2.0, PANEL_BG_MID);

        // Get selected item info
        let (item_name, total_price, can_buy) = if let Some(stock_item) = shop_data.stock.get(state.ui_state.shop_selected_buy_index) {
            let name = state.item_registry.get(&stock_item.item_id)
                .map(|def| def.display_name.as_str())
                .unwrap_or(&stock_item.item_id);
            let total = stock_item.price * state.ui_state.shop_buy_quantity;
            let can_afford = state.inventory.gold >= total;
            let in_stock = stock_item.quantity >= state.ui_state.shop_buy_quantity;
            (name, total, can_afford && in_stock)
        } else {
            ("", 0, false)
        };

        // Item name
        self.draw_text_sharp(item_name, x + 11.0, bar_y + 16.0, 16.0, TEXT_NORMAL);

        // Quantity controls
        let qty_y = bar_y + 38.0;
        let qty_label = "Qty:";
        let qty_dims = self.measure_text_sharp(qty_label, 16.0);
        self.draw_text_sharp(qty_label, x + 11.0, qty_y, 16.0, TEXT_DIM);

        // Minus button
        let btn_size = 18.0;
        let minus_x = x + 11.0 + qty_dims.width + 12.0;
        let btn_y = qty_y - 13.0;
        let minus_bounds = Rect::new(minus_x, btn_y, btn_size, btn_size);
        layout.add(UiElementId::ShopBuyQuantityMinus, minus_bounds);
        let minus_hovered = matches!(hovered, Some(UiElementId::ShopBuyQuantityMinus));
        let minus_bg = if minus_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
        draw_rectangle(minus_x, btn_y, btn_size, btn_size, SLOT_BORDER);
        draw_rectangle(minus_x + 1.0, btn_y + 1.0, btn_size - 2.0, btn_size - 2.0, minus_bg);
        
        let minus_text_dims = self.measure_text_sharp("-", 16.0);
        self.draw_text_sharp("-", minus_x + (btn_size - minus_text_dims.width) / 2.0, qty_y - 1.0, 16.0, TEXT_NORMAL);

        // Quantity display
        let qty_text = format!("{}", state.ui_state.shop_buy_quantity);
        let qty_val_dims = self.measure_text_sharp(&qty_text, 16.0);
        let qty_val_x = minus_x + btn_size + 8.0;
        self.draw_text_sharp(&qty_text, qty_val_x, qty_y, 16.0, TEXT_TITLE);

        // Plus button
        let plus_x = qty_val_x + qty_val_dims.width + 8.0;
        let plus_bounds = Rect::new(plus_x, btn_y, btn_size, btn_size);
        layout.add(UiElementId::ShopBuyQuantityPlus, plus_bounds);
        let plus_hovered = matches!(hovered, Some(UiElementId::ShopBuyQuantityPlus));
        let plus_bg = if plus_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
        draw_rectangle(plus_x, btn_y, btn_size, btn_size, SLOT_BORDER);
        draw_rectangle(plus_x + 1.0, btn_y + 1.0, btn_size - 2.0, btn_size - 2.0, plus_bg);
        
        let plus_text_dims = self.measure_text_sharp("+", 16.0);
        self.draw_text_sharp("+", plus_x + (btn_size - plus_text_dims.width) / 2.0, qty_y - 1.0, 16.0, TEXT_NORMAL);

        // Total and Buy button row
        let total_y = bar_y + 60.0;
        let total_label = "Total:";
        let total_dims = self.measure_text_sharp(total_label, 16.0);
        self.draw_text_sharp(total_label, x + 11.0, total_y - 2.0, 16.0, TEXT_DIM);
        
        let price_text = format!("{}g", total_price);
        let price_color = if can_buy { TEXT_GOLD } else { Color::new(0.8, 0.3, 0.3, 1.0) };
        let price_x = x + 11.0 + total_dims.width + 8.0;
        
        // Nugget icon
        let icon_size = 12.0;
        let icon_margin = 4.0;
        if let Some(texture) = &self.gold_nugget_texture {
            draw_texture_ex(
                texture,
                price_x,
                total_y - 13.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(icon_size, icon_size)),
                    ..Default::default()
                },
            );
        }
        
        self.draw_text_sharp(&price_text, price_x + icon_size + icon_margin, total_y - 2.0, 16.0, price_color);

        // Buy button
        let button_w = 60.0;
        let button_h = 28.0;
        let button_x = x + width - button_w - 8.0;
        let button_y = bar_y + TRANSACTION_HEIGHT - button_h - 8.0;
        let button_bounds = Rect::new(button_x, button_y, button_w, button_h);
        layout.add(UiElementId::ShopBuyConfirmButton, button_bounds);

        let button_hovered = matches!(hovered, Some(UiElementId::ShopBuyConfirmButton));
        let (button_bg, button_border) = if !can_buy {
            (Color::new(0.1, 0.1, 0.1, 1.0), Color::new(0.3, 0.3, 0.3, 1.0))
        } else if button_hovered {
            (Color::new(0.2, 0.5, 0.2, 1.0), Color::new(0.3, 0.7, 0.3, 1.0))
        } else {
            (Color::new(0.15, 0.4, 0.15, 1.0), Color::new(0.25, 0.6, 0.25, 1.0))
        };

        draw_rectangle(button_x, button_y, button_w, button_h, button_border);
        draw_rectangle(button_x + 2.0, button_y + 2.0, button_w - 4.0, button_h - 4.0, button_bg);

        let button_text_color = if can_buy { WHITE } else { TEXT_DIM };
        let btn_dims = self.measure_text_sharp("Buy", 16.0);
        let btn_text_x = button_x + (button_w - btn_dims.width) / 2.0;
        self.draw_text_sharp("Buy", btn_text_x, button_y + 19.0, 16.0, button_text_color);
    }

    fn render_sell_transaction(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout, x: f32, bar_y: f32, width: f32, shop_data: &crate::game::ShopData) {
        // Background
        draw_rectangle(x, bar_y, width, TRANSACTION_HEIGHT, SLOT_BORDER);
        draw_rectangle(x + 1.0, bar_y + 1.0, width - 2.0, TRANSACTION_HEIGHT - 2.0, PANEL_BG_MID);

        // Get selected item info
        let inventory_items: Vec<_> = state.inventory.slots.iter()
            .filter_map(|slot| slot.as_ref())
            .collect();

        let (item_name, total_price, can_sell) = if let Some(inv_slot) = inventory_items.get(state.ui_state.shop_selected_sell_index) {
            let name = state.item_registry.get(&inv_slot.item_id)
                .map(|def| def.display_name.as_str())
                .unwrap_or(&inv_slot.item_id);
            if let Some(item_def) = state.item_registry.get(&inv_slot.item_id) {
                if item_def.sellable {
                    let sell_price = (item_def.base_price as f32 * shop_data.buy_multiplier) as i32;
                    let total = sell_price * state.ui_state.shop_sell_quantity;
                    let has_quantity = inv_slot.quantity >= state.ui_state.shop_sell_quantity;
                    (name, total, has_quantity)
                } else {
                    (name, 0, false)
                }
            } else {
                (name, 0, false)
            }
        } else {
            ("", 0, false)
        };

        // Item name
        self.draw_text_sharp(item_name, x + 11.0, bar_y + 16.0, 16.0, TEXT_NORMAL);

        // Quantity controls
        let qty_y = bar_y + 38.0;
        let qty_label = "Qty:";
        let qty_dims = self.measure_text_sharp(qty_label, 16.0);
        self.draw_text_sharp(qty_label, x + 11.0, qty_y, 16.0, TEXT_DIM);

        // Minus button
        let btn_size = 18.0;
        let minus_x = x + 11.0 + qty_dims.width + 12.0;
        let btn_y = qty_y - 13.0;
        let minus_bounds = Rect::new(minus_x, btn_y, btn_size, btn_size);
        layout.add(UiElementId::ShopSellQuantityMinus, minus_bounds);
        let minus_hovered = matches!(hovered, Some(UiElementId::ShopSellQuantityMinus));
        let minus_bg = if minus_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
        draw_rectangle(minus_x, btn_y, btn_size, btn_size, SLOT_BORDER);
        draw_rectangle(minus_x + 1.0, btn_y + 1.0, btn_size - 2.0, btn_size - 2.0, minus_bg);
        
        let minus_text_dims = self.measure_text_sharp("-", 16.0);
        self.draw_text_sharp("-", minus_x + (btn_size - minus_text_dims.width) / 2.0, qty_y - 1.0, 16.0, TEXT_NORMAL);

        // Quantity display
        let qty_text = format!("{}", state.ui_state.shop_sell_quantity);
        let qty_val_dims = self.measure_text_sharp(&qty_text, 16.0);
        let qty_val_x = minus_x + btn_size + 8.0;
        self.draw_text_sharp(&qty_text, qty_val_x, qty_y, 16.0, TEXT_TITLE);

        // Plus button
        let plus_x = qty_val_x + qty_val_dims.width + 8.0;
        let plus_bounds = Rect::new(plus_x, btn_y, btn_size, btn_size);
        layout.add(UiElementId::ShopSellQuantityPlus, plus_bounds);
        let plus_hovered = matches!(hovered, Some(UiElementId::ShopSellQuantityPlus));
        let plus_bg = if plus_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
        draw_rectangle(plus_x, btn_y, btn_size, btn_size, SLOT_BORDER);
        draw_rectangle(plus_x + 1.0, btn_y + 1.0, btn_size - 2.0, btn_size - 2.0, plus_bg);
        
        let plus_text_dims = self.measure_text_sharp("+", 16.0);
        self.draw_text_sharp("+", plus_x + (btn_size - plus_text_dims.width) / 2.0, qty_y - 1.0, 16.0, TEXT_NORMAL);

        // Total and Sell button row
        let total_y = bar_y + 60.0;
        let total_label = "You get:";
        let total_dims = self.measure_text_sharp(total_label, 16.0);
        self.draw_text_sharp(total_label, x + 11.0, total_y - 2.0, 16.0, TEXT_DIM);
        
        let price_text = format!("{}g", total_price);
        let price_color = if can_sell { TEXT_GOLD } else { Color::new(0.8, 0.3, 0.3, 1.0) };
        let price_x = x + 11.0 + total_dims.width + 8.0;
        
        // Nugget icon
        let icon_size = 12.0;
        let icon_margin = 4.0;
        if let Some(texture) = &self.gold_nugget_texture {
            draw_texture_ex(
                texture,
                price_x,
                total_y - 13.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(icon_size, icon_size)),
                    ..Default::default()
                },
            );
        }
        
        self.draw_text_sharp(&price_text, price_x + icon_size + icon_margin, total_y - 2.0, 16.0, price_color);

        // Sell button
        let button_w = 60.0;
        let button_h = 28.0;
        let button_x = x + width - button_w - 8.0;
        let button_y = bar_y + TRANSACTION_HEIGHT - button_h - 8.0;
        let button_bounds = Rect::new(button_x, button_y, button_w, button_h);
        layout.add(UiElementId::ShopSellConfirmButton, button_bounds);

        let button_hovered = matches!(hovered, Some(UiElementId::ShopSellConfirmButton));
        let (button_bg, button_border) = if !can_sell {
            (Color::new(0.1, 0.1, 0.1, 1.0), Color::new(0.3, 0.3, 0.3, 1.0))
        } else if button_hovered {
            (Color::new(0.95, 0.35, 0.35, 1.0), Color::new(1.0, 0.5, 0.5, 1.0))
        } else {
            (Color::new(0.8, 0.15, 0.15, 1.0), Color::new(0.9, 0.25, 0.25, 1.0))
        };

        draw_rectangle(button_x, button_y, button_w, button_h, button_border);
        draw_rectangle(button_x + 2.0, button_y + 2.0, button_w - 4.0, button_h - 4.0, button_bg);

        let button_text_color = if can_sell { WHITE } else { TEXT_DIM };
        let btn_dims = self.measure_text_sharp("Sell", 16.0);
        let btn_text_x = button_x + (button_w - btn_dims.width) / 2.0;
        self.draw_text_sharp("Sell", btn_text_x, button_y + 19.0, 16.0, button_text_color);
    }
}
