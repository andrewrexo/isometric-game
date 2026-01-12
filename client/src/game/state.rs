use std::collections::HashMap;
use super::entities::Player;
use super::item::{GroundItem, Inventory, RecipeDefinition};
use super::item_registry::ItemRegistry;
use super::npc::Npc;
use super::tilemap::Tilemap;
use super::chunk::ChunkManager;
use super::pathfinding::PathState;
use crate::ui::UiElementId;

pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub initialized: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            initialized: false,
        }
    }
}

pub struct ChatMessage {
    pub sender_name: String,
    pub text: String,
    pub timestamp: f64,
}

/// Floating damage number for combat feedback
pub struct DamageEvent {
    pub x: f32,
    pub y: f32,
    pub damage: i32,
    pub time: f64, // When the event was created (game time)
}

/// Floating level up text
pub struct LevelUpEvent {
    pub x: f32,
    pub y: f32,
    pub new_level: i32,
    pub time: f64,
}

/// Chat bubble displayed above a player's head
pub struct ChatBubble {
    pub player_id: String,
    pub text: String,
    pub time: f64,
}

/// A choice in a dialogue box
#[derive(Clone, Debug)]
pub struct DialogueChoice {
    pub id: String,
    pub text: String,
}

/// Active dialogue being shown to the player
#[derive(Clone, Debug)]
pub struct ActiveDialogue {
    pub quest_id: String,
    pub npc_id: String,
    pub speaker: String,
    pub text: String,
    pub choices: Vec<DialogueChoice>,
    pub show_time: f64,
}

/// A quest objective with progress tracking
#[derive(Clone, Debug)]
pub struct QuestObjective {
    pub id: String,
    pub description: String,
    pub current: i32,
    pub target: i32,
    pub completed: bool,
}

/// An active quest with its objectives
#[derive(Clone, Debug)]
pub struct ActiveQuest {
    pub id: String,
    pub name: String,
    pub objectives: Vec<QuestObjective>,
}

/// Quest completion notification
#[derive(Clone, Debug)]
pub struct QuestCompletedEvent {
    pub quest_id: String,
    pub quest_name: String,
    pub exp_reward: i32,
    pub gold_reward: i32,
    pub time: f64,
}

/// Server-wide announcement from admin
#[derive(Clone, Debug)]
pub struct Announcement {
    pub text: String,
    pub time: f64,
}

/// Context menu for right-clicking items
#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub slot_index: usize,
    pub x: f32,
    pub y: f32,
    pub is_equipment: bool, // true if this is an equipment slot, not inventory
    pub equipment_slot: Option<String>, // "body", "feet", etc. when is_equipment is true
}

/// Source of a drag operation
#[derive(Debug, Clone, PartialEq)]
pub enum DragSource {
    Inventory(usize),          // Inventory slot index
    Equipment(String),         // Equipment slot type ("body", "feet")
}

/// Drag state for inventory/equipment rearrangement
#[derive(Debug, Clone)]
pub struct DragState {
    pub source: DragSource,
    pub item_id: String,
    pub quantity: i32,
}

/// Double-click tracking for inventory slots
#[derive(Debug, Clone)]
pub struct DoubleClickState {
    pub last_click_slot: Option<usize>,
    pub last_click_time: f64,
}

pub struct UiState {
    pub chat_open: bool,
    pub chat_input: String,
    pub chat_cursor: usize,        // Cursor position in chat_input (character index)
    pub chat_scroll_offset: usize, // Scroll offset for long messages (character index)
    pub chat_key_repeat_time: f64, // Last time a repeated key action fired
    pub chat_key_initial_delay: bool, // Whether we're still in initial delay
    pub chat_messages: Vec<ChatMessage>,
    pub inventory_open: bool,
    // Quest UI state
    pub active_dialogue: Option<ActiveDialogue>,
    pub active_quests: Vec<ActiveQuest>,
    pub quest_completed_events: Vec<QuestCompletedEvent>,
    pub quest_log_open: bool,
    // Crafting UI state
    pub crafting_open: bool,
    pub crafting_selected_category: usize,
    pub crafting_selected_recipe: usize,
    pub crafting_npc_id: Option<String>,
    // Escape menu state
    pub escape_menu_open: bool,
    // Mouse hover state for UI elements
    pub hovered_element: Option<UiElementId>,
    // Context menu state
    pub context_menu: Option<ContextMenu>,
    // Drag state for inventory slot rearrangement
    pub drag_state: Option<DragState>,
    // Double-click tracking for equipping items
    pub double_click_state: DoubleClickState,
    // Server announcements
    pub announcements: Vec<Announcement>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            chat_open: false,
            chat_input: String::new(),
            chat_cursor: 0,
            chat_scroll_offset: 0,
            chat_key_repeat_time: 0.0,
            chat_key_initial_delay: true,
            chat_messages: Vec::new(),
            inventory_open: false,
            active_dialogue: None,
            active_quests: Vec::new(),
            quest_completed_events: Vec::new(),
            quest_log_open: false,
            crafting_open: false,
            crafting_selected_category: 0,
            crafting_selected_recipe: 0,
            crafting_npc_id: None,
            escape_menu_open: false,
            hovered_element: None,
            context_menu: None,
            drag_state: None,
            double_click_state: DoubleClickState {
                last_click_slot: None,
                last_click_time: 0.0,
            },
            announcements: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

pub struct GameState {
    // Connection
    pub connection_status: ConnectionStatus,
    pub local_player_id: Option<String>,
    pub selected_character_name: Option<String>,
    pub disconnect_requested: bool,
    pub reconnection_failed: bool,
    /// Timestamp of last Face command sent (to ignore stale server direction updates)
    pub last_face_command_time: f64,

    // World
    pub tilemap: Tilemap,
    pub chunk_manager: ChunkManager,
    pub players: HashMap<String, Player>,
    pub npcs: HashMap<String, Npc>,
    pub ground_items: HashMap<String, GroundItem>,

    // Targeting
    pub selected_entity_id: Option<String>,

    // Combat feedback
    pub damage_events: Vec<DamageEvent>,
    pub level_up_events: Vec<LevelUpEvent>,

    // Chat bubbles above players
    pub chat_bubbles: Vec<ChatBubble>,

    // Inventory
    pub inventory: Inventory,

    // Item registry (loaded from server)
    pub item_registry: ItemRegistry,

    // Crafting
    pub recipe_definitions: Vec<RecipeDefinition>,

    // Camera and UI
    pub camera: Camera,
    pub ui_state: UiState,

    // Server tick (for ordering)
    pub server_tick: u64,

    // Debug
    pub debug_mode: bool,

    // Tile hover state (world coordinates of tile under mouse)
    pub hovered_tile: Option<(i32, i32)>,

    // Automated pathfinding state
    pub auto_path: Option<PathState>,
}

impl GameState {
    pub fn new() -> Self {
        // Create a test tilemap (32x32 tiles) - kept for compatibility
        let tilemap = Tilemap::new_test_map(32, 32);

        Self {
            connection_status: ConnectionStatus::Disconnected,
            local_player_id: None,
            selected_character_name: None,
            disconnect_requested: false,
            reconnection_failed: false,
            last_face_command_time: 0.0,
            tilemap,
            chunk_manager: ChunkManager::new(),
            players: HashMap::new(),
            npcs: HashMap::new(),
            ground_items: HashMap::new(),
            selected_entity_id: None,
            damage_events: Vec::new(),
            level_up_events: Vec::new(),
            chat_bubbles: Vec::new(),
            inventory: Inventory::new(),
            item_registry: ItemRegistry::new(),
            recipe_definitions: Vec::new(),
            camera: Camera::default(),
            ui_state: UiState::default(),
            server_tick: 0,
            debug_mode: false,
            hovered_tile: None,
            auto_path: None,
        }
    }

    /// Clear the current auto-path (e.g., when player presses movement keys)
    pub fn clear_auto_path(&mut self) {
        self.auto_path = None;
    }

    /// Update with current input direction for smooth local movement
    pub fn update(&mut self, delta: f32, input_dx: f32, input_dy: f32) {
        // Update local player - smoothly interpolate visual toward server grid position
        if let Some(local_id) = &self.local_player_id {
            if let Some(player) = self.players.get_mut(local_id) {
                // Update facing direction based on input
                if input_dx != 0.0 || input_dy != 0.0 {
                    let new_dir = super::entities::Direction::from_velocity(input_dx, input_dy);
                    player.direction = new_dir;
                    player.animation.direction = new_dir;
                }

                // Smoothly interpolate visual position toward server grid position
                player.interpolate_visual(delta);
            }
        }

        // Update other players (smooth interpolation toward their server positions)
        if let Some(local_id) = &self.local_player_id {
            for (id, player) in self.players.iter_mut() {
                if id != local_id {
                    player.update(delta);
                }
            }
        } else {
            // No local player yet - update all
            for player in self.players.values_mut() {
                player.update(delta);
            }
        }

        // Update camera to follow local player
        if let Some(local_id) = &self.local_player_id {
            if let Some(player) = self.players.get(local_id) {
                self.camera.x = player.x;
                self.camera.y = player.y;
                self.camera.initialized = true;
            }
        }

        // Update NPCs (interpolation toward server positions)
        for npc in self.npcs.values_mut() {
            npc.update(delta);
        }

        // Clean up old damage events (older than 1.5 seconds)
        let current_time = macroquad::time::get_time();
        self.damage_events.retain(|event| current_time - event.time < 1.5);

        // Clean up old level up events (older than 2.0 seconds)
        self.level_up_events.retain(|event| current_time - event.time < 2.0);

        // Clean up old chat bubbles (older than 5.0 seconds)
        self.chat_bubbles.retain(|bubble| current_time - bubble.time < 5.0);

        // Clean up old quest completion events (older than 4 seconds)
        self.ui_state.quest_completed_events.retain(|event| current_time - event.time < 4.0);

        // Clean up old announcements (older than 8 seconds)
        self.ui_state.announcements.retain(|ann| current_time - ann.time < 8.0);
    }

    pub fn get_local_player(&self) -> Option<&Player> {
        self.local_player_id.as_ref().and_then(|id| self.players.get(id))
    }
}
