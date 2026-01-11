pub mod state;
mod entities;
pub mod tilemap;
pub mod npc;
pub mod item;
pub mod item_registry;
pub mod chunk;

pub use state::{GameState, Camera, ConnectionStatus, ChatMessage, ChatBubble, UiState, DamageEvent, LevelUpEvent, DialogueChoice, ActiveDialogue, QuestObjective, ActiveQuest, QuestCompletedEvent, ContextMenu, DragState, DragSource, DoubleClickState};
pub use entities::{Player, Direction};
pub use tilemap::{Tilemap, TilemapLayer, LayerType};
pub use npc::{Npc, NpcType, NpcState};
pub use item::{GroundItem, Inventory, InventorySlot, INVENTORY_SIZE, RecipeDefinition, RecipeIngredient, RecipeResult};
pub use item_registry::{ItemRegistry, ItemDefinition, EquipmentStats};
pub use chunk::{ChunkManager, ChunkCoord, ChunkLayerType, Chunk, CHUNK_SIZE};
