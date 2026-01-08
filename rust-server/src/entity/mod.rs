pub mod prototype;
pub mod registry;
pub mod spawner;

pub use prototype::{AnimationType, EntityPrototype, LootEntry};
pub use registry::EntityRegistry;
pub use spawner::{calculate_exp_reward, generate_loot_from_prototype};
