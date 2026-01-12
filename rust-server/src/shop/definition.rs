//! Shop Definition Structures
//!
//! Defines data structures for shop definitions and stock management.

use serde::{Deserialize, Serialize};

/// A shop definition with stock management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopDefinition {
    pub id: String,
    pub display_name: String,
    pub stock: Vec<ShopStockItem>,
}

/// An item stocked in a shop with quantity and restock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopStockItem {
    pub item_id: String,
    pub max_quantity: i32,
    pub restock_rate: i32,
    #[serde(skip)]
    pub current_quantity: i32,
}

impl ShopDefinition {
    /// Initialize stock quantities to their maximum values
    pub fn initialize_stock(&mut self) {
        for item in &mut self.stock {
            item.current_quantity = item.max_quantity;
        }
    }

    /// Get immutable reference to stock item by item_id
    pub fn get_stock(&self, item_id: &str) -> Option<&ShopStockItem> {
        self.stock.iter().find(|s| s.item_id == item_id)
    }

    /// Get mutable reference to stock item by item_id
    pub fn get_stock_mut(&mut self, item_id: &str) -> Option<&mut ShopStockItem> {
        self.stock.iter_mut().find(|s| s.item_id == item_id)
    }

    /// Restock all items by their restock_rate, capped at max_quantity
    pub fn restock(&mut self) {
        for item in &mut self.stock {
            item.current_quantity = (item.current_quantity + item.restock_rate).min(item.max_quantity);
        }
    }
}
