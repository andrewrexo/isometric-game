use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{info, warn, error};

use super::prototype::{
    AnimationType, DialogueConfig, EntityBehaviors, EntityPrototype, LootEntry,
    RawEntityPrototype, ResolvedRewards, ResolvedStats,
};

/// Registry for all entity prototypes
pub struct EntityRegistry {
    prototypes: HashMap<String, EntityPrototype>,
}

impl EntityRegistry {
    pub fn new() -> Self {
        Self {
            prototypes: HashMap::new(),
        }
    }

    /// Load all entity definitions from a directory
    pub fn load_from_directory(&mut self, data_dir: &Path) -> Result<(), String> {
        let entities_dir = data_dir.join("entities");

        // First pass: load all raw prototypes
        let mut raw_prototypes: HashMap<String, RawEntityPrototype> = HashMap::new();

        // Load monsters
        let monsters_dir = entities_dir.join("monsters");
        if monsters_dir.exists() {
            self.load_toml_files(&monsters_dir, &mut raw_prototypes)?;
        }

        // Load NPCs
        let npcs_dir = entities_dir.join("npcs");
        if npcs_dir.exists() {
            self.load_toml_files(&npcs_dir, &mut raw_prototypes)?;
        }

        info!("Loaded {} raw entity prototypes", raw_prototypes.len());

        // Second pass: resolve inheritance
        self.resolve_all_prototypes(raw_prototypes)?;

        info!("Resolved {} entity prototypes", self.prototypes.len());

        Ok(())
    }

    fn load_toml_files(
        &self,
        dir: &Path,
        raw_prototypes: &mut HashMap<String, RawEntityPrototype>,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "toml") {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;

                // Parse as table of entities
                let table: HashMap<String, RawEntityPrototype> = toml::from_str(&content)
                    .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;

                for (id, proto) in table {
                    if raw_prototypes.contains_key(&id) {
                        warn!("Duplicate entity ID '{}' in {:?}, overwriting", id, path);
                    }
                    info!("Loaded entity prototype: {}", id);
                    raw_prototypes.insert(id, proto);
                }
            }
        }

        Ok(())
    }

    fn resolve_all_prototypes(
        &mut self,
        raw_prototypes: HashMap<String, RawEntityPrototype>,
    ) -> Result<(), String> {
        // Topological sort to handle inheritance order
        let sorted_ids = self.topological_sort(&raw_prototypes)?;

        for id in sorted_ids {
            let raw = raw_prototypes.get(&id).unwrap();
            let resolved = self.resolve_prototype(&id, raw)?;
            self.prototypes.insert(id, resolved);
        }

        Ok(())
    }

    fn topological_sort(
        &self,
        raw_prototypes: &HashMap<String, RawEntityPrototype>,
    ) -> Result<Vec<String>, String> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn visit(
            id: &str,
            raw_prototypes: &HashMap<String, RawEntityPrototype>,
            sorted: &mut Vec<String>,
            visited: &mut HashSet<String>,
            visiting: &mut HashSet<String>,
        ) -> Result<(), String> {
            if visited.contains(id) {
                return Ok(());
            }
            if visiting.contains(id) {
                return Err(format!("Circular inheritance detected at '{}'", id));
            }

            visiting.insert(id.to_string());

            if let Some(raw) = raw_prototypes.get(id) {
                if let Some(parent_id) = &raw.extends {
                    if !raw_prototypes.contains_key(parent_id) {
                        return Err(format!(
                            "Entity '{}' extends unknown parent '{}'",
                            id, parent_id
                        ));
                    }
                    visit(parent_id, raw_prototypes, sorted, visited, visiting)?;
                }
            }

            visiting.remove(id);
            visited.insert(id.to_string());
            sorted.push(id.to_string());

            Ok(())
        }

        for id in raw_prototypes.keys() {
            visit(id, raw_prototypes, &mut sorted, &mut visited, &mut visiting)?;
        }

        Ok(sorted)
    }

    fn resolve_prototype(
        &self,
        id: &str,
        raw: &RawEntityPrototype,
    ) -> Result<EntityPrototype, String> {
        // Get parent prototype if extends is specified
        let parent = if let Some(parent_id) = &raw.extends {
            self.prototypes.get(parent_id)
        } else {
            None
        };

        // Merge stats with parent (child overrides parent)
        let stats = ResolvedStats {
            max_hp: raw.stats.max_hp
                .or_else(|| parent.map(|p| p.stats.max_hp))
                .unwrap_or(100),
            damage: raw.stats.damage
                .or_else(|| parent.map(|p| p.stats.damage))
                .unwrap_or(10),
            attack_range: raw.stats.attack_range
                .or_else(|| parent.map(|p| p.stats.attack_range))
                .unwrap_or(1),
            aggro_range: raw.stats.aggro_range
                .or_else(|| parent.map(|p| p.stats.aggro_range))
                .unwrap_or(5),
            chase_range: raw.stats.chase_range
                .or_else(|| parent.map(|p| p.stats.chase_range))
                .unwrap_or(8),
            move_cooldown_ms: raw.stats.move_cooldown_ms
                .or_else(|| parent.map(|p| p.stats.move_cooldown_ms))
                .unwrap_or(500),
            attack_cooldown_ms: raw.stats.attack_cooldown_ms
                .or_else(|| parent.map(|p| p.stats.attack_cooldown_ms))
                .unwrap_or(1000),
            respawn_time_ms: raw.stats.respawn_time_ms
                .or_else(|| parent.map(|p| p.stats.respawn_time_ms))
                .unwrap_or(10000),
        };

        // Merge rewards
        let rewards = ResolvedRewards {
            exp_base: raw.rewards.exp_base
                .or_else(|| parent.map(|p| p.rewards.exp_base))
                .unwrap_or(10),
            gold_min: raw.rewards.gold_min
                .or_else(|| parent.map(|p| p.rewards.gold_min))
                .unwrap_or(1),
            gold_max: raw.rewards.gold_max
                .or_else(|| parent.map(|p| p.rewards.gold_max))
                .unwrap_or(5),
        };

        // Merge loot tables (child appends to parent)
        let mut loot: Vec<LootEntry> = parent
            .map(|p| p.loot.clone())
            .unwrap_or_default();
        loot.extend(raw.loot.clone());

        // Parse animation type
        let animation_type = raw.animation_type.as_deref()
            .map(AnimationType::from_str)
            .or_else(|| parent.map(|p| p.animation_type))
            .unwrap_or_default();

        // Merge behaviors (child overrides if set)
        let behaviors = EntityBehaviors::from(&raw.behaviors);

        Ok(EntityPrototype {
            id: id.to_string(),
            display_name: raw.display_name.clone()
                .or_else(|| parent.map(|p| p.display_name.clone()))
                .unwrap_or_else(|| id.to_string()),
            sprite: raw.sprite.clone()
                .or_else(|| parent.map(|p| p.sprite.clone()))
                .unwrap_or_else(|| "unknown".to_string()),
            animation_type,
            description: raw.description.clone()
                .or_else(|| parent.map(|p| p.description.clone()))
                .unwrap_or_default(),
            stats,
            rewards,
            loot,
            behaviors,
            merchant: raw.merchant.clone()
                .or_else(|| parent.and_then(|p| p.merchant.clone())),
            quest_giver: raw.quest_giver.clone()
                .or_else(|| parent.and_then(|p| p.quest_giver.clone())),
            dialogue: raw.dialogue.clone().unwrap_or_default(),
        })
    }

    /// Get a prototype by ID
    pub fn get(&self, id: &str) -> Option<&EntityPrototype> {
        self.prototypes.get(id)
    }

    /// Get all prototype IDs
    pub fn ids(&self) -> impl Iterator<Item = &String> {
        self.prototypes.keys()
    }

    /// Get all prototypes
    pub fn all(&self) -> impl Iterator<Item = &EntityPrototype> {
        self.prototypes.values()
    }

    /// Check if a prototype exists
    pub fn contains(&self, id: &str) -> bool {
        self.prototypes.contains_key(id)
    }

    /// Get the number of loaded prototypes
    pub fn len(&self) -> usize {
        self.prototypes.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.prototypes.is_empty()
    }
}

impl Default for EntityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
