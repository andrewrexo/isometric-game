//! Crafting Recipe Registry
//!
//! Loads and caches recipe definitions from TOML files.

use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

use super::definition::{RawRecipeDefinition, RecipeCategory, RecipeDefinition};

/// Registry for all recipe definitions
pub struct CraftingRegistry {
    recipes: HashMap<String, RecipeDefinition>,
}

impl CraftingRegistry {
    pub fn new() -> Self {
        Self {
            recipes: HashMap::new(),
        }
    }

    /// Load all recipe definitions from a directory
    pub fn load_from_directory(&mut self, data_dir: &Path) -> Result<(), String> {
        let recipes_dir = data_dir.join("recipes");

        if !recipes_dir.exists() {
            warn!("Recipes directory does not exist: {:?}", recipes_dir);
            return Ok(());
        }

        let entries = std::fs::read_dir(&recipes_dir)
            .map_err(|e| format!("Failed to read recipes directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "toml") {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;

                // Parse as table of recipes
                let table: HashMap<String, RawRecipeDefinition> = toml::from_str(&content)
                    .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;

                for (id, raw) in table {
                    if self.recipes.contains_key(&id) {
                        warn!("Duplicate recipe ID '{}' in {:?}, overwriting", id, path);
                    }
                    let recipe = RecipeDefinition::from_raw(&id, &raw);
                    info!(
                        "Loaded recipe: {} ({}) - {} ingredients -> {} results",
                        recipe.display_name,
                        id,
                        recipe.ingredients.len(),
                        recipe.results.len()
                    );
                    self.recipes.insert(id, recipe);
                }
            }
        }

        info!("Loaded {} recipe definitions", self.recipes.len());

        Ok(())
    }

    /// Get a recipe definition by ID
    pub fn get(&self, id: &str) -> Option<&RecipeDefinition> {
        self.recipes.get(id)
    }

    /// Get all recipe IDs
    pub fn ids(&self) -> impl Iterator<Item = &String> {
        self.recipes.keys()
    }

    /// Get all recipes
    pub fn all(&self) -> impl Iterator<Item = &RecipeDefinition> {
        self.recipes.values()
    }

    /// Get recipes by category
    pub fn by_category(&self, category: RecipeCategory) -> Vec<&RecipeDefinition> {
        self.recipes
            .values()
            .filter(|r| r.category == category)
            .collect()
    }

    /// Check if a recipe exists
    pub fn contains(&self, id: &str) -> bool {
        self.recipes.contains_key(id)
    }

    /// Get the number of loaded recipes
    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }

    /// Generate recipe definitions message for client sync
    pub fn to_client_definitions(&self) -> crate::protocol::ServerMessage {
        use crate::item::ItemType;
        use crate::protocol::{ClientRecipeDef, RecipeIngredient, RecipeResult};

        let recipes: Vec<ClientRecipeDef> = self
            .recipes
            .values()
            .map(|recipe| ClientRecipeDef {
                id: recipe.id.clone(),
                display_name: recipe.display_name.clone(),
                description: recipe.description.clone(),
                category: recipe.category.as_str().to_string(),
                level_required: recipe.level_required,
                ingredients: recipe
                    .ingredients
                    .iter()
                    .map(|i| {
                        // Look up item name from ItemType
                        let item_name = ItemType::from_id(&i.item_id)
                            .map(|t| t.name().to_string())
                            .unwrap_or_else(|| i.item_id.clone());
                        RecipeIngredient {
                            item_id: i.item_id.clone(),
                            item_name,
                            count: i.count,
                        }
                    })
                    .collect(),
                results: recipe
                    .results
                    .iter()
                    .map(|r| {
                        // Look up item name from ItemType
                        let item_name = ItemType::from_id(&r.item_id)
                            .map(|t| t.name().to_string())
                            .unwrap_or_else(|| r.item_id.clone());
                        RecipeResult {
                            item_id: r.item_id.clone(),
                            item_name,
                            count: r.count,
                        }
                    })
                    .collect(),
            })
            .collect();

        crate::protocol::ServerMessage::RecipeDefinitions { recipes }
    }
}

impl Default for CraftingRegistry {
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
    fn test_load_recipes_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let recipes_dir = temp_dir.path().join("recipes");
        std::fs::create_dir(&recipes_dir).unwrap();

        let toml_content = r#"
[test_recipe]
display_name = "Test Recipe"
category = "consumables"
level_required = 1

[[test_recipe.ingredients]]
item_id = "slime_core"
count = 3

[[test_recipe.results]]
item_id = "health_potion"
count = 1
"#;

        let mut file = std::fs::File::create(recipes_dir.join("test.toml")).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let mut registry = CraftingRegistry::new();
        registry.load_from_directory(temp_dir.path()).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_recipe"));

        let recipe = registry.get("test_recipe").unwrap();
        assert_eq!(recipe.display_name, "Test Recipe");
        assert_eq!(recipe.category, RecipeCategory::Consumables);
        assert_eq!(recipe.ingredients.len(), 1);
        assert_eq!(recipe.ingredients[0].item_id, "slime_core");
        assert_eq!(recipe.ingredients[0].count, 3);
    }
}
