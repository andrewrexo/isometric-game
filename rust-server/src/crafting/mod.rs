//! Crafting System
//!
//! Provides recipe definitions and a registry for crafting items.

pub mod definition;
pub mod registry;

pub use definition::{CraftResult, Ingredient, RecipeCategory, RecipeDefinition};
pub use registry::CraftingRegistry;
