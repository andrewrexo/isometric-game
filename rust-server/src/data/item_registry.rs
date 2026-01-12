use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

use super::item_def::{ItemDefinition, RawItemDefinition};

/// Registry for all item definitions
pub struct ItemRegistry {
    items: HashMap<String, ItemDefinition>,
}

impl ItemRegistry {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    /// Load all item definitions from a directory
    pub fn load_from_directory(&mut self, data_dir: &Path) -> Result<(), String> {
        let items_dir = data_dir.join("items");

        if !items_dir.exists() {
            warn!("Items directory does not exist: {:?}", items_dir);
            return Ok(());
        }

        let entries = std::fs::read_dir(&items_dir)
            .map_err(|e| format!("Failed to read items directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "toml") {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;

                // Parse as table of items
                let table: HashMap<String, RawItemDefinition> = toml::from_str(&content)
                    .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;

                for (id, raw) in table {
                    if self.items.contains_key(&id) {
                        warn!("Duplicate item ID '{}' in {:?}, overwriting", id, path);
                    }
                    let item = ItemDefinition::from_raw(&id, &raw);
                    // info!("Loaded item: {} ({})", item.display_name, id);
                    self.items.insert(id, item);
                }
            }
        }

        info!("Loaded {} item definitions", self.items.len());

        Ok(())
    }

    /// Get an item definition by ID
    pub fn get(&self, id: &str) -> Option<&ItemDefinition> {
        self.items.get(id)
    }

    /// Get all item IDs
    pub fn ids(&self) -> impl Iterator<Item = &String> {
        self.items.keys()
    }

    /// Get all items
    pub fn all(&self) -> impl Iterator<Item = &ItemDefinition> {
        self.items.values()
    }

    /// Check if an item exists
    pub fn contains(&self, id: &str) -> bool {
        self.items.contains_key(id)
    }

    /// Get the number of loaded items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Generate item definitions message for client sync
    pub fn to_client_definitions(&self) -> crate::protocol::ServerMessage {
        use crate::protocol::ClientItemDef;
        use super::item_def::EquipmentSlot;

        let items: Vec<ClientItemDef> = self.items
            .values()
            .map(|item| {
                // Extract equipment stats if present and has valid slot
                let (equipment_slot, level_required, damage_bonus, defense_bonus) =
                    if let Some(ref equip) = item.equipment {
                        if equip.slot_type != EquipmentSlot::None {
                            (
                                Some(equip.slot_type.as_str().to_string()),
                                Some(equip.level_required),
                                Some(equip.damage_bonus),
                                Some(equip.defense_bonus),
                            )
                        } else {
                            (None, None, None, None)
                        }
                    } else {
                        (None, None, None, None)
                    };

                ClientItemDef {
                    id: item.id.clone(),
                    display_name: item.display_name.clone(),
                    sprite: item.sprite.clone(),
                    category: format!("{:?}", item.category).to_lowercase(),
                    max_stack: item.max_stack,
                    description: item.description.clone(),
                    equipment_slot,
                    level_required,
                    damage_bonus,
                    defense_bonus,
                }
            })
            .collect();

        crate::protocol::ServerMessage::ItemDefinitions { items }
    }
}

impl Default for ItemRegistry {
    fn default() -> Self {
        Self::new()
    }
}
