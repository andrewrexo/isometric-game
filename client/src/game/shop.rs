//! Client-side shop data structures

use serde::{Deserialize, Serialize};

/// Shop data received from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopData {
    pub shop_id: String,
    pub display_name: String,
    pub buy_multiplier: f32,
    pub sell_multiplier: f32,
    pub stock: Vec<ShopStockItem>,
}

/// A single item in the shop's stock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopStockItem {
    pub item_id: String,
    pub quantity: i32,
    pub price: i32,
}

/// Sub-tab selection for shop UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopSubTab {
    Buy,
    Sell,
}
