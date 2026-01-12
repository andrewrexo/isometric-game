//! Shop panel rendering

use macroquad::prelude::*;
use crate::game::{GameState, ShopSubTab};
use crate::ui::{UiElementId, UiLayout, ScrollableListConfig, draw_scrollbar};
use super::super::Renderer;
use super::common::*;

/// Constants for shop list rendering
const SHOP_ITEM_HEIGHT: f32 = 48.0;
const SHOP_ITEM_SPACING: f32 = 4.0;
const SCROLLBAR_WIDTH: f32 = 8.0;

impl Renderer {
    pub(crate) fn render_shop_tab(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout, panel_x: f32, content_y: f32, content_width: f32, content_height: f32) {
        let shop_data = match &state.ui_state.shop_data {
            Some(data) => data,
            None => {
                self.draw_text_sharp("Loading shop...", panel_x + 20.0, content_y + 40.0, 16.0, TEXT_DIM);
                return;
            }
        };

        // ===== SUB-TABS: Buy / Sell =====
        let tab_y = content_y;
        let tab_height = 28.0;
        let tab_width = 100.0;
        let mut tab_x = panel_x + FRAME_THICKNESS + 10.0;

        // Buy Tab
        let is_buy_selected = state.ui_state.shop_sub_tab == ShopSubTab::Buy;
        let buy_bounds = Rect::new(tab_x, tab_y, tab_width, tab_height);
        layout.add(UiElementId::ShopSubTab(0), buy_bounds);

        let is_buy_hovered = matches!(hovered, Some(UiElementId::ShopSubTab(0)));
        let (buy_bg, buy_border) = if is_buy_selected {
            (SLOT_HOVER_BG, SLOT_SELECTED_BORDER)
        } else if is_buy_hovered {
            (Color::new(0.141, 0.141, 0.188, 1.0), SLOT_HOVER_BORDER)
        } else {
            (SLOT_BG_EMPTY, SLOT_BORDER)
        };

        draw_rectangle(tab_x, tab_y, tab_width, tab_height, buy_border);
        draw_rectangle(tab_x + 1.0, tab_y + 1.0, tab_width - 2.0, tab_height - 2.0, buy_bg);

        let buy_text_color = if is_buy_selected { TEXT_TITLE } else if is_buy_hovered { TEXT_NORMAL } else { TEXT_DIM };
        self.draw_text_sharp("Buy", tab_x + 35.0, tab_y + 19.0, 16.0, buy_text_color);

        tab_x += tab_width + 4.0;

        // Sell Tab
        let is_sell_selected = state.ui_state.shop_sub_tab == ShopSubTab::Sell;
        let sell_bounds = Rect::new(tab_x, tab_y, tab_width, tab_height);
        layout.add(UiElementId::ShopSubTab(1), sell_bounds);

        let is_sell_hovered = matches!(hovered, Some(UiElementId::ShopSubTab(1)));
        let (sell_bg, sell_border) = if is_sell_selected {
            (SLOT_HOVER_BG, SLOT_SELECTED_BORDER)
        } else if is_sell_hovered {
            (Color::new(0.141, 0.141, 0.188, 1.0), SLOT_HOVER_BORDER)
        } else {
            (SLOT_BG_EMPTY, SLOT_BORDER)
        };

        draw_rectangle(tab_x, tab_y, tab_width, tab_height, sell_border);
        draw_rectangle(tab_x + 1.0, tab_y + 1.0, tab_width - 2.0, tab_height - 2.0, sell_bg);

        let sell_text_color = if is_sell_selected { TEXT_TITLE } else if is_sell_hovered { TEXT_NORMAL } else { TEXT_DIM };
        self.draw_text_sharp("Sell", tab_x + 32.0, tab_y + 19.0, 16.0, sell_text_color);

        // ===== CONTENT AREA =====
        let list_y = content_y + tab_height + 8.0;
        let list_height = content_height - tab_height - 120.0; // Leave room for transaction bar

        match state.ui_state.shop_sub_tab {
            ShopSubTab::Buy => {
                self.render_shop_buy_list(state, hovered, layout, panel_x, list_y, content_width, list_height, shop_data);
            }
            ShopSubTab::Sell => {
                self.render_shop_sell_list(state, hovered, layout, panel_x, list_y, content_width, list_height);
            }
        }

        // ===== TRANSACTION BAR =====
        let bar_y = list_y + list_height + 8.0;
        self.render_transaction_bar(state, hovered, layout, panel_x, bar_y, content_width, shop_data);
    }

    fn render_shop_buy_list(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout, panel_x: f32, list_y: f32, list_width: f32, list_height: f32, shop_data: &crate::game::ShopData) {
        let content_x = panel_x + FRAME_THICKNESS + 10.0;
        let item_width = list_width - 40.0 - SCROLLBAR_WIDTH;

        // Calculate scroll state
        let scroll_config = ScrollableListConfig {
            visible_height: list_height,
            item_height: SHOP_ITEM_HEIGHT,
            item_spacing: SHOP_ITEM_SPACING,
            total_items: shop_data.stock.len(),
            scroll_offset: state.ui_state.shop_buy_scroll,
        };
        let scroll_state = scroll_config.calculate();

        // Set up clipping rectangle
        let clip_rect = Rect::new(content_x, list_y, item_width + SCROLLBAR_WIDTH, list_height);

        // Store the scroll area bounds for mouse wheel handling
        layout.add(UiElementId::ShopBuyScrollArea, clip_rect);

        // Render only visible items
        for i in scroll_state.first_visible..scroll_state.last_visible {
            if let Some(stock_item) = shop_data.stock.get(i) {
                let relative_idx = i - scroll_state.first_visible;
                let item_y = list_y + scroll_state.first_item_offset + (relative_idx as f32) * (SHOP_ITEM_HEIGHT + SHOP_ITEM_SPACING);

                // Skip if completely outside visible area (for partial items at edges)
                if item_y + SHOP_ITEM_HEIGHT < list_y || item_y > list_y + list_height {
                    continue;
                }

                let is_selected = i == state.ui_state.shop_selected_buy_index;
                let bounds = Rect::new(content_x, item_y, item_width, SHOP_ITEM_HEIGHT);
                layout.add(UiElementId::ShopBuyItem(i), bounds);

                let is_hovered = matches!(hovered, Some(UiElementId::ShopBuyItem(idx)) if *idx == i);

                let (bg_color, border_color) = if is_selected {
                    (SLOT_HOVER_BG, SLOT_SELECTED_BORDER)
                } else if is_hovered {
                    (Color::new(0.141, 0.141, 0.188, 1.0), SLOT_HOVER_BORDER)
                } else {
                    (SLOT_BG_EMPTY, SLOT_BORDER)
                };

                draw_rectangle(content_x, item_y, item_width, SHOP_ITEM_HEIGHT, border_color);
                draw_rectangle(content_x + 1.0, item_y + 1.0, item_width - 2.0, SHOP_ITEM_HEIGHT - 2.0, bg_color);

                // Item sprite (32x32)
                let sprite_x = content_x + 8.0;
                let sprite_y = item_y + 8.0;
                self.draw_item_icon(&stock_item.item_id, sprite_x, sprite_y, 32.0, 32.0, state);

                // Item name
                let name = state.item_registry.get(&stock_item.item_id)
                    .map(|def| def.display_name.as_str())
                    .unwrap_or(&stock_item.item_id);
                self.draw_text_sharp(name, sprite_x + 40.0, item_y + 18.0, 16.0, TEXT_NORMAL);

                // Buy price
                let price_text = format!("{} gold", stock_item.price);
                self.draw_text_sharp(&price_text, sprite_x + 40.0, item_y + 34.0, 16.0, TEXT_GOLD);

                // Stock count
                let stock_text = format!("Stock: {}", stock_item.quantity);
                let stock_color = if stock_item.quantity > 0 { TEXT_DIM } else { Color::new(0.8, 0.3, 0.3, 1.0) };
                self.draw_text_sharp(&stock_text, content_x + item_width - 100.0, item_y + 26.0, 16.0, stock_color);
            }
        }

        // Draw scrollbar if needed
        if scroll_state.show_scrollbar {
            let scrollbar_x = content_x + item_width + 4.0;
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

    fn render_shop_sell_list(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout, panel_x: f32, list_y: f32, list_width: f32, list_height: f32) {
        let content_x = panel_x + FRAME_THICKNESS + 10.0;
        let item_width = list_width - 40.0 - SCROLLBAR_WIDTH;

        // Collect all items in inventory
        let inventory_items: Vec<(usize, &crate::game::InventorySlot)> = state.inventory.slots.iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|s| (i, s)))
            .collect();

        if inventory_items.is_empty() {
            self.draw_text_sharp("No items to sell", content_x + 10.0, list_y + 20.0, 16.0, TEXT_DIM);
            return;
        }

        // Calculate scroll state
        let scroll_config = ScrollableListConfig {
            visible_height: list_height,
            item_height: SHOP_ITEM_HEIGHT,
            item_spacing: SHOP_ITEM_SPACING,
            total_items: inventory_items.len(),
            scroll_offset: state.ui_state.shop_sell_scroll,
        };
        let scroll_state = scroll_config.calculate();

        // Store the scroll area bounds for mouse wheel handling
        let clip_rect = Rect::new(content_x, list_y, item_width + SCROLLBAR_WIDTH, list_height);
        layout.add(UiElementId::ShopSellScrollArea, clip_rect);

        // Render only visible items
        for i in scroll_state.first_visible..scroll_state.last_visible {
            if let Some((_slot_idx, inv_slot)) = inventory_items.get(i) {
                let relative_idx = i - scroll_state.first_visible;
                let item_y = list_y + scroll_state.first_item_offset + (relative_idx as f32) * (SHOP_ITEM_HEIGHT + SHOP_ITEM_SPACING);

                // Skip if completely outside visible area
                if item_y + SHOP_ITEM_HEIGHT < list_y || item_y > list_y + list_height {
                    continue;
                }

                let is_selected = i == state.ui_state.shop_selected_sell_index;
                let bounds = Rect::new(content_x, item_y, item_width, SHOP_ITEM_HEIGHT);
                layout.add(UiElementId::ShopSellItem(i), bounds);

                let is_hovered = matches!(hovered, Some(UiElementId::ShopSellItem(idx)) if *idx == i);

                let (bg_color, border_color) = if is_selected {
                    (SLOT_HOVER_BG, SLOT_SELECTED_BORDER)
                } else if is_hovered {
                    (Color::new(0.141, 0.141, 0.188, 1.0), SLOT_HOVER_BORDER)
                } else {
                    (SLOT_BG_EMPTY, SLOT_BORDER)
                };

                draw_rectangle(content_x, item_y, item_width, SHOP_ITEM_HEIGHT, border_color);
                draw_rectangle(content_x + 1.0, item_y + 1.0, item_width - 2.0, SHOP_ITEM_HEIGHT - 2.0, bg_color);

                // Item sprite (32x32)
                let sprite_x = content_x + 8.0;
                let sprite_y = item_y + 8.0;
                self.draw_item_icon(&inv_slot.item_id, sprite_x, sprite_y, 32.0, 32.0, state);

                // Item name
                let name = state.item_registry.get(&inv_slot.item_id)
                    .map(|def| def.display_name.as_str())
                    .unwrap_or(&inv_slot.item_id);
                self.draw_text_sharp(name, sprite_x + 40.0, item_y + 18.0, 16.0, TEXT_NORMAL);

                // Quantity
                let qty_text = format!("x{}", inv_slot.quantity);
                self.draw_text_sharp(&qty_text, sprite_x + 40.0, item_y + 34.0, 16.0, TEXT_DIM);

                // Sell price (calculated from base price)
                if let Some(shop_data) = &state.ui_state.shop_data {
                    if let Some(item_def) = state.item_registry.get(&inv_slot.item_id) {
                        if item_def.sellable {
                            let sell_price = (item_def.base_price as f32 * shop_data.buy_multiplier) as i32;
                            let price_text = format!("{} gold", sell_price);
                            self.draw_text_sharp(&price_text, content_x + item_width - 100.0, item_y + 26.0, 16.0, TEXT_GOLD);
                        } else {
                            self.draw_text_sharp("Cannot sell", content_x + item_width - 100.0, item_y + 26.0, 16.0, Color::new(0.8, 0.3, 0.3, 1.0));
                        }
                    } else {
                        self.draw_text_sharp("Cannot sell", content_x + item_width - 100.0, item_y + 26.0, 16.0, Color::new(0.8, 0.3, 0.3, 1.0));
                    }
                }
            }
        }

        // Draw scrollbar if needed
        if scroll_state.show_scrollbar {
            let scrollbar_x = content_x + item_width + 4.0;
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

    fn render_transaction_bar(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout, panel_x: f32, bar_y: f32, bar_width: f32, shop_data: &crate::game::ShopData) {
        let bar_x = panel_x + FRAME_THICKNESS + 10.0;
        let bar_height = 80.0;

        // Background
        draw_rectangle(bar_x, bar_y, bar_width - 40.0, bar_height, SLOT_BORDER);
        draw_rectangle(bar_x + 1.0, bar_y + 1.0, bar_width - 42.0, bar_height - 2.0, PANEL_BG_MID);

        // Get selected item info
        let (item_name, price, can_transact) = match state.ui_state.shop_sub_tab {
            ShopSubTab::Buy => {
                if let Some(stock_item) = shop_data.stock.get(state.ui_state.shop_selected_buy_index) {
                    let name = state.item_registry.get(&stock_item.item_id)
                        .map(|def| def.display_name.as_str())
                        .unwrap_or(&stock_item.item_id);
                    let total = stock_item.price * state.ui_state.shop_transaction_quantity;
                    let can_afford = state.inventory.gold >= total;
                    let in_stock = stock_item.quantity >= state.ui_state.shop_transaction_quantity;
                    (name, total, can_afford && in_stock)
                } else {
                    ("", 0, false)
                }
            }
            ShopSubTab::Sell => {
                // Find the selected item in inventory
                let inventory_items: Vec<_> = state.inventory.slots.iter()
                    .filter_map(|slot| slot.as_ref())
                    .collect();
                if let Some(inv_slot) = inventory_items.get(state.ui_state.shop_selected_sell_index) {
                    let name = state.item_registry.get(&inv_slot.item_id)
                        .map(|def| def.display_name.as_str())
                        .unwrap_or(&inv_slot.item_id);
                    if let Some(item_def) = state.item_registry.get(&inv_slot.item_id) {
                        if item_def.sellable {
                            let sell_price = (item_def.base_price as f32 * shop_data.buy_multiplier) as i32;
                            let total = sell_price * state.ui_state.shop_transaction_quantity;
                            let has_quantity = inv_slot.quantity >= state.ui_state.shop_transaction_quantity;
                            (name, total, has_quantity)
                        } else {
                            (name, 0, false)
                        }
                    } else {
                        (name, 0, false)
                    }
                } else {
                    ("", 0, false)
                }
            }
        };

        // Item name
        self.draw_text_sharp(item_name, bar_x + 10.0, bar_y + 20.0, 16.0, TEXT_NORMAL);

        // Quantity controls
        let qty_y = bar_y + 35.0;
        self.draw_text_sharp("Quantity:", bar_x + 10.0, qty_y, 14.0, TEXT_DIM);

        // Minus button
        let minus_x = bar_x + 90.0;
        let minus_bounds = Rect::new(minus_x, qty_y - 12.0, 24.0, 24.0);
        layout.add(UiElementId::ShopQuantityMinus, minus_bounds);
        let minus_hovered = matches!(hovered, Some(UiElementId::ShopQuantityMinus));
        let minus_bg = if minus_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
        draw_rectangle(minus_x, qty_y - 12.0, 24.0, 24.0, SLOT_BORDER);
        draw_rectangle(minus_x + 1.0, qty_y - 11.0, 22.0, 22.0, minus_bg);
        self.draw_text_sharp("-", minus_x + 8.0, qty_y + 4.0, 16.0, TEXT_NORMAL);

        // Quantity display
        let qty_text = format!("{}", state.ui_state.shop_transaction_quantity);
        self.draw_text_sharp(&qty_text, minus_x + 32.0, qty_y, 16.0, TEXT_TITLE);

        // Plus button
        let plus_x = minus_x + 64.0;
        let plus_bounds = Rect::new(plus_x, qty_y - 12.0, 24.0, 24.0);
        layout.add(UiElementId::ShopQuantityPlus, plus_bounds);
        let plus_hovered = matches!(hovered, Some(UiElementId::ShopQuantityPlus));
        let plus_bg = if plus_hovered { SLOT_HOVER_BG } else { SLOT_BG_EMPTY };
        draw_rectangle(plus_x, qty_y - 12.0, 24.0, 24.0, SLOT_BORDER);
        draw_rectangle(plus_x + 1.0, qty_y - 11.0, 22.0, 22.0, plus_bg);
        self.draw_text_sharp("+", plus_x + 7.0, qty_y + 4.0, 16.0, TEXT_NORMAL);

        // Total price
        let total_y = bar_y + 55.0;
        let action_text = if state.ui_state.shop_sub_tab == ShopSubTab::Buy { "Total:" } else { "You get:" };
        self.draw_text_sharp(action_text, bar_x + 10.0, total_y, 14.0, TEXT_DIM);
        let price_text = format!("{} gold", price);
        let price_color = if can_transact { TEXT_GOLD } else { Color::new(0.8, 0.3, 0.3, 1.0) };
        self.draw_text_sharp(&price_text, bar_x + 90.0, total_y, 16.0, price_color);

        // Confirm button
        let button_x = bar_x + bar_width - 200.0;
        let button_y = bar_y + 20.0;
        let button_w = 120.0;
        let button_h = 36.0;
        let button_bounds = Rect::new(button_x, button_y, button_w, button_h);
        layout.add(UiElementId::ShopConfirmButton, button_bounds);

        let button_hovered = matches!(hovered, Some(UiElementId::ShopConfirmButton));
        let (button_bg, button_border) = if !can_transact {
            (Color::new(0.1, 0.1, 0.1, 1.0), Color::new(0.3, 0.3, 0.3, 1.0))
        } else if button_hovered {
            (Color::new(0.2, 0.5, 0.2, 1.0), Color::new(0.3, 0.7, 0.3, 1.0))
        } else {
            (Color::new(0.15, 0.4, 0.15, 1.0), Color::new(0.25, 0.6, 0.25, 1.0))
        };

        draw_rectangle(button_x, button_y, button_w, button_h, button_border);
        draw_rectangle(button_x + 2.0, button_y + 2.0, button_w - 4.0, button_h - 4.0, button_bg);

        let button_text = if state.ui_state.shop_sub_tab == ShopSubTab::Buy { "Buy" } else { "Sell" };
        let button_text_color = if can_transact { TEXT_TITLE } else { TEXT_DIM };
        self.draw_text_sharp(button_text, button_x + 40.0, button_y + 24.0, 16.0, button_text_color);
    }
}
