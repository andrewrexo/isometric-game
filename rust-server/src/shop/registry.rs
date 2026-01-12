//! Shop Registry
//!
//! Loads and caches shop definitions from TOML files.

use super::definition::ShopDefinition;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Registry for all shop definitions
pub struct ShopRegistry {
    shops: HashMap<String, ShopDefinition>,
}

impl ShopRegistry {
    /// Create a new empty shop registry
    pub fn new() -> Self {
        Self {
            shops: HashMap::new(),
        }
    }

    /// Load all shop definitions from a directory
    pub fn load_from_directory(&mut self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            warn!("Shop directory does not exist: {:?}", path);
            return Ok(());
        }

        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let file_path = entry.path();

            if file_path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let contents = fs::read_to_string(&file_path)
                    .map_err(|e| format!("Failed to read {:?}: {}", file_path, e))?;

                let mut shop: ShopDefinition = toml::from_str(&contents)
                    .map_err(|e| format!("Failed to parse {:?}: {}", file_path, e))?;

                shop.initialize_stock();

                if self.shops.contains_key(&shop.id) {
                    warn!("Duplicate shop ID '{}' in {:?}, overwriting", shop.id, file_path);
                }

                self.shops.insert(shop.id.clone(), shop);
            }
        }

        info!("Loaded {} shop definitions", self.shops.len());
        Ok(())
    }

    /// Get a shop definition by ID
    pub fn get(&self, shop_id: &str) -> Option<&ShopDefinition> {
        self.shops.get(shop_id)
    }

    /// Get a mutable shop definition by ID
    pub fn get_mut(&mut self, shop_id: &str) -> Option<&mut ShopDefinition> {
        self.shops.get_mut(shop_id)
    }

    /// Get an iterator over all shop IDs
    pub fn ids(&self) -> impl Iterator<Item = &String> {
        self.shops.keys()
    }

    /// Get an iterator over all shop definitions
    pub fn all(&self) -> impl Iterator<Item = &ShopDefinition> {
        self.shops.values()
    }

    /// Check if a shop exists in the registry
    pub fn contains(&self, shop_id: &str) -> bool {
        self.shops.contains_key(shop_id)
    }

    /// Get the number of shops in the registry
    pub fn len(&self) -> usize {
        self.shops.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.shops.is_empty()
    }
}

impl Default for ShopRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_load_shops_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let shops_dir = temp_dir.path();

        let toml_content = r#"
id = "test_shop"
display_name = "Test Shop"

[[stock]]
item_id = "test_item"
max_quantity = 5
restock_rate = 1
"#;

        let mut file = std::fs::File::create(shops_dir.join("test.toml")).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let mut registry = ShopRegistry::new();
        registry.load_from_directory(shops_dir).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_shop"));

        let shop = registry.get("test_shop").unwrap();
        assert_eq!(shop.display_name, "Test Shop");
        assert_eq!(shop.stock.len(), 1);
    }
}
