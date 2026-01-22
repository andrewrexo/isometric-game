use std::collections::HashMap;
use super::entities::Player;
use super::item::{GroundItem, Inventory, RecipeDefinition};
use super::item_registry::ItemRegistry;
use super::npc::Npc;
use super::tilemap::Tilemap;
use super::chunk::ChunkManager;
use super::pathfinding::PathState;
use super::shop::{ShopData, ShopSubTab};
use crate::render::animation::AnimationState;
use crate::ui::UiElementId;

/// Frame timing diagnostics for performance analysis
#[derive(Clone)]
pub struct FrameTimings {
    pub network_ms: f64,
    pub render_total_ms: f64,
    pub render_ground_ms: f64,
    pub render_entities_ms: f64,
    pub render_overhead_ms: f64,
    pub render_ui_ms: f64,
    pub render_effects_ms: f64,
    pub update_ms: f64,
    pub total_ms: f64,
    pub entity_count: usize,
    pub chunk_count: usize,
    pub tiles_rendered: usize,
    // Frame delta tracking (rolling window)
    pub delta_ms: f64,
    pub delta_min_ms: f64,
    pub delta_max_ms: f64,
    delta_samples: [f64; 60],
    delta_idx: usize,
    // Optional FPS cap (None = uncapped)
    pub fps_cap: Option<u32>,
    // Time spent in next_frame().await (for diagnosing variance)
    pub next_frame_ms: f64,
    pub next_frame_min_ms: f64,
    pub next_frame_max_ms: f64,
    next_frame_samples: [f64; 60],
    next_frame_idx: usize,
    // Smoothed delta for visual interpolation (0.0 = no smoothing, 1.0 = max smoothing)
    pub delta_smoothing: f32,
    pub smoothed_delta: f32,
}

impl Default for FrameTimings {
    fn default() -> Self {
        Self {
            network_ms: 0.0,
            render_total_ms: 0.0,
            render_ground_ms: 0.0,
            render_entities_ms: 0.0,
            render_overhead_ms: 0.0,
            render_ui_ms: 0.0,
            render_effects_ms: 0.0,
            update_ms: 0.0,
            total_ms: 0.0,
            entity_count: 0,
            chunk_count: 0,
            tiles_rendered: 0,
            delta_ms: 0.0,
            delta_min_ms: 0.0,
            delta_max_ms: 0.0,
            delta_samples: [0.0; 60],
            delta_idx: 0,
            fps_cap: None, // Uncapped by default
            next_frame_ms: 0.0,
            next_frame_min_ms: 0.0,
            next_frame_max_ms: 0.0,
            next_frame_samples: [0.0; 60],
            next_frame_idx: 0,
            delta_smoothing: 0.0, // 0.0 = disabled, try 0.5-0.9 for smoothing
            smoothed_delta: 0.016, // Start at ~60fps
        }
    }
}

impl FrameTimings {
    pub fn record_next_frame(&mut self, ms: f64) {
        self.next_frame_ms = ms;
        self.next_frame_samples[self.next_frame_idx] = ms;
        self.next_frame_idx = (self.next_frame_idx + 1) % 60;

        // Calculate min/max over the window
        self.next_frame_min_ms = f64::MAX;
        self.next_frame_max_ms = f64::MIN;
        for &sample in &self.next_frame_samples {
            if sample > 0.0 {
                self.next_frame_min_ms = self.next_frame_min_ms.min(sample);
                self.next_frame_max_ms = self.next_frame_max_ms.max(sample);
            }
        }
        if self.next_frame_min_ms == f64::MAX {
            self.next_frame_min_ms = ms;
        }
        if self.next_frame_max_ms == f64::MIN {
            self.next_frame_max_ms = ms;
        }
    }
}

impl FrameTimings {
    pub fn record_delta(&mut self, delta_ms: f64) {
        self.delta_ms = delta_ms;
        self.delta_samples[self.delta_idx] = delta_ms;
        self.delta_idx = (self.delta_idx + 1) % 60;

        // Calculate min/max over the window
        self.delta_min_ms = f64::MAX;
        self.delta_max_ms = f64::MIN;
        for &sample in &self.delta_samples {
            if sample > 0.0 {
                self.delta_min_ms = self.delta_min_ms.min(sample);
                self.delta_max_ms = self.delta_max_ms.max(sample);
            }
        }
        if self.delta_min_ms == f64::MAX {
            self.delta_min_ms = delta_ms;
        }
        if self.delta_max_ms == f64::MIN {
            self.delta_max_ms = delta_ms;
        }

        // Update smoothed delta for visual interpolation
        let delta_secs = (delta_ms / 1000.0) as f32;
        if self.delta_smoothing > 0.0 {
            self.smoothed_delta = self.smoothed_delta * self.delta_smoothing
                + delta_secs * (1.0 - self.delta_smoothing);
        } else {
            self.smoothed_delta = delta_secs;
        }
    }
}

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

/// Chat channel types for different message sources
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatChannel {
    Local,      // Nearby players only (current default)
    Global,     // Server-wide player chat
    System,     // XP gains, quest completions, shop transactions
    // Future:
    // Party,
    // Guild,
}

pub struct ChatMessage {
    pub sender_name: String,
    pub text: String,
    pub timestamp: f64,
    pub channel: ChatChannel,
}

impl ChatMessage {
    /// Create a player chat message (defaults to Local channel)
    pub fn player(sender_name: String, text: String) -> Self {
        ChatMessage {
            sender_name,
            text,
            timestamp: macroquad::time::get_time(),
            channel: ChatChannel::Local,
        }
    }

    /// Create a system message
    pub fn system(text: String) -> Self {
        ChatMessage {
            sender_name: "[System]".to_string(),
            text,
            timestamp: macroquad::time::get_time(),
            channel: ChatChannel::System,
        }
    }
}

/// Floating damage/healing number for combat feedback
/// - Positive damage = damage dealt (red)
/// - Negative damage = healing (green, displayed as +X)
/// - Zero = miss (gray, displayed as "MISS")
pub struct DamageEvent {
    pub x: f32,
    pub y: f32,
    pub damage: i32,
    pub time: f64,
    /// Entity ID to look up sprite height at render time
    pub target_id: String,
    pub source_id: Option<String>,
    pub projectile: Option<String>,
}

/// Active projectile for ranged attack visualization
pub struct Projectile {
    pub sprite: String,
    pub start_x: f32,
    pub start_y: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub start_time: f64,
    pub duration: f64,
}

impl Projectile {
    /// Get current position (0.0 to 1.0 progress)
    pub fn progress(&self, current_time: f64) -> f32 {
        let elapsed = current_time - self.start_time;
        (elapsed / self.duration).min(1.0) as f32
    }

    /// Check if projectile animation is complete
    pub fn is_complete(&self, current_time: f64) -> bool {
        current_time - self.start_time >= self.duration
    }

    /// Get current world position
    pub fn current_pos(&self, current_time: f64) -> (f32, f32) {
        let t = self.progress(current_time);
        let x = self.start_x + (self.end_x - self.start_x) * t;
        let y = self.start_y + (self.end_y - self.start_y) * t;
        (x, y)
    }
}

/// Floating level up text
pub struct LevelUpEvent {
    pub x: f32,
    pub y: f32,
    pub skill: String,
    pub new_level: i32,
    pub time: f64,
}

/// Floating skill XP gain text
pub struct SkillXpEvent {
    pub x: f32,
    pub y: f32,
    pub skill: String,
    pub xp_gained: i64,
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

/// Target for context menu - what was right-clicked
#[derive(Debug, Clone)]
pub enum ContextMenuTarget {
    InventorySlot(usize),
    EquipmentSlot(String),
    Gold,
}

/// Context menu for right-clicking items
#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub target: ContextMenuTarget,
    pub x: f32,
    pub y: f32,
}

/// Dialog for entering gold drop amount
#[derive(Debug, Clone)]
pub struct GoldDropDialog {
    pub input: String,
    pub cursor: usize,
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
    // Shop UI state
    pub shop_data: Option<ShopData>,
    pub shop_npc_id: Option<String>,
    pub shop_sub_tab: ShopSubTab,
    pub shop_main_tab: usize, // 0=Recipes, 1=Shop
    pub shop_selected_buy_index: usize,
    pub shop_selected_sell_index: usize,
    pub shop_buy_quantity: i32,
    pub shop_sell_quantity: i32,
    pub shop_buy_scroll: f32,  // Scroll offset for buy list (pixels)
    pub shop_sell_scroll: f32, // Scroll offset for sell list (pixels)
    // Escape menu state
    pub escape_menu_open: bool,
    // Audio settings (synced with AudioManager)
    pub audio_volume: f32,
    pub audio_sfx_volume: f32,
    pub audio_muted: bool,
    // Input settings
    pub shift_drop_enabled: bool,
    // Menu button panel states
    pub character_open: bool,
    pub social_open: bool,
    pub skills_open: bool,
    pub gear_panel_open: bool,
    // Mouse hover state for UI elements
    pub hovered_element: Option<UiElementId>,
    // Context menu state
    pub context_menu: Option<ContextMenu>,
    // Gold drop dialog state
    pub gold_drop_dialog: Option<GoldDropDialog>,
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
            shop_data: None,
            shop_npc_id: None,
            shop_sub_tab: ShopSubTab::Buy,
            shop_main_tab: 0,
            shop_selected_buy_index: 0,
            shop_selected_sell_index: 0,
            shop_buy_quantity: 1,
            shop_sell_quantity: 1,
            shop_buy_scroll: 0.0,
            shop_sell_scroll: 0.0,
            escape_menu_open: false,
            audio_volume: 0.7,
            audio_sfx_volume: 0.7,
            audio_muted: false,
            shift_drop_enabled: true,
            character_open: false,
            social_open: false,
            skills_open: false,
            gear_panel_open: false,
            hovered_element: None,
            context_menu: None,
            gold_drop_dialog: None,
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
    pub skill_xp_events: Vec<SkillXpEvent>,
    pub projectiles: Vec<Projectile>,

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

    // Entity hover state (ID of entity under mouse cursor)
    pub hovered_entity_id: Option<String>,

    // Automated pathfinding state
    pub auto_path: Option<PathState>,

    // Performance diagnostics (visible in debug mode)
    pub frame_timings: FrameTimings,
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
            skill_xp_events: Vec::new(),
            projectiles: Vec::new(),
            chat_bubbles: Vec::new(),
            inventory: Inventory::new(),
            item_registry: ItemRegistry::new(),
            recipe_definitions: Vec::new(),
            camera: Camera::default(),
            ui_state: UiState::default(),
            server_tick: 0,
            debug_mode: false,
            hovered_tile: None,
            hovered_entity_id: None,
            auto_path: None,
            frame_timings: FrameTimings::default(),
        }
    }

    /// Clear the current auto-path (e.g., when player presses movement keys)
    pub fn clear_auto_path(&mut self) {
        self.auto_path = None;
    }

    /// Update with current input direction for smooth local movement
    pub fn update(&mut self, delta: f32, input_dx: f32, input_dy: f32) {
        // Use smoothed delta for visual interpolation (reduces jitter from frame variance)
        let visual_delta = self.frame_timings.smoothed_delta;

        // Update local player - smoothly interpolate visual toward server grid position
        if let Some(local_id) = &self.local_player_id {
            if let Some(player) = self.players.get_mut(local_id) {
                // Update facing direction based on input only when NOT moving and NOT attacking
                // This prevents 1-frame direction jitter at tile boundaries
                // (is_moving reflects previous frame, set by interpolate_visual)
                let is_attacking = matches!(
                    player.animation.state,
                    AnimationState::Attacking | AnimationState::Casting | AnimationState::ShootingBow
                );
                if !player.is_moving && !is_attacking && (input_dx != 0.0 || input_dy != 0.0) {
                    let new_dir = super::entities::Direction::from_velocity(input_dx, input_dy);
                    player.direction = new_dir;
                    player.animation.direction = new_dir;
                }

                // Smoothly interpolate visual position toward server grid position
                player.interpolate_visual(visual_delta);
            }
        }

        // Update other players (smooth interpolation toward their server positions)
        if let Some(local_id) = &self.local_player_id {
            for (id, player) in self.players.iter_mut() {
                if id != local_id {
                    player.update(visual_delta);
                }
            }
        } else {
            // No local player yet - update all
            for player in self.players.values_mut() {
                player.update(visual_delta);
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
            npc.update(visual_delta);
        }

        // Clean up old damage events (older than 1.2 seconds)
        let current_time = macroquad::time::get_time();
        self.damage_events.retain(|event| current_time - event.time < 1.2);

        // Clean up old level up events (older than 2.0 seconds)
        self.level_up_events.retain(|event| current_time - event.time < 2.0);

        // Clean up old skill XP events (older than 1.5 seconds)
        self.skill_xp_events.retain(|event| current_time - event.time < 1.5);

        // Clean up old chat bubbles (older than 5.0 seconds)
        self.chat_bubbles.retain(|bubble| current_time - bubble.time < 5.0);

        // Clean up completed projectiles
        self.projectiles.retain(|p| !p.is_complete(current_time));

        // Clean up old quest completion events (older than 4 seconds)
        self.ui_state.quest_completed_events.retain(|event| current_time - event.time < 4.0);

        // Clean up old announcements (older than 8 seconds)
        self.ui_state.announcements.retain(|ann| current_time - ann.time < 8.0);
    }

    pub fn get_local_player(&self) -> Option<&Player> {
        self.local_player_id.as_ref().and_then(|id| self.players.get(id))
    }
}
