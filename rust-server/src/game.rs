use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::chunk::ChunkCoord;
use crate::entity::{EntityPrototype, EntityRegistry};
use crate::data::ItemRegistry;
use crate::skills::{Skills, SkillType, calculate_hit, calculate_max_hit, roll_damage};
use crate::item::{self, GroundItem, Inventory, GOLD_ITEM_ID};
use crate::npc::{Npc, NpcType, NpcUpdate};
use crate::protocol::{ServerMessage, QuestObjectiveData};
use crate::quest::{QuestRegistry, QuestRunner, PlayerQuestState, QuestEvent};
use crate::shop::{ShopRegistry, ShopDefinition, ShopStockItem};
use crate::world::World;

// ============================================================================
// Constants
// ============================================================================

const TICK_RATE: f32 = 20.0;

// Grid-based movement: ticks between tile moves (5 ticks * 50ms = 250ms = 15 frames at 60fps)
const MOVE_COOLDOWN_TICKS: u64 = 5;

const MAP_WIDTH: u32 = 32;
const MAP_HEIGHT: u32 = 32;
const STARTING_HP: i32 = 100;

// Combat constants
const ATTACK_RANGE: i32 = 1; // Maximum distance to attack (in tiles)
const ATTACK_COOLDOWN_MS: u64 = 700; // Slightly shorter than client (800ms) to account for network latency

// ============================================================================
// Player Save Data (for database persistence)
// ============================================================================

#[derive(Debug, Clone)]
pub struct PlayerSaveData {
    pub x: f32,
    pub y: f32,
    pub hp: i32,
    pub skills: Skills,
    pub gold: i32,
    pub inventory_json: String,
    pub gender: String,
    pub skin: String,
    pub equipped_head: Option<String>,
    pub equipped_body: Option<String>,
    pub equipped_weapon: Option<String>,
    pub equipped_back: Option<String>,
    pub equipped_feet: Option<String>,
    pub equipped_ring: Option<String>,
    pub equipped_gloves: Option<String>,
    pub equipped_necklace: Option<String>,
    pub equipped_belt: Option<String>,
}

// ============================================================================
// Direction
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Direction {
    Down = 0,
    Left = 1,
    Up = 2,
    Right = 3,
    DownLeft = 4,
    DownRight = 5,
    UpLeft = 6,
    UpRight = 7,
}

impl Direction {
    pub fn from_velocity(dx: f32, dy: f32) -> Self {
        if dx == 0.0 && dy == 0.0 {
            return Direction::Down;
        }

        let angle = dy.atan2(dx);
        let octant = ((angle + std::f32::consts::PI) / (std::f32::consts::PI / 4.0)).round() as i32 % 8;

        match octant {
            0 => Direction::Left,
            1 => Direction::UpLeft,
            2 => Direction::Up,
            3 => Direction::UpRight,
            4 => Direction::Right,
            5 => Direction::DownRight,
            6 => Direction::Down,
            7 => Direction::DownLeft,
            _ => Direction::Down,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Direction::Down,
            1 => Direction::Left,
            2 => Direction::Up,
            3 => Direction::Right,
            4 => Direction::DownLeft,
            5 => Direction::DownRight,
            6 => Direction::UpLeft,
            7 => Direction::UpRight,
            _ => Direction::Down,
        }
    }
}

// ============================================================================
// Player
// ============================================================================

#[derive(Debug, Clone)]
pub struct Player {
    pub id: String,
    pub name: String,
    // Grid position (integer tile coordinates)
    pub x: i32,
    pub y: i32,
    pub spawn_x: i32,
    pub spawn_y: i32,
    // Queued movement direction (-1, 0, or 1)
    pub move_dx: i32,
    pub move_dy: i32,
    pub last_move_tick: u64, // Tick-based movement cooldown
    pub direction: Direction,
    pub hp: i32,
    pub skills: Skills, // Combat skills (Hitpoints determines max HP)
    pub active: bool, // Whether WebSocket is connected
    pub target_id: Option<String>, // Currently targeted entity (player or NPC)
    pub last_attack_time: u64, // Timestamp of last attack (ms)
    pub is_dead: bool,
    pub death_time: u64, // When the player died (for respawn timer)
    pub inventory: Inventory,
    // Character appearance
    pub gender: String, // "male" or "female"
    pub skin: String,   // "tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"
    // Equipment
    pub equipped_head: Option<String>,
    pub equipped_body: Option<String>,
    pub equipped_weapon: Option<String>,
    pub equipped_back: Option<String>,
    pub equipped_feet: Option<String>,
    pub equipped_ring: Option<String>,
    pub equipped_gloves: Option<String>,
    pub equipped_necklace: Option<String>,
    pub equipped_belt: Option<String>,
    // Admin privileges
    pub is_admin: bool,
    pub is_god_mode: bool, // Invincibility for admins
}

const PLAYER_RESPAWN_TIME_MS: u64 = 5000; // 5 seconds to respawn

impl Player {
    pub fn new(id: &str, name: &str, spawn_x: i32, spawn_y: i32, gender: &str, skin: &str) -> Self {
        let skills = Skills::new(); // HP 10, Attack/Strength/Defence 1
        Self {
            id: id.to_string(),
            name: name.to_string(),
            x: spawn_x,
            y: spawn_y,
            spawn_x,
            spawn_y,
            move_dx: 0,
            move_dy: 0,
            last_move_tick: 0,
            direction: Direction::Down,
            hp: skills.hitpoints.level, // HP = Hitpoints level
            skills,
            active: false,
            target_id: None,
            last_attack_time: 0,
            is_dead: false,
            death_time: 0,
            inventory: Inventory::new(),
            gender: gender.to_string(),
            skin: skin.to_string(),
            equipped_head: None,
            equipped_body: None,
            equipped_weapon: None,
            equipped_back: None,
            equipped_feet: None,
            equipped_ring: None,
            equipped_gloves: None,
            equipped_necklace: None,
            equipped_belt: None,
            is_admin: false,
            is_god_mode: false,
        }
    }

    /// Max HP is determined by Hitpoints skill level
    pub fn max_hp(&self) -> i32 {
        self.skills.hitpoints.level
    }

    /// Combat level calculated from all combat skills
    pub fn combat_level(&self) -> i32 {
        self.skills.combat_level()
    }

    /// Get all equipped item IDs for stat calculation
    fn all_equipped(&self) -> [&Option<String>; 9] {
        [
            &self.equipped_head,
            &self.equipped_body,
            &self.equipped_weapon,
            &self.equipped_back,
            &self.equipped_feet,
            &self.equipped_ring,
            &self.equipped_gloves,
            &self.equipped_necklace,
            &self.equipped_belt,
        ]
    }

    /// Calculate total attack bonus (accuracy) from equipped items
    pub fn attack_bonus(&self, item_registry: &ItemRegistry) -> i32 {
        let mut bonus = 0;
        for equipped in self.all_equipped() {
            if let Some(item_id) = equipped {
                if let Some(def) = item_registry.get(item_id) {
                    if let Some(equip) = &def.equipment {
                        bonus += equip.attack_bonus;
                    }
                }
            }
        }
        bonus
    }

    /// Calculate total strength bonus (max hit) from equipped items
    pub fn strength_bonus(&self, item_registry: &ItemRegistry) -> i32 {
        let mut bonus = 0;
        for equipped in self.all_equipped() {
            if let Some(item_id) = equipped {
                if let Some(def) = item_registry.get(item_id) {
                    if let Some(equip) = &def.equipment {
                        bonus += equip.strength_bonus;
                    }
                }
            }
        }
        bonus
    }

    /// Calculate total defence bonus from equipped items
    pub fn defence_bonus(&self, item_registry: &ItemRegistry) -> i32 {
        let mut bonus = 0;
        for equipped in self.all_equipped() {
            if let Some(item_id) = equipped {
                if let Some(def) = item_registry.get(item_id) {
                    if let Some(equip) = &def.equipment {
                        bonus += equip.defence_bonus;
                    }
                }
            }
        }
        bonus
    }

    /// Award combat XP based on damage dealt.
    /// Combat skill gets 4 XP per damage.
    /// Hitpoints gets 1.33 XP per damage (1/3 of combat rate).
    /// Returns a vector of (SkillType, xp_gained, total_xp, level, leveled_up) for skills that gained XP.
    pub fn award_combat_xp(&mut self, damage: i32) -> Vec<(SkillType, i64, i64, i32, bool)> {
        use crate::skills::{COMBAT_XP_PER_DAMAGE, HITPOINTS_XP_PER_DAMAGE};

        let mut results = Vec::new();

        // Combat XP = 4 per damage (full amount to single Combat skill)
        let combat_xp = (damage as f64 * COMBAT_XP_PER_DAMAGE) as i64;
        // Hitpoints XP = 1.33 per damage
        let hp_xp = (damage as f64 * HITPOINTS_XP_PER_DAMAGE) as i64;

        // Award Combat XP
        let combat_leveled = self.skills.combat.add_xp(combat_xp);
        if combat_leveled {
            tracing::info!("{} leveled up Combat to {}!", self.name, self.skills.combat.level);
        }
        results.push((SkillType::Combat, combat_xp, self.skills.combat.xp, self.skills.combat.level, combat_leveled));

        // Award Hitpoints XP
        let old_hp_level = self.skills.hitpoints.level;
        let hp_leveled = self.skills.hitpoints.add_xp(hp_xp);
        if hp_leveled {
            // Hitpoints level up means max HP increased
            let new_max = self.skills.hitpoints.level;
            tracing::info!("{} leveled up Hitpoints to {}! (Max HP: {})", self.name, new_max, new_max);
            // Heal the difference (new levels worth of HP)
            self.hp += new_max - old_hp_level;
        }
        results.push((SkillType::Hitpoints, hp_xp, self.skills.hitpoints.xp, self.skills.hitpoints.level, hp_leveled));

        results
    }

    pub fn is_alive(&self) -> bool {
        !self.is_dead && self.hp > 0
    }

    pub fn die(&mut self, current_time: u64) {
        self.is_dead = true;
        self.death_time = current_time;
        self.hp = 0;
        self.move_dx = 0;
        self.move_dy = 0;
        self.target_id = None;
    }

    pub fn ready_to_respawn(&self, current_time: u64) -> bool {
        self.is_dead && (current_time - self.death_time >= PLAYER_RESPAWN_TIME_MS)
    }

    pub fn respawn(&mut self) {
        self.x = self.spawn_x;
        self.y = self.spawn_y;
        self.hp = self.max_hp(); // Use method since max_hp is now derived from skills
        self.is_dead = false;
        self.death_time = 0;
        self.target_id = None;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerUpdate {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub direction: u8,
    // Velocity for client-side prediction (-1, 0, or 1)
    pub vel_x: i32,
    pub vel_y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub combat_level: i32,
    // Individual skill levels
    pub hitpoints_level: i32,
    pub combat_skill_level: i32,
    pub gold: i32,
    // Character appearance
    pub gender: String,
    pub skin: String,
    // Equipment
    pub equipped_head: Option<String>,
    pub equipped_body: Option<String>,
    pub equipped_weapon: Option<String>,
    pub equipped_back: Option<String>,
    pub equipped_feet: Option<String>,
    pub equipped_ring: Option<String>,
    pub equipped_gloves: Option<String>,
    pub equipped_necklace: Option<String>,
    pub equipped_belt: Option<String>,
    // Admin status
    pub is_admin: bool,
}

// ============================================================================
// Game Room
// ============================================================================

pub struct GameRoom {
    pub id: String,
    pub name: String,
    players: RwLock<HashMap<String, Player>>,
    npcs: RwLock<HashMap<String, Npc>>,
    ground_items: RwLock<HashMap<String, GroundItem>>,
    world: Arc<World>,
    /// Entity prototype registry for spawning and loot
    entity_registry: Arc<EntityRegistry>,
    /// Quest registry for quest definitions
    quest_registry: Arc<QuestRegistry>,
    /// Quest script runner for Lua execution
    quest_runner: Arc<QuestRunner>,
    /// Per-player quest state
    player_quest_states: RwLock<HashMap<String, PlayerQuestState>>,
    /// Crafting recipe registry
    crafting_registry: Arc<crate::crafting::CraftingRegistry>,
    /// Item definition registry
    item_registry: Arc<ItemRegistry>,
    /// Shop registry for merchant NPCs
    shop_registry: RwLock<ShopRegistry>,
    /// Last time shops were restocked
    last_shop_restock: RwLock<std::time::Instant>,
    /// Track which chunk each player is in for streaming updates
    player_chunks: RwLock<HashMap<String, ChunkCoord>>,
    tick: RwLock<u64>,
    broadcast_tx: broadcast::Sender<ServerMessage>,
    /// Per-player message senders for unicast (SECURITY: private inventory updates)
    player_senders: RwLock<HashMap<String, mpsc::Sender<Vec<u8>>>>,
}

impl GameRoom {
    pub async fn new(
        name: &str,
        entity_registry: Arc<EntityRegistry>,
        quest_registry: Arc<QuestRegistry>,
        crafting_registry: Arc<crate::crafting::CraftingRegistry>,
        item_registry: Arc<ItemRegistry>,
    ) -> Self {
        let (tx, _) = broadcast::channel(256);
        let world = Arc::new(World::new("maps/world_0"));

        // Create quest runner with the registry
        let quest_runner = Arc::new(QuestRunner::new(quest_registry.clone()));

        // Load initial chunk and spawn NPCs from entity_spawns
        let mut npcs = HashMap::new();
        let mut npc_counter = 0u32;

        // Load chunk (0, 0) which contains initial spawns
        if let Some(chunk) = world.get_or_load_chunk(crate::chunk::ChunkCoord::new(0, 0)).await {
            for spawn in &chunk.entity_spawns {
                let npc_id = spawn.unique_id.clone()
                    .unwrap_or_else(|| format!("npc_{}", npc_counter));
                npc_counter += 1;

                let npc = if let Some(prototype) = entity_registry.get(&spawn.entity_id) {
                    // Spawn from prototype
                    tracing::info!(
                        "Spawning {} at ({}, {}) level {}",
                        spawn.entity_id, spawn.world_x, spawn.world_y, spawn.level
                    );
                    Npc::from_prototype(
                        &npc_id,
                        &spawn.entity_id,
                        prototype,
                        spawn.world_x,
                        spawn.world_y,
                        spawn.level,
                    )
                } else {
                    // Fallback to legacy NpcType if prototype not found
                    tracing::warn!("Prototype '{}' not found, using fallback", spawn.entity_id);
                    Npc::new(&npc_id, NpcType::Slime, spawn.world_x, spawn.world_y, spawn.level)
                };
                npcs.insert(npc_id, npc);
            }
        }

        tracing::info!("Spawned {} NPCs from chunk entity_spawns", npcs.len());

        // Load shop registry
        let mut shop_registry = ShopRegistry::new();
        if let Err(e) = shop_registry.load_from_directory(std::path::Path::new("data/shops")) {
            tracing::error!("Failed to load shop registry: {}", e);
        }
        tracing::info!("Loaded {} shop definitions", shop_registry.len());

        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            players: RwLock::new(HashMap::new()),
            npcs: RwLock::new(npcs),
            ground_items: RwLock::new(HashMap::new()),
            world,
            entity_registry,
            quest_registry,
            quest_runner,
            player_quest_states: RwLock::new(HashMap::new()),
            crafting_registry,
            item_registry,
            shop_registry: RwLock::new(shop_registry),
            last_shop_restock: RwLock::new(std::time::Instant::now()),
            player_chunks: RwLock::new(HashMap::new()),
            tick: RwLock::new(0),
            broadcast_tx: tx,
            player_senders: RwLock::new(HashMap::new()),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }

    pub async fn broadcast(&self, msg: ServerMessage) {
        // Ignore send errors (no receivers)
        let _ = self.broadcast_tx.send(msg);
    }

    /// Register a player's message sender for unicast
    pub async fn register_player_sender(&self, player_id: &str, sender: mpsc::Sender<Vec<u8>>) {
        let mut senders = self.player_senders.write().await;
        senders.insert(player_id.to_string(), sender);
        tracing::debug!("Registered sender for player {}", player_id);
    }

    /// Unregister a player's message sender
    pub async fn unregister_player_sender(&self, player_id: &str) {
        let mut senders = self.player_senders.write().await;
        senders.remove(player_id);
        tracing::debug!("Unregistered sender for player {}", player_id);
    }

    /// Send a message to a specific player (unicast)
    /// SECURITY: Use this for private data like inventory updates
    pub async fn send_to_player(&self, player_id: &str, msg: ServerMessage) {
        use crate::protocol::encode_server_message;

        let senders = self.player_senders.read().await;
        if let Some(sender) = senders.get(player_id) {
            if let Ok(bytes) = encode_server_message(&msg) {
                if let Err(e) = sender.try_send(bytes) {
                    tracing::warn!("Failed to send unicast to {}: {}", player_id, e);
                }
            }
        } else {
            tracing::debug!("No sender registered for player {}", player_id);
        }
    }

    pub async fn reserve_player(&self, player_id: &str, name: &str, gender: &str, skin: &str) {
        let (spawn_x, spawn_y) = self.world.get_spawn_position().await;
        let mut players = self.players.write().await;
        let player = Player::new(player_id, name, spawn_x, spawn_y, gender, skin);
        players.insert(player_id.to_string(), player);

        // Track player's starting chunk
        let chunk = ChunkCoord::from_world(spawn_x, spawn_y);
        let mut chunks = self.player_chunks.write().await;
        chunks.insert(player_id.to_string(), chunk);
    }

    /// Reserve player with saved data from database
    pub async fn reserve_player_with_data(
        &self,
        player_id: &str,
        name: &str,
        x: i32,
        y: i32,
        hp: i32,
        skills: Skills,
        gold: i32,
        inventory_json: &str,
        gender: &str,
        skin: &str,
        equipped_head: Option<String>,
        equipped_body: Option<String>,
        equipped_weapon: Option<String>,
        equipped_back: Option<String>,
        equipped_feet: Option<String>,
        equipped_ring: Option<String>,
        equipped_gloves: Option<String>,
        equipped_necklace: Option<String>,
        equipped_belt: Option<String>,
        is_admin: bool,
    ) {
        let mut players = self.players.write().await;
        let mut player = Player::new(player_id, name, x, y, gender, skin);

        // Restore saved stats
        player.hp = hp.min(skills.hitpoints.level); // Cap HP at max (hitpoints level)
        player.skills = skills;
        player.inventory.gold = gold;
        player.equipped_head = equipped_head;
        player.equipped_body = equipped_body;
        player.equipped_weapon = equipped_weapon;
        player.equipped_back = equipped_back;
        player.equipped_feet = equipped_feet;
        player.equipped_ring = equipped_ring;
        player.equipped_gloves = equipped_gloves;
        player.equipped_necklace = equipped_necklace;
        player.equipped_belt = equipped_belt;
        player.is_admin = is_admin;

        // Restore inventory from JSON - support both old (u8) and new (String) formats
        // Skip invalid slots (empty item_id or quantity <= 0) to prevent ghost items
        if let Ok(slots) = serde_json::from_str::<Vec<(usize, String, i32)>>(inventory_json) {
            // New format: (slot_idx, item_id, quantity)
            for (slot_idx, item_id, quantity) in slots {
                if slot_idx < player.inventory.slots.len() && !item_id.is_empty() && quantity > 0 {
                    player.inventory.slots[slot_idx] = Some(item::InventorySlot::new(item_id, quantity));
                }
            }
        } else if let Ok(slots) = serde_json::from_str::<Vec<(usize, u8, i32)>>(inventory_json) {
            // Legacy format: (slot_idx, item_type_u8, quantity) - migrate to string IDs
            for (slot_idx, item_type_u8, quantity) in slots {
                if slot_idx < player.inventory.slots.len() && quantity > 0 {
                    let item_id = match item_type_u8 {
                        0 => "health_potion",
                        1 => "mana_potion",
                        3 => "slime_core",
                        4 => "iron_ore",
                        5 => "goblin_ear",
                        _ => continue, // Skip unknown items (2 was gold, handled separately)
                    }.to_string();
                    player.inventory.slots[slot_idx] = Some(item::InventorySlot::new(item_id, quantity));
                }
            }
        }

        tracing::info!(
            "Restored player {} at ({}, {}) with {} HP, combat level {}, {} gold, appearance: {} {}",
            name, x, y, hp, player.combat_level(), gold, gender, skin
        );

        players.insert(player_id.to_string(), player);
    }

    pub async fn activate_player(&self, player_id: &str) -> String {
        let mut players = self.players.write().await;
        if let Some(player) = players.get_mut(player_id) {
            player.active = true;
            return player.name.clone();
        }
        "Unknown".to_string()
    }

    pub async fn remove_player(&self, player_id: &str) {
        let mut players = self.players.write().await;
        players.remove(player_id);
    }

    /// Get player data for saving to database
    pub async fn get_player_save_data(&self, player_id: &str) -> Option<PlayerSaveData> {
        let players = self.players.read().await;
        players.get(player_id).map(|p| {
            // Serialize inventory to JSON - new format with string item IDs
            // Filter out empty/invalid slots to prevent ghost items
            let inventory_slots: Vec<(usize, String, i32)> = p.inventory.slots
                .iter()
                .enumerate()
                .filter_map(|(idx, slot)| {
                    slot.as_ref()
                        .filter(|s| s.quantity > 0 && !s.item_id.is_empty())
                        .map(|s| (idx, s.item_id.clone(), s.quantity))
                })
                .collect();
            let inventory_json = serde_json::to_string(&inventory_slots).unwrap_or_else(|_| "[]".to_string());

            PlayerSaveData {
                x: p.x as f32,
                y: p.y as f32,
                hp: p.hp,
                skills: p.skills.clone(),
                gold: p.inventory.gold,
                inventory_json,
                gender: p.gender.clone(),
                skin: p.skin.clone(),
                equipped_head: p.equipped_head.clone(),
                equipped_body: p.equipped_body.clone(),
                equipped_weapon: p.equipped_weapon.clone(),
                equipped_back: p.equipped_back.clone(),
                equipped_feet: p.equipped_feet.clone(),
                equipped_ring: p.equipped_ring.clone(),
                equipped_gloves: p.equipped_gloves.clone(),
                equipped_necklace: p.equipped_necklace.clone(),
                equipped_belt: p.equipped_belt.clone(),
            }
        })
    }

    /// Initialize quest state for a player (called on join)
    pub async fn set_player_quest_state(&self, player_id: &str, state: PlayerQuestState) {
        let mut quest_states = self.player_quest_states.write().await;
        quest_states.insert(player_id.to_string(), state);
    }

    /// Get quest state for saving (called on disconnect/auto-save)
    pub async fn get_player_quest_state(&self, player_id: &str) -> Option<PlayerQuestState> {
        let quest_states = self.player_quest_states.read().await;
        quest_states.get(player_id).cloned()
    }

    /// Get QuestAccepted messages for all active quests (for syncing on login)
    pub async fn get_active_quest_messages(&self, player_id: &str) -> Vec<ServerMessage> {
        let quest_states = self.player_quest_states.read().await;
        let quest_state = match quest_states.get(player_id) {
            Some(state) => state,
            None => return Vec::new(),
        };

        let mut messages = Vec::new();
        for (quest_id, progress) in &quest_state.active_quests {
            if let Some(quest) = self.quest_registry.get(quest_id).await {
                let objectives: Vec<QuestObjectiveData> = quest.objectives
                    .iter()
                    .map(|o| {
                        // Get current progress from saved state
                        let (current, completed) = progress.objectives
                            .get(&o.id)
                            .map(|p| (p.current, p.completed))
                            .unwrap_or((0, false));
                        QuestObjectiveData {
                            id: o.id.clone(),
                            description: o.description.clone(),
                            current,
                            target: o.count,
                            completed,
                        }
                    })
                    .collect();
                messages.push(ServerMessage::QuestAccepted {
                    quest_id: quest_id.clone(),
                    quest_name: quest.name.clone(),
                    objectives,
                });
            }
        }
        messages
    }

    pub async fn player_count(&self) -> usize {
        let players = self.players.read().await;
        players.values().filter(|p| p.active).count()
    }

    pub async fn get_all_players(&self) -> Vec<Player> {
        let players = self.players.read().await;
        players.values().filter(|p| p.active).cloned().collect()
    }

    pub async fn get_player_position(&self, player_id: &str) -> Option<(i32, i32)> {
        let players = self.players.read().await;
        players.get(player_id).map(|p| (p.x, p.y))
    }

    pub async fn get_player_appearance(&self, player_id: &str) -> Option<(String, String)> {
        let players = self.players.read().await;
        players.get(player_id).map(|p| (p.gender.clone(), p.skin.clone()))
    }

    /// Get the initial inventory update message for a player (used on connection)
    pub async fn get_player_inventory_update(&self, player_id: &str) -> Option<ServerMessage> {
        let players = self.players.read().await;
        players.get(player_id).map(|p| ServerMessage::InventoryUpdate {
            player_id: player_id.to_string(),
            slots: p.inventory.to_update(),
            gold: p.inventory.gold,
        })
    }

    pub async fn get_all_npcs(&self) -> Vec<Npc> {
        let npcs = self.npcs.read().await;
        npcs.values().cloned().collect()
    }

    /// Spawn an NPC at a specific location (admin command)
    pub async fn spawn_npc_at(&self, prototype_id: &str, x: f32, y: f32) -> String {
        let npc_id = format!("admin_npc_{}", Uuid::new_v4());

        let npc = if let Some(prototype) = self.entity_registry.get(prototype_id) {
            Npc::from_prototype(
                &npc_id,
                prototype_id,
                prototype,
                x as i32,
                y as i32,
                1, // Default level
            )
        } else {
            // Fallback to legacy NpcType
            Npc::new(&npc_id, NpcType::Slime, x as i32, y as i32, 1)
        };

        let mut npcs = self.npcs.write().await;
        npcs.insert(npc_id.clone(), npc);
        tracing::info!("Admin spawned NPC {} at ({}, {})", prototype_id, x, y);
        npc_id
    }

    pub async fn handle_move(&self, player_id: &str, dx: f32, dy: f32) {
        let mut players = self.players.write().await;
        if let Some(player) = players.get_mut(player_id) {
            // Convert to grid movement (-1, 0, or 1)
            // No diagonal movement in grid-based system
            let move_dx: i32;
            let move_dy: i32;

            if dx.abs() > dy.abs() {
                // Horizontal priority
                move_dx = if dx > 0.1 { 1 } else if dx < -0.1 { -1 } else { 0 };
                move_dy = 0;
            } else if dy.abs() > 0.1 {
                // Vertical priority
                move_dx = 0;
                move_dy = if dy > 0.1 { 1 } else if dy < -0.1 { -1 } else { 0 };
            } else {
                move_dx = 0;
                move_dy = 0;
            }

            player.move_dx = move_dx;
            player.move_dy = move_dy;

            if move_dx != 0 || move_dy != 0 {
                player.direction = Direction::from_velocity(move_dx as f32, move_dy as f32);
            }
        }
    }

    /// Handle face command - change direction without moving
    pub async fn handle_face(&self, player_id: &str, direction: u8) {
        tracing::info!("[SERVER] handle_face called: player_id={}, direction={}", player_id, direction);
        let mut players = self.players.write().await;
        if let Some(player) = players.get_mut(player_id) {
            let old_dir = player.direction;
            player.direction = Direction::from_u8(direction);
            tracing::info!("[SERVER] Updated player direction: {:?} -> {:?}", old_dir, player.direction);
            // Ensure player is not moving when just facing
            player.move_dx = 0;
            player.move_dy = 0;
        } else {
            tracing::warn!("[SERVER] handle_face: player not found: {}", player_id);
        }
    }

    pub async fn handle_chat(&self, player_id: &str, text: &str) {
        let sanitized = text.trim().chars().take(200).collect::<String>();
        if sanitized.is_empty() {
            return;
        }

        // Check for commands (messages starting with /)
        if sanitized.starts_with('/') {
            self.handle_chat_command(player_id, &sanitized).await;
            return;
        }

        // Regular chat message
        let players = self.players.read().await;
        if let Some(player) = players.get(player_id) {
            let msg = ServerMessage::ChatMessage {
                sender_id: player_id.to_string(),
                sender_name: player.name.clone(),
                text: sanitized,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            };
            drop(players); // Release lock before broadcast
            self.broadcast(msg).await;
        }
    }

    /// Handle chat commands (messages starting with /)
    async fn handle_chat_command(&self, player_id: &str, text: &str) {
        let parts: Vec<&str> = text.split_whitespace().collect();
        let command = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

        // Check if player is admin for privileged commands
        let is_admin = {
            let players = self.players.read().await;
            players.get(player_id).map(|p| p.is_admin).unwrap_or(false)
        };

        // Admin-only commands check
        let admin_commands = ["/give", "/setlevel", "/teleport", "/spawn", "/heal", "/kill", "/god", "/announce"];
        if admin_commands.contains(&command.as_str()) && !is_admin {
            self.send_system_message(player_id, "This command requires admin privileges.").await;
            return;
        }

        match command.as_str() {
            "/give" => {
                // /give item_id [quantity]
                if parts.len() < 2 {
                    self.send_system_message(player_id, "Usage: /give <item_id> [quantity]").await;
                    return;
                }

                let item_id = parts[1];
                let quantity = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);

                // Validate item exists
                if self.item_registry.get(item_id).is_none() {
                    self.send_system_message(player_id, &format!("Unknown item: {}", item_id)).await;
                    return;
                }

                // Add item to player's inventory
                // add_item returns the quantity that DIDN'T fit (0 = all added successfully)
                let (leftover, inventory_update, gold) = {
                    let mut players = self.players.write().await;
                    if let Some(player) = players.get_mut(player_id) {
                        let leftover = player.inventory.add_item(item_id, quantity, &self.item_registry);
                        (leftover, player.inventory.to_update(), player.inventory.gold)
                    } else {
                        (quantity, vec![], 0)
                    }
                };

                let added = quantity - leftover;
                if added > 0 {
                    tracing::info!("Player {} spawned {}x {}", player_id, added, item_id);
                    self.send_system_message(player_id, &format!("Gave {}x {}", added, item_id)).await;

                    // Send inventory update
                    self.send_to_player(player_id, ServerMessage::InventoryUpdate {
                        player_id: player_id.to_string(),
                        slots: inventory_update,
                        gold,
                    }).await;
                } else {
                    self.send_system_message(player_id, "Inventory full").await;
                }
            }
            "/setlevel" => {
                // /setlevel <level> - Sets all combat skills to the given level
                use crate::skills::Skill;

                if parts.len() < 2 {
                    self.send_system_message(player_id, "Usage: /setlevel <level>").await;
                    return;
                }

                let level: i32 = match parts[1].parse() {
                    Ok(l) if l >= 1 && l <= 99 => l,
                    _ => {
                        self.send_system_message(player_id, "Level must be between 1 and 99").await;
                        return;
                    }
                };

                // Update combat skills and heal to full
                {
                    let mut players = self.players.write().await;
                    if let Some(player) = players.get_mut(player_id) {
                        player.skills.hitpoints = Skill::new(level);
                        player.skills.combat = Skill::new(level);
                        player.hp = player.max_hp(); // Heal to full
                        tracing::info!("Player {} set all skills to level {}", player_id, level);
                    } else {
                        return;
                    }
                };

                let combat_level = (level + level) / 2; // (HP + Combat) / 2
                self.send_system_message(player_id, &format!("Skills set to level {} (Combat Level: {})", level, combat_level)).await;
                // TODO: Phase 5 will add proper skill level broadcast messages
            }
            "/help" => {
                if is_admin {
                    self.send_system_message(player_id, "Commands: /give <item> [qty], /setlevel <lvl>, /teleport <x> <y>, /spawn <npc> [x] [y], /heal [player], /kill <player>, /god, /announce <msg>, /items, /help").await;
                } else {
                    self.send_system_message(player_id, "Commands: /items, /help").await;
                }
            }
            "/items" => {
                // List available items
                let items: Vec<&String> = self.item_registry.ids().collect();
                let list = items.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                self.send_system_message(player_id, &format!("Items: {}", list)).await;
            }
            "/teleport" => {
                // /teleport <x> <y>
                if parts.len() < 3 {
                    self.send_system_message(player_id, "Usage: /teleport <x> <y>").await;
                    return;
                }
                let x: i32 = match parts[1].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.send_system_message(player_id, "Invalid x coordinate").await;
                        return;
                    }
                };
                let y: i32 = match parts[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.send_system_message(player_id, "Invalid y coordinate").await;
                        return;
                    }
                };
                {
                    let mut players = self.players.write().await;
                    if let Some(player) = players.get_mut(player_id) {
                        player.x = x;
                        player.y = y;
                        tracing::info!("Player {} teleported to ({}, {})", player_id, x, y);
                    }
                }
                self.send_system_message(player_id, &format!("Teleported to ({}, {})", x, y)).await;
            }
            "/spawn" => {
                // /spawn <npc_id> [x] [y]
                if parts.len() < 2 {
                    self.send_system_message(player_id, "Usage: /spawn <npc_id> [x] [y]").await;
                    return;
                }
                let npc_id = parts[1];

                // Get spawn position (player position if not specified)
                let (spawn_x, spawn_y) = {
                    let players = self.players.read().await;
                    if let Some(player) = players.get(player_id) {
                        let x = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(player.x);
                        let y = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(player.y);
                        (x, y)
                    } else {
                        return;
                    }
                };

                // Check if NPC type exists
                if self.entity_registry.get(npc_id).is_none() {
                    let available: Vec<&String> = self.entity_registry.ids().collect();
                    let list = available.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                    self.send_system_message(player_id, &format!("Unknown NPC: {}. Available: {}", npc_id, list)).await;
                    return;
                }

                // Spawn the NPC
                let spawned_id = self.spawn_npc_at(npc_id, spawn_x as f32, spawn_y as f32).await;
                self.send_system_message(player_id, &format!("Spawned {} at ({}, {}) [id: {}]", npc_id, spawn_x, spawn_y, spawned_id)).await;
                tracing::info!("Admin {} spawned {} at ({}, {})", player_id, npc_id, spawn_x, spawn_y);
            }
            "/heal" => {
                // /heal [player_name]
                let target_name = parts.get(1).map(|s| *s);

                let healed = {
                    let mut players = self.players.write().await;
                    if let Some(name) = target_name {
                        // Find player by name
                        if let Some(player) = players.values_mut().find(|p| p.name.eq_ignore_ascii_case(name)) {
                            player.hp = player.max_hp();
                            player.is_dead = false;
                            Some(player.name.clone())
                        } else {
                            None
                        }
                    } else {
                        // Heal self
                        if let Some(player) = players.get_mut(player_id) {
                            player.hp = player.max_hp();
                            player.is_dead = false;
                            Some(player.name.clone())
                        } else {
                            None
                        }
                    }
                };

                match healed {
                    Some(name) => self.send_system_message(player_id, &format!("Healed {} to full HP", name)).await,
                    None => self.send_system_message(player_id, "Player not found").await,
                }
            }
            "/kill" => {
                // /kill <player_name>
                if parts.len() < 2 {
                    self.send_system_message(player_id, "Usage: /kill <player_name>").await;
                    return;
                }
                let target_name = parts[1];

                let killed = {
                    let mut players = self.players.write().await;
                    if let Some(player) = players.values_mut().find(|p| p.name.eq_ignore_ascii_case(target_name)) {
                        player.hp = 0;
                        player.is_dead = true;
                        player.death_time = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;
                        Some(player.name.clone())
                    } else {
                        None
                    }
                };

                match killed {
                    Some(name) => {
                        self.send_system_message(player_id, &format!("Killed {}", name)).await;
                        tracing::info!("Admin {} killed player {}", player_id, name);
                    }
                    None => self.send_system_message(player_id, "Player not found").await,
                }
            }
            "/god" => {
                // Toggle god mode (invincibility)
                let (enabled, player_name) = {
                    let mut players = self.players.write().await;
                    if let Some(player) = players.get_mut(player_id) {
                        player.is_god_mode = !player.is_god_mode;
                        (player.is_god_mode, player.name.clone())
                    } else {
                        return;
                    }
                };
                let status = if enabled { "enabled" } else { "disabled" };
                self.send_system_message(player_id, &format!("God mode {}", status)).await;
                tracing::info!("Admin {} ({}) toggled god mode: {}", player_name, player_id, status);
            }
            "/announce" => {
                // /announce <message>
                if parts.len() < 2 {
                    self.send_system_message(player_id, "Usage: /announce <message>").await;
                    return;
                }
                let message = parts[1..].join(" ");

                // Broadcast announcement to all players
                self.broadcast(ServerMessage::Announcement {
                    text: message.clone(),
                }).await;
                tracing::info!("Admin {} announced: {}", player_id, message);
            }
            _ => {
                self.send_system_message(player_id, &format!("Unknown command: {}. Try /help", command)).await;
            }
        }
    }

    /// Send a system message to a specific player
    async fn send_system_message(&self, player_id: &str, text: &str) {
        let msg = ServerMessage::ChatMessage {
            sender_id: "system".to_string(),
            sender_name: "[System]".to_string(),
            text: text.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        self.send_to_player(player_id, msg).await;
    }

    pub async fn handle_attack(&self, player_id: &str) {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Get attacker info including combat stats
        let (attacker_name, attacker_x, attacker_y, attacker_dir, last_attack,
             combat_level, attack_bonus, strength_bonus) = {
            let players = self.players.read().await;
            let player = match players.get(player_id) {
                Some(p) => p,
                None => {
                    tracing::warn!("Attack failed: player {} not found", player_id);
                    return;
                }
            };

            // Dead players can't attack
            if player.is_dead {
                return;
            }

            let atk_bonus = player.attack_bonus(&self.item_registry);
            let str_bonus = player.strength_bonus(&self.item_registry);
            (
                player.name.clone(),
                player.x, player.y, player.direction,
                player.last_attack_time,
                player.skills.combat.level,
                atk_bonus,
                str_bonus,
            )
        };

        // Check cooldown
        if current_time - last_attack < ATTACK_COOLDOWN_MS {
            tracing::info!("[ATTACK] Cooldown not met: current_time={}, last_attack={}, ATTACK_COOLDOWN_MS={}", current_time, last_attack, ATTACK_COOLDOWN_MS);
            return;
        }

        // Calculate the tile position in front of the player based on facing direction
        let (front_x, front_y) = match attacker_dir {
            Direction::Up => (attacker_x, attacker_y - 1),
            Direction::Down => (attacker_x, attacker_y + 1),
            Direction::Left => (attacker_x - 1, attacker_y),
            Direction::Right => (attacker_x + 1, attacker_y),
            Direction::UpLeft => (attacker_x - 1, attacker_y - 1),
            Direction::UpRight => (attacker_x + 1, attacker_y - 1),
            Direction::DownLeft => (attacker_x - 1, attacker_y + 1),
            Direction::DownRight => (attacker_x + 1, attacker_y + 1),
        };

        tracing::info!("{} attacks toward ({}, {}) facing {:?}", attacker_name, front_x, front_y, attacker_dir);

        // Find target at the front tile - check NPCs first, then players
        let mut target_id: Option<String> = None;
        let mut is_npc = false;

        // Check NPCs
        {
            let npcs = self.npcs.read().await;
            for (npc_id, npc) in npcs.iter() {
                if npc.is_alive() && npc.x == front_x && npc.y == front_y {
                    target_id = Some(npc_id.clone());
                    is_npc = true;
                    tracing::info!("{} found NPC target: {} at ({}, {})", attacker_name, npc.name(), npc.x, npc.y);
                    break;
                }
            }
        }

        // Check players if no NPC found
        if target_id.is_none() {
            let players = self.players.read().await;
            for (pid, player) in players.iter() {
                if pid != player_id && player.active && player.hp > 0 && player.x == front_x && player.y == front_y {
                    target_id = Some(pid.clone());
                    is_npc = false;
                    tracing::info!("{} found player target: {} at ({}, {})", attacker_name, player.name, player.x, player.y);
                    break;
                }
            }
        }

        // No valid target found
        let target_id = match target_id {
            Some(id) => id,
            None => {
                tracing::debug!("{} attack missed - no target at ({}, {})", attacker_name, front_x, front_y);
                return;
            }
        };

        // Update attacker's last attack time and stop movement
        {
            let mut players = self.players.write().await;
            if let Some(player) = players.get_mut(player_id) {
                player.last_attack_time = current_time;
                // Stop movement when attacking (player must stand still to attack)
                player.move_dx = 0;
                player.move_dy = 0;
            }
        }

        // Apply damage to target using hit/miss mechanics
        // 1. Roll attack vs defence to determine if we hit
        // 2. If hit, calculate max hit from strength and roll damage
        let (target_hp, target_name, target_died, actual_damage) = if is_npc {
            // NPCs use their level as both attack and defence level (no equipment bonuses)
            let mut npcs = self.npcs.write().await;
            if let Some(npc) = npcs.get_mut(&target_id) {
                // NPC's defence = level, no equipment bonus
                let npc_defence_level = npc.level;
                let npc_defence_bonus = 0;

                // Check if attack hits (combat_level used for both attack and strength)
                if !calculate_hit(combat_level, attack_bonus, npc_defence_level, npc_defence_bonus) {
                    // Miss - deal 0 damage
                    let name = npc.name();
                    tracing::info!(
                        "{} misses {} (atk {} + {} vs def {} + {})",
                        attacker_name, name, combat_level, attack_bonus, npc_defence_level, npc_defence_bonus
                    );
                    (npc.hp, name, false, 0)
                } else {
                    // Hit - calculate and apply damage
                    let max_hit = calculate_max_hit(combat_level, strength_bonus);
                    let damage = roll_damage(max_hit);
                    let died = npc.take_damage(damage, current_time);
                    let name = npc.name();
                    tracing::info!(
                        "{} hits {} for {} damage (max: {}, HP: {})",
                        attacker_name, name, damage, max_hit, npc.hp
                    );
                    (npc.hp, name, died, damage)
                }
            } else {
                return;
            }
        } else {
            // Players have defence from skills and equipment
            let mut players = self.players.write().await;
            if let Some(target) = players.get_mut(&target_id) {
                if target.is_dead {
                    return; // Already dead
                }
                // God mode prevents all damage
                if target.is_god_mode {
                    return;
                }

                // Get target's defence stats (uses combat level for defence)
                let target_combat_level = target.skills.combat.level;
                let target_defence_bonus = target.defence_bonus(&self.item_registry);

                // Check if attack hits
                if !calculate_hit(combat_level, attack_bonus, target_combat_level, target_defence_bonus) {
                    // Miss - deal 0 damage
                    let name = target.name.clone();
                    tracing::info!(
                        "{} misses {} (cmb {} + {} vs cmb {} + {})",
                        attacker_name, name, combat_level, attack_bonus, target_combat_level, target_defence_bonus
                    );
                    (target.hp, name, false, 0)
                } else {
                    // Hit - calculate and apply damage
                    let max_hit = calculate_max_hit(combat_level, strength_bonus);
                    let damage = roll_damage(max_hit);
                    target.hp = (target.hp - damage).max(0);
                    let name = target.name.clone();
                    let died = target.hp <= 0;
                    if died {
                        target.die(current_time);
                    }
                    tracing::info!(
                        "{} hits {} for {} damage (max: {}, HP: {})",
                        attacker_name, name, damage, max_hit, target.hp
                    );
                    (target.hp, name, died, damage)
                }
            } else {
                return;
            }
        };

        // Use front position as target position for damage event
        let target_x = front_x as f32;
        let target_y = front_y as f32;

        // Broadcast damage event to all clients
        let damage_msg = ServerMessage::DamageEvent {
            source_id: player_id.to_string(),
            target_id: target_id.clone(),
            damage: actual_damage,
            target_hp,
            target_x,
            target_y,
        };
        self.broadcast(damage_msg).await;

        // Send success result to attacker
        let result_msg = ServerMessage::AttackResult {
            success: true,
            reason: None,
        };
        self.broadcast(result_msg).await;

        // Handle death
        if target_died {
            tracing::info!("{} killed {}", attacker_name, target_name);
            if is_npc {
                // Get NPC info for exp and loot
                let (prototype_id, npc_level, npc_type) = {
                    let npcs = self.npcs.read().await;
                    npcs.get(&target_id)
                        .map(|n| (n.prototype_id.clone(), n.level, n.npc_type))
                        .unwrap_or((None, 1, NpcType::Slime))
                };

                // Calculate EXP reward - use prototype if available
                let exp_reward = if let Some(ref proto_id) = prototype_id {
                    if let Some(prototype) = self.entity_registry.get(proto_id) {
                        crate::entity::calculate_exp_reward(prototype, npc_level)
                    } else {
                        npc_type.stats().exp_reward * npc_level
                    }
                } else {
                    npc_type.stats().exp_reward * npc_level
                };

                // Award combat XP to killer based on damage dealt
                // Use exp_reward as a proxy for "damage" in XP calculation
                let xp_results: Option<Vec<(SkillType, i64, i64, i32, bool)>> = if exp_reward > 0 {
                    let mut players = self.players.write().await;
                    if let Some(killer) = players.get_mut(player_id) {
                        Some(killer.award_combat_xp(exp_reward))
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Send skill XP and level-up messages (after releasing write lock)
                if let Some(results) = xp_results {
                    for (skill_type, xp_gained, total_xp, level, leveled_up) in results {
                        // Send XP gain message
                        self.send_to_player(player_id, ServerMessage::SkillXp {
                            player_id: player_id.to_string(),
                            skill: skill_type.as_str().to_string(),
                            xp_gained,
                            total_xp,
                            level,
                        }).await;

                        // Send level-up message if applicable
                        if leveled_up {
                            tracing::info!("Player {} leveled up {} to {}", player_id, skill_type.as_str(), level);
                            self.broadcast(ServerMessage::SkillLevelUp {
                                player_id: player_id.to_string(),
                                skill: skill_type.as_str().to_string(),
                                new_level: level,
                            }).await;
                        }
                    }
                }

                // Broadcast NPC death
                let death_msg = ServerMessage::NpcDied {
                    id: target_id.clone(),
                    killer_id: player_id.to_string(),
                };
                self.broadcast(death_msg).await;

                // Process quest kill event
                let entity_type = prototype_id.clone().unwrap_or_else(|| npc_type.stats().name.to_string());
                self.process_quest_kill(player_id, &entity_type).await;

                // Spawn item drops - use prototype loot table if available
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let drops = if let Some(ref proto_id) = prototype_id {
                    if let Some(prototype) = self.entity_registry.get(proto_id) {
                        crate::entity::generate_loot_from_prototype(
                            prototype, target_x, target_y, player_id, current_time, npc_level
                        )
                    } else {
                        item::generate_drops(npc_type, target_x, target_y, player_id, current_time)
                    }
                } else {
                    item::generate_drops(npc_type, target_x, target_y, player_id, current_time)
                };

                for item in drops {
                    let mut items = self.ground_items.write().await;

                    // For gold, try to combine with existing pile at same tile
                    if item.item_id == "gold" {
                        let tile_x = item.x.floor() as i32;
                        let tile_y = item.y.floor() as i32;

                        // Find existing gold at same tile with same owner
                        let existing_gold_id = items.iter()
                            .find(|(_, existing)| {
                                existing.item_id == "gold"
                                    && existing.x.floor() as i32 == tile_x
                                    && existing.y.floor() as i32 == tile_y
                                    && existing.owner_id == item.owner_id
                            })
                            .map(|(id, _)| id.clone());

                        if let Some(existing_id) = existing_gold_id {
                            // Combine with existing pile
                            if let Some(existing) = items.get_mut(&existing_id) {
                                existing.quantity += item.quantity;
                                let update_msg = ServerMessage::ItemQuantityUpdated {
                                    id: existing_id.clone(),
                                    quantity: existing.quantity,
                                };
                                drop(items); // Release lock before broadcast
                                self.broadcast(update_msg).await;
                            }
                            continue;
                        }
                    }

                    // No existing pile to combine with - create new item
                    let drop_msg = ServerMessage::ItemDropped {
                        id: item.id.clone(),
                        item_id: item.item_id.clone(),
                        x: item.x,
                        y: item.y,
                        quantity: item.quantity,
                    };
                    items.insert(item.id.clone(), item);
                    drop(items); // Release lock before broadcast
                    self.broadcast(drop_msg).await;
                }
            } else {
                // Broadcast player death
                let death_msg = ServerMessage::PlayerDied {
                    id: target_id.clone(),
                    killer_id: player_id.to_string(),
                };
                self.broadcast(death_msg).await;
            }
        }
    }

    /// Process quest kill event
    async fn process_quest_kill(&self, player_id: &str, entity_type: &str) {
        // Get player's quest state
        let mut quest_states = self.player_quest_states.write().await;
        let quest_state = quest_states.entry(player_id.to_string())
            .or_insert_with(PlayerQuestState::new);

        // Create kill event
        let event = QuestEvent::MonsterKilled {
            player_id: player_id.to_string(),
            entity_type: entity_type.to_string(),
            level: 1, // TODO: Get actual monster level from context
        };

        // Process the event through the registry
        let results = self.quest_registry.process_event(&event, quest_state).await;

        // Handle results - send notifications to player
        for result in results {
            if let (Some(objective_id), Some(current), Some(target)) =
                (&result.objective_id, result.new_progress, result.target)
            {
                tracing::info!(
                    "Player {} progress on objective {} for quest {}: {}/{}",
                    player_id, objective_id, result.quest_id, current, target
                );

                // Send objective progress update to player
                if let Some(sender) = self.player_senders.read().await.get(player_id) {
                    let msg = ServerMessage::QuestObjectiveProgress {
                        quest_id: result.quest_id.clone(),
                        objective_id: objective_id.clone(),
                        current,
                        target,
                    };
                    if let Ok(data) = crate::protocol::encode_server_message(&msg) {
                        let _ = sender.send(data).await;
                    }
                }

                if result.objective_completed {
                    tracing::info!(
                        "Player {} completed objective {} for quest {}",
                        player_id, objective_id, result.quest_id
                    );
                }
            }

            if result.quest_ready {
                tracing::info!(
                    "Player {} quest {} is ready to complete!",
                    player_id, result.quest_id
                );
            }
        }
    }

    /// Process quest item collection event
    async fn process_quest_item_collect(&self, player_id: &str, item_id: &str, count: i32) {
        let mut quest_states = self.player_quest_states.write().await;
        let quest_state = quest_states.entry(player_id.to_string())
            .or_insert_with(PlayerQuestState::new);

        let event = QuestEvent::ItemCollected {
            player_id: player_id.to_string(),
            item_id: item_id.to_string(),
            count,
        };

        let results = self.quest_registry.process_event(&event, quest_state).await;

        for result in results {
            if let (Some(objective_id), Some(current), Some(target)) =
                (&result.objective_id, result.new_progress, result.target)
            {
                tracing::info!(
                    "Player {} collected quest item objective {} for quest {}: {}/{}",
                    player_id, objective_id, result.quest_id, current, target
                );

                if let Some(sender) = self.player_senders.read().await.get(player_id) {
                    let msg = ServerMessage::QuestObjectiveProgress {
                        quest_id: result.quest_id.clone(),
                        objective_id: objective_id.clone(),
                        current,
                        target,
                    };
                    if let Ok(data) = crate::protocol::encode_server_message(&msg) {
                        let _ = sender.send(data).await;
                    }
                }
            }
        }
    }

    /// Check if two directions are close enough (within 45 degrees)
    fn directions_match(dir1: Direction, dir2: Direction) -> bool {
        // Convert to numeric for comparison
        let d1 = dir1 as i32;
        let d2 = dir2 as i32;
        let diff = (d1 - d2).abs();
        // Directions match if they're the same or adjacent (with wraparound)
        diff <= 1 || diff == 7
    }

    pub async fn handle_target(&self, player_id: &str, target_id: &str) {
        tracing::info!("Target request: player {} -> target '{}'", player_id, target_id);

        // Validate target exists (can be player or NPC)
        let valid_target = {
            if target_id.is_empty() {
                true // Clear target
            } else if target_id == player_id {
                false // Can't target self
            } else {
                // Check if target is a player
                let players = self.players.read().await;
                let is_player = players.get(target_id).map(|p| p.active).unwrap_or(false);
                drop(players);

                if is_player {
                    true
                } else {
                    // Check if target is an NPC
                    let npcs = self.npcs.read().await;
                    npcs.get(target_id).map(|n| n.is_alive()).unwrap_or(false)
                }
            }
        };

        if valid_target {
            let mut players = self.players.write().await;
            if let Some(player) = players.get_mut(player_id) {
                let new_target = if target_id.is_empty() {
                    None
                } else {
                    Some(target_id.to_string())
                };
                player.target_id = new_target.clone();
                tracing::info!("{} now targeting {:?}", player.name, new_target);

                // Broadcast target change to all clients
                let msg = ServerMessage::TargetChanged {
                    player_id: player_id.to_string(),
                    target_id: new_target,
                };
                drop(players); // Release lock before broadcast
                self.broadcast(msg).await;
            }
        }
    }

    pub async fn handle_pickup(&self, player_id: &str, item_id: &str) {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Get player position
        let (player_x, player_y) = {
            let players = self.players.read().await;
            match players.get(player_id) {
                Some(p) if p.active && !p.is_dead => (p.x as f32, p.y as f32),
                _ => return, // Player not found, inactive, or dead
            }
        };

        // Check if item exists and can be picked up
        let item_info = {
            let items = self.ground_items.read().await;
            items.get(item_id).map(|item| {
                // Check distance (must be within 2 tiles)
                let dx = item.x - player_x;
                let dy = item.y - player_y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance > 2.0 {
                    return None;
                }

                if !item.can_pickup(player_id, current_time) {
                    return None;
                }

                Some((item.item_id.clone(), item.quantity))
            }).flatten()
        };

        if let Some((picked_item_id, quantity)) = item_info {
            // Remove item from ground
            let removed = {
                let mut items = self.ground_items.write().await;
                items.remove(item_id).is_some()
            };

            if removed {
                // Get display name from registry for logging
                let display_name = self.item_registry
                    .get(&picked_item_id)
                    .map(|def| def.display_name.as_str())
                    .unwrap_or(&picked_item_id);
                tracing::debug!("Player {} picked up {} x{}", player_id, display_name, quantity);

                // Add to player's inventory
                let (leftover, inventory_update, gold) = {
                    let mut players = self.players.write().await;
                    if let Some(player) = players.get_mut(player_id) {
                        let leftover = player.inventory.add_item(&picked_item_id, quantity, &self.item_registry);
                        (leftover, player.inventory.to_update(), player.inventory.gold)
                    } else {
                        return;
                    }
                };

                // Process quest item collection (amount actually picked up)
                let picked_up_count = quantity - leftover;
                if picked_up_count > 0 {
                    self.process_quest_item_collect(player_id, &picked_item_id, picked_up_count).await;
                }

                // Broadcast pickup (public info - everyone sees item disappear)
                let pickup_msg = ServerMessage::ItemPickedUp {
                    item_id: item_id.to_string(),
                    player_id: player_id.to_string(),
                };
                self.broadcast(pickup_msg).await;

                // SECURITY: Unicast inventory update (private - only this player receives)
                let inv_msg = ServerMessage::InventoryUpdate {
                    player_id: player_id.to_string(),
                    slots: inventory_update,
                    gold,
                };
                self.send_to_player(player_id, inv_msg).await;

                // If some items couldn't fit, drop them back on ground
                if leftover > 0 {
                    tracing::debug!("Inventory full, dropping {} back", leftover);
                    // Could spawn a new ground item here
                }
            }
        }
    }

    /// Handle NPC interaction (quest givers, merchants, etc.)
    pub async fn handle_npc_interact(&self, player_id: &str, npc_id: &str) {
        // Get player position
        let (player_x, player_y) = {
            let players = self.players.read().await;
            match players.get(player_id) {
                Some(p) if p.active && !p.is_dead => (p.x, p.y),
                _ => return,
            }
        };

        // Get NPC info and check distance
        let npc_info = {
            let npcs = self.npcs.read().await;
            npcs.get(npc_id).map(|npc| {
                let dx = (npc.x - player_x) as f32;
                let dy = (npc.y - player_y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();
                let entity_type = npc.prototype_id.clone().unwrap_or_else(|| "slime".to_string());
                (entity_type, distance, npc.is_alive())
            })
        };

        let (entity_type, distance, is_alive): (String, f32, bool) = match npc_info {
            Some(info) => info,
            None => {
                tracing::warn!("Player {} tried to interact with unknown NPC {}", player_id, npc_id);
                return;
            }
        };

        // Must be within interaction range (2 tiles) and NPC must be alive
        if distance > 2.5 || !is_alive {
            tracing::debug!(
                "Player {} can't interact with NPC {} (distance: {}, alive: {})",
                player_id, npc_id, distance, is_alive
            );
            return;
        }

        // Check entity prototype for behaviors
        let prototype = self.entity_registry.get(&entity_type);

        // Check if this NPC is a merchant/craftsman
        let is_merchant = prototype.as_ref()
            .map(|p| p.behaviors.merchant || p.behaviors.craftsman)
            .unwrap_or(false);

        // Check if this NPC has quests associated with it
        let quests = self.quest_registry.get_quests_for_npc(&entity_type).await;

        // If merchant/craftsman and no quests (or quest_giver behavior is not set), open shop
        let is_quest_giver = prototype.as_ref()
            .map(|p| p.behaviors.quest_giver)
            .unwrap_or(false);

        if is_merchant && (quests.is_empty() || !is_quest_giver) {
            tracing::info!("Player {} opening shop with NPC {} ({})", player_id, npc_id, entity_type);

            // Get merchant config to load shop data
            if let Some(proto) = prototype {
                if let Some(merchant_config) = &proto.merchant {
                    // Get shop definition from registry
                    let shop_registry = self.shop_registry.read().await;
                    if let Some(shop_def) = shop_registry.get(&merchant_config.shop_id) {
                        // Build shop data with current prices
                        let stock = shop_def.stock.iter().map(|item| {
                            let base_price = self.item_registry
                                .get(&item.item_id)
                                .map(|def| def.base_price)
                                .unwrap_or(10);
                            let price = (base_price as f32 * merchant_config.sell_multiplier).max(1.0) as i32;

                            crate::protocol::ShopStockItemData {
                                item_id: item.item_id.clone(),
                                quantity: item.current_quantity,
                                price,
                            }
                        }).collect();

                        let shop_data = crate::protocol::ShopData {
                            shop_id: shop_def.id.clone(),
                            display_name: shop_def.display_name.clone(),
                            buy_multiplier: merchant_config.buy_multiplier,
                            sell_multiplier: merchant_config.sell_multiplier,
                            stock,
                        };

                        drop(shop_registry);

                        let msg = ServerMessage::ShopData {
                            npc_id: npc_id.to_string(),
                            shop: shop_data,
                        };
                        self.send_to_player(player_id, msg).await;
                        return;
                    } else {
                        tracing::warn!("Shop '{}' not found for merchant NPC {}", merchant_config.shop_id, npc_id);
                    }
                }
            }

            // Fallback: send empty ShopOpen if shop data couldn't be loaded
            let msg = ServerMessage::ShopOpen {
                npc_id: npc_id.to_string(),
            };
            self.send_to_player(player_id, msg).await;
            return;
        }

        if quests.is_empty() {
            tracing::debug!("NPC {} ({}) has no quests", npc_id, entity_type);
            // Could show generic dialogue here
            return;
        }

        // Get or create player quest state
        let mut quest_states = self.player_quest_states.write().await;
        let quest_state = quest_states.entry(player_id.to_string())
            .or_insert_with(PlayerQuestState::new);

        // Find the appropriate quest to interact with
        // Priority: 1) Active quest ready to complete, 2) Active quest in progress,
        //           3) Available new quest (or repeatable completed quest), 4) Completed quest (for post-completion dialogue)
        let mut target_quest: Option<(String, &str)> = None;
        let mut completed_quest: Option<(String, &str)> = None;

        for quest in &quests {
            let quest_id = &quest.id;

            if let Some(progress) = quest_state.get_quest(quest_id) {
                // Player has this quest active
                if progress.status == crate::quest::QuestStatus::ReadyToComplete {
                    target_quest = Some((quest_id.clone(), "ready_to_complete"));
                    break;
                } else if progress.status == crate::quest::QuestStatus::Active {
                    target_quest = Some((quest_id.clone(), "in_progress"));
                    // Don't break, keep looking for ready_to_complete
                }
            } else if quest_state.is_quest_completed(quest_id) {
                // Quest is completed
                if quest.repeatable {
                    // Repeatable quest - treat as not started so player can do it again
                    if target_quest.is_none() {
                        // Remove from completed list so it can be restarted
                        quest_state.completed_quests.retain(|id| id != quest_id);
                        target_quest = Some((quest_id.clone(), "not_started"));
                    }
                } else {
                    // Non-repeatable - save as fallback for post-completion dialogue
                    if completed_quest.is_none() {
                        completed_quest = Some((quest_id.clone(), "completed"));
                    }
                }
            } else {
                // Player doesn't have this quest and hasn't completed it
                if target_quest.is_none() {
                    target_quest = Some((quest_id.clone(), "not_started"));
                }
            }
        }

        // Use completed quest as fallback if no other quest is found
        if target_quest.is_none() {
            target_quest = completed_quest;
        }

        if let Some((quest_id, state)) = target_quest {
            tracing::info!(
                "Player {} interacting with quest {} (state: {})",
                player_id, quest_id, state
            );

            // Trigger NpcInteraction event to complete any talk_to objectives
            // This must happen BEFORE running the script so the script sees the updated state
            let event = QuestEvent::NpcInteraction {
                player_id: player_id.to_string(),
                npc_id: entity_type.clone(),
            };
            let talk_results = self.quest_registry.process_event(&event, quest_state).await;
            for result in talk_results {
                if result.quest_ready {
                    tracing::info!(
                        "Player {} quest {} is now ready to complete after talking to NPC",
                        player_id, result.quest_id
                    );
                }
            }

            // Run the quest script interaction
            let result = self.quest_runner.run_on_interact(
                player_id,
                &quest_id,
                quest_state,
                None, // No choice yet
            ).await;

            match result {
                Ok(script_result) => {
                    // Send dialogue to player
                    if let Some(dialogue) = script_result.dialogue {
                        let choices: Vec<crate::protocol::DialogueChoice> = dialogue.choices
                            .into_iter()
                            .map(|c| crate::protocol::DialogueChoice {
                                id: c.id,
                                text: c.text,
                            })
                            .collect();

                        let msg = ServerMessage::ShowDialogue {
                            quest_id: quest_id.clone(),
                            npc_id: npc_id.to_string(),
                            speaker: dialogue.speaker,
                            text: dialogue.text,
                            choices,
                        };
                        self.send_to_player(player_id, msg).await;

                        // Persist dialogue step for proper dialogue flow
                        if let Some(step) = script_result.new_dialogue_step {
                            let step_key = format!("{}_dialogue_step", quest_id);
                            quest_state.set_flag(&step_key, &step.to_string());
                        }
                    }

                    // Handle quest acceptance
                    if script_result.quest_accepted {
                        if let Some(quest) = self.quest_registry.get(&quest_id).await {
                            let objective_targets: Vec<(String, i32)> = quest.objectives
                                .iter()
                                .map(|o| (o.id.clone(), o.count))
                                .collect();
                            quest_state.start_quest(&quest_id, &objective_targets);
                            tracing::info!("Player {} accepted quest {}", player_id, quest_id);

                            // Send QuestAccepted to client
                            let objectives: Vec<QuestObjectiveData> = quest.objectives
                                .iter()
                                .map(|o| QuestObjectiveData {
                                    id: o.id.clone(),
                                    description: o.description.clone(),
                                    current: 0,
                                    target: o.count,
                                    completed: false,
                                })
                                .collect();
                            let msg = ServerMessage::QuestAccepted {
                                quest_id: quest_id.clone(),
                                quest_name: quest.name.clone(),
                                objectives,
                            };
                            self.send_to_player(player_id, msg).await;
                        }
                    }

                    // Handle quest completion
                    if script_result.quest_completed {
                        quest_state.complete_quest(&quest_id);
                        if let Some(quest) = self.quest_registry.get(&quest_id).await {
                            let msg = ServerMessage::QuestCompleted {
                                quest_id: quest_id.clone(),
                                quest_name: quest.name.clone(),
                                rewards_exp: quest.rewards.exp,
                                rewards_gold: quest.rewards.gold,
                            };
                            self.send_to_player(player_id, msg).await;

                            // Grant rewards
                            let mut players = self.players.write().await;
                            if let Some(player) = players.get_mut(player_id) {
                                player.inventory.gold += quest.rewards.gold;
                                // TODO: Grant EXP and items
                            }
                        }
                        tracing::info!("Player {} completed quest {}", player_id, quest_id);
                    }

                    // Send notifications
                    for notification in script_result.notifications {
                        tracing::info!("Quest notification for {}: {}", player_id, notification);
                        // TODO: Send notification message to client
                    }
                }
                Err(e) => {
                    tracing::error!("Quest script error: {}", e);
                }
            }
        }
    }

    /// Handle dialogue choice from player
    pub async fn handle_dialogue_choice(&self, player_id: &str, quest_id: &str, choice_id: &str) {
        let mut quest_states = self.player_quest_states.write().await;
        let quest_state = quest_states.entry(player_id.to_string())
            .or_insert_with(PlayerQuestState::new);

        // Run the quest script with the player's choice
        let result = self.quest_runner.run_on_interact(
            player_id,
            quest_id,
            quest_state,
            Some(choice_id),
        ).await;

        // Get the NPC id from the quest giver
        let npc_id = if let Some(quest) = self.quest_registry.get(quest_id).await {
            quest.giver_npc.clone()
        } else {
            String::new()
        };

        match result {
            Ok(script_result) => {
                // Send next dialogue if any, otherwise close the dialogue
                if let Some(dialogue) = script_result.dialogue {
                    let choices: Vec<crate::protocol::DialogueChoice> = dialogue.choices
                        .into_iter()
                        .map(|c| crate::protocol::DialogueChoice {
                            id: c.id,
                            text: c.text,
                        })
                        .collect();

                    let msg = ServerMessage::ShowDialogue {
                        quest_id: quest_id.to_string(),
                        npc_id: npc_id.clone(),
                        speaker: dialogue.speaker,
                        text: dialogue.text,
                        choices,
                    };
                    self.send_to_player(player_id, msg).await;
                    // Persist dialogue step for proper dialogue flow
                    if let Some(step) = script_result.new_dialogue_step {
                        let step_key = format!("{}_dialogue_step", quest_id);
                        quest_state.set_flag(&step_key, &step.to_string());
                    }
                } else {
                    // No follow-up dialogue - tell client to close
                    let msg = ServerMessage::DialogueClosed;
                    self.send_to_player(player_id, msg).await;

                    // Reset dialogue step since conversation ended
                    let step_key = format!("{}_dialogue_step", quest_id);
                    quest_state.flags.remove(&step_key);
                }

                // Handle quest acceptance
                if script_result.quest_accepted {
                    if let Some(quest) = self.quest_registry.get(quest_id).await {
                        let objective_targets: Vec<(String, i32)> = quest.objectives
                            .iter()
                            .map(|o| (o.id.clone(), o.count))
                            .collect();
                        quest_state.start_quest(quest_id, &objective_targets);
                        tracing::info!("Player {} accepted quest {}", player_id, quest_id);

                        // Send QuestAccepted to client
                        let objectives: Vec<QuestObjectiveData> = quest.objectives
                            .iter()
                            .map(|o| QuestObjectiveData {
                                id: o.id.clone(),
                                description: o.description.clone(),
                                current: 0,
                                target: o.count,
                                completed: false,
                            })
                            .collect();
                        let msg = ServerMessage::QuestAccepted {
                            quest_id: quest_id.to_string(),
                            quest_name: quest.name.clone(),
                            objectives,
                        };
                        self.send_to_player(player_id, msg).await;
                    }
                }

                // Handle quest completion
                if script_result.quest_completed {
                    quest_state.complete_quest(quest_id);
                    if let Some(quest) = self.quest_registry.get(quest_id).await {
                        let msg = ServerMessage::QuestCompleted {
                            quest_id: quest_id.to_string(),
                            quest_name: quest.name.clone(),
                            rewards_exp: quest.rewards.exp,
                            rewards_gold: quest.rewards.gold,
                        };
                        self.send_to_player(player_id, msg).await;

                        let mut players = self.players.write().await;
                        if let Some(player) = players.get_mut(player_id) {
                            player.inventory.gold += quest.rewards.gold;
                        }
                    }
                    tracing::info!("Player {} completed quest {}", player_id, quest_id);
                }
            }
            Err(e) => {
                tracing::error!("Quest script error: {}", e);
            }
        }
    }

    pub async fn handle_use_item(&self, player_id: &str, slot_index: u8) {
        // Get player and try to use item
        let (used_item_id, effect, inventory_update, gold) = {
            let mut players = self.players.write().await;
            if let Some(player) = players.get_mut(player_id) {
                // Dead players can't use items
                if player.is_dead {
                    return;
                }

                if let Some(item_id) = player.inventory.use_item(slot_index as usize, &self.item_registry) {
                    // Get effect from registry
                    let effect = if let Some(def) = self.item_registry.get(&item_id) {
                        use crate::data::UseEffect;
                        match &def.use_effect {
                            Some(UseEffect::Heal { amount }) => {
                                player.hp = (player.hp + amount).min(player.max_hp());
                                format!("heal:{}", amount)
                            }
                            Some(UseEffect::RestoreMana { amount }) => {
                                // Mana not implemented yet
                                format!("mana:{}", amount)
                            }
                            Some(UseEffect::Buff { stat, amount, duration_ms }) => {
                                // Buffs not implemented yet
                                format!("buff:{}:{}:{}", stat, amount, duration_ms)
                            }
                            Some(UseEffect::Teleport { destination }) => {
                                // Teleport not implemented yet
                                format!("teleport:{}", destination)
                            }
                            None => "none".to_string(),
                        }
                    } else {
                        "none".to_string()
                    };

                    let update = player.inventory.to_update();
                    (Some(item_id), effect, update, player.inventory.gold)
                } else {
                    return;
                }
            } else {
                return;
            }
        };

        if let Some(item_id) = used_item_id {
            let display_name = self.item_registry
                .get(&item_id)
                .map(|def| def.display_name.as_str())
                .unwrap_or(&item_id);
            tracing::debug!("Player {} used {} ({})", player_id, display_name, effect);

            // SECURITY: Unicast item used and inventory update (private to this player)
            let used_msg = ServerMessage::ItemUsed {
                player_id: player_id.to_string(),
                slot: slot_index,
                item_id: item_id.clone(),
                effect,
            };
            self.send_to_player(player_id, used_msg).await;

            let inv_msg = ServerMessage::InventoryUpdate {
                player_id: player_id.to_string(),
                slots: inventory_update,
                gold,
            };
            self.send_to_player(player_id, inv_msg).await;
        }
    }

    /// Handle a crafting request from a player
    pub async fn handle_craft(&self, player_id: &str, recipe_id: &str) {
        use crate::protocol::RecipeResult as ProtoRecipeResult;

        // Get recipe definition
        let recipe = match self.crafting_registry.get(recipe_id) {
            Some(r) => r.clone(),
            None => {
                self.send_to_player(
                    player_id,
                    ServerMessage::CraftResult {
                        success: false,
                        recipe_id: recipe_id.to_string(),
                        error: Some("Recipe not found".to_string()),
                        items_gained: vec![],
                    },
                )
                .await;
                return;
            }
        };

        // Get player and perform all checks
        let mut players = self.players.write().await;
        let player = match players.get_mut(player_id) {
            Some(p) if p.active && !p.is_dead => p,
            _ => return,
        };

        // Check level requirement (use combat level for now, will add crafting skill later)
        if player.combat_level() < recipe.level_required {
            drop(players);
            self.send_to_player(
                player_id,
                ServerMessage::CraftResult {
                    success: false,
                    recipe_id: recipe_id.to_string(),
                    error: Some(format!("Requires combat level {}", recipe.level_required)),
                    items_gained: vec![],
                },
            )
            .await;
            return;
        }

        // Check all ingredients (using string IDs now)
        for ingredient in &recipe.ingredients {
            if !player.inventory.has_item(&ingredient.item_id, ingredient.count) {
                drop(players);
                self.send_to_player(
                    player_id,
                    ServerMessage::CraftResult {
                        success: false,
                        recipe_id: recipe_id.to_string(),
                        error: Some("Missing ingredients".to_string()),
                        items_gained: vec![],
                    },
                )
                .await;
                return;
            }
        }

        // Check inventory space for results
        for result in &recipe.results {
            if !player.inventory.has_space_for(&result.item_id, result.count, &self.item_registry) {
                drop(players);
                self.send_to_player(
                    player_id,
                    ServerMessage::CraftResult {
                        success: false,
                        recipe_id: recipe_id.to_string(),
                        error: Some("Inventory full".to_string()),
                        items_gained: vec![],
                    },
                )
                .await;
                return;
            }
        }

        // All checks passed - consume ingredients
        for ingredient in &recipe.ingredients {
            player.inventory.remove_item(&ingredient.item_id, ingredient.count);
        }

        // Add results
        let mut items_gained = Vec::new();
        for result in &recipe.results {
            player.inventory.add_item(&result.item_id, result.count, &self.item_registry);
            let display_name = self.item_registry
                .get(&result.item_id)
                .map(|def| def.display_name.clone())
                .unwrap_or_else(|| result.item_id.clone());
            items_gained.push(ProtoRecipeResult {
                item_id: result.item_id.clone(),
                item_name: display_name,
                count: result.count,
            });
        }

        // Get inventory update
        let inventory_update = player.inventory.to_update();
        let gold = player.inventory.gold;
        drop(players);

        tracing::info!(
            "Player {} crafted {} (gained {:?})",
            player_id,
            recipe_id,
            items_gained
        );

        // Send success result
        self.send_to_player(
            player_id,
            ServerMessage::CraftResult {
                success: true,
                recipe_id: recipe_id.to_string(),
                error: None,
                items_gained,
            },
        )
        .await;

        // Send inventory update
        self.send_to_player(
            player_id,
            ServerMessage::InventoryUpdate {
                player_id: player_id.to_string(),
                slots: inventory_update,
                gold,
            },
        )
        .await;
    }

    /// Handle shop buy transaction
    pub async fn handle_shop_buy(&self, player_id: &str, npc_id: &str, item_id: &str, quantity: i32) {
        // Validate quantity
        if quantity <= 0 {
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Invalid quantity")).await;
            return;
        }

        // Get player position and gold
        let (player_x, player_y, player_gold) = {
            let players = self.players.read().await;
            match players.get(player_id) {
                Some(p) if p.active && !p.is_dead => (p.x, p.y, p.inventory.gold),
                _ => return,
            }
        };

        // Get NPC position and prototype ID
        let npc_info = {
            let npcs = self.npcs.read().await;
            npcs.get(npc_id).map(|npc| {
                let dx = (npc.x - player_x) as f32;
                let dy = (npc.y - player_y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();
                (npc.prototype_id.clone(), distance, npc.is_alive())
            })
        };

        let (prototype_id, distance, is_alive) = match npc_info {
            Some(info) => info,
            None => {
                self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("NPC not found")).await;
                return;
            }
        };

        // Check distance (must be within 2.5 tiles)
        if distance > 2.5 || !is_alive {
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Too far from merchant")).await;
            return;
        }

        // Get prototype and merchant config
        let prototype_id = prototype_id.unwrap_or_else(|| "unknown".to_string());
        let merchant_config = match self.entity_registry.get(&prototype_id) {
            Some(proto) => match &proto.merchant {
                Some(config) => config.clone(),
                None => {
                    self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Not a merchant")).await;
                    return;
                }
            },
            None => {
                self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Invalid merchant")).await;
                return;
            }
        };

        // Get shop definition and check stock
        let mut shop_registry = self.shop_registry.write().await;
        let shop = match shop_registry.get_mut(&merchant_config.shop_id) {
            Some(s) => s,
            None => {
                drop(shop_registry);
                self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Shop not found")).await;
                return;
            }
        };

        // Check if item is in stock
        let stock_item = match shop.get_stock_mut(item_id) {
            Some(s) => s,
            None => {
                drop(shop_registry);
                self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Item not sold here")).await;
                return;
            }
        };

        // Check stock quantity
        if stock_item.current_quantity < quantity {
            drop(shop_registry);
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Insufficient stock")).await;
            return;
        }

        // Get item definition for base price
        let item_def = match self.item_registry.get(item_id) {
            Some(def) => def.clone(),
            None => {
                drop(shop_registry);
                self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Item not found")).await;
                return;
            }
        };

        // Calculate total cost
        let base_price = item_def.base_price;
        let unit_price = (base_price as f32 * merchant_config.sell_multiplier).max(1.0) as i32;
        let total_cost = unit_price * quantity;

        // Check if player has enough gold
        if player_gold < total_cost {
            drop(shop_registry);
            self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Not enough gold")).await;
            return;
        }

        // Check if player has inventory space
        {
            let players = self.players.read().await;
            let player = match players.get(player_id) {
                Some(p) if p.active && !p.is_dead => p,
                _ => {
                    drop(shop_registry);
                    return;
                }
            };

            if !player.inventory.has_space_for(item_id, quantity, &self.item_registry) {
                drop(shop_registry);
                self.send_shop_result(player_id, false, "buy", item_id, 0, 0, Some("Inventory full")).await;
                return;
            }
        }

        // All checks passed - process transaction
        stock_item.current_quantity -= quantity;
        let new_stock = stock_item.current_quantity;
        drop(shop_registry);

        // Update player inventory
        {
            let mut players = self.players.write().await;
            if let Some(player) = players.get_mut(player_id) {
                player.inventory.gold -= total_cost;
                player.inventory.add_item(item_id, quantity, &self.item_registry);

                let inventory_update = player.inventory.to_update();
                let gold = player.inventory.gold;
                drop(players);

                // Send success result
                self.send_shop_result(player_id, true, "buy", item_id, quantity, -total_cost, None).await;

                // Send inventory update
                self.send_to_player(
                    player_id,
                    ServerMessage::InventoryUpdate {
                        player_id: player_id.to_string(),
                        slots: inventory_update,
                        gold,
                    },
                )
                .await;

                // Broadcast stock update to nearby players
                self.broadcast_shop_stock_update(npc_id, item_id, new_stock).await;

                tracing::info!(
                    "Player {} bought {}x{} from {} for {} gold",
                    player_id, quantity, item_id, npc_id, total_cost
                );
            }
        }
    }

    /// Send shop result message to player
    async fn send_shop_result(
        &self,
        player_id: &str,
        success: bool,
        action: &str,
        item_id: &str,
        quantity: i32,
        gold_change: i32,
        error: Option<&str>,
    ) {
        self.send_to_player(
            player_id,
            ServerMessage::ShopResult {
                success,
                action: action.to_string(),
                item_id: item_id.to_string(),
                quantity,
                gold_change,
                error: error.map(|s| s.to_string()),
            },
        )
        .await;
    }

    /// Broadcast shop stock update to all players
    async fn broadcast_shop_stock_update(&self, npc_id: &str, item_id: &str, new_quantity: i32) {
        self.broadcast(ServerMessage::ShopStockUpdate {
            npc_id: npc_id.to_string(),
            item_id: item_id.to_string(),
            new_quantity,
        })
        .await;
    }

    /// Handle shop sell transaction
    pub async fn handle_shop_sell(&self, player_id: &str, npc_id: &str, item_id: &str, quantity: i32) {
        // Validate quantity
        if quantity <= 0 {
            self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Invalid quantity")).await;
            return;
        }

        // Get player position
        let (player_x, player_y) = {
            let players = self.players.read().await;
            match players.get(player_id) {
                Some(p) if p.active && !p.is_dead => (p.x, p.y),
                _ => return,
            }
        };

        // Get NPC position and prototype ID
        let npc_info = {
            let npcs = self.npcs.read().await;
            npcs.get(npc_id).map(|npc| {
                let dx = (npc.x - player_x) as f32;
                let dy = (npc.y - player_y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();
                (npc.prototype_id.clone(), distance, npc.is_alive())
            })
        };

        let (prototype_id, distance, is_alive) = match npc_info {
            Some(info) => info,
            None => {
                self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("NPC not found")).await;
                return;
            }
        };

        // Check distance (must be within 2.5 tiles)
        if distance > 2.5 || !is_alive {
            self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Too far from merchant")).await;
            return;
        }

        // Get prototype and merchant config
        let prototype_id = prototype_id.unwrap_or_else(|| "unknown".to_string());
        let merchant_config = match self.entity_registry.get(&prototype_id) {
            Some(proto) => match &proto.merchant {
                Some(config) => config.clone(),
                None => {
                    self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Not a merchant")).await;
                    return;
                }
            },
            None => {
                self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Invalid merchant")).await;
                return;
            }
        };

        // Get item definition
        let item_def = match self.item_registry.get(item_id) {
            Some(def) => def.clone(),
            None => {
                self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Item not found")).await;
                return;
            }
        };

        // Check if item is sellable
        if !item_def.sellable {
            self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("Item cannot be sold")).await;
            return;
        }

        // Check if player has the item
        let has_item = {
            let players = self.players.read().await;
            match players.get(player_id) {
                Some(p) if p.active && !p.is_dead => p.inventory.has_item(item_id, quantity),
                _ => false,
            }
        };

        if !has_item {
            self.send_shop_result(player_id, false, "sell", item_id, 0, 0, Some("You don't have enough of that item")).await;
            return;
        }

        // Calculate sell price
        let base_price = item_def.base_price;
        let unit_price = (base_price as f32 * merchant_config.buy_multiplier).max(1.0) as i32;
        let total_value = unit_price * quantity;

        // Process transaction
        {
            let mut players = self.players.write().await;
            if let Some(player) = players.get_mut(player_id) {
                // Remove items from inventory
                player.inventory.remove_item(item_id, quantity);
                // Add gold
                player.inventory.gold += total_value;

                let inventory_update = player.inventory.to_update();
                let gold = player.inventory.gold;
                drop(players);

                // Send success result
                self.send_shop_result(player_id, true, "sell", item_id, quantity, total_value, None).await;

                // Send inventory update
                self.send_to_player(
                    player_id,
                    ServerMessage::InventoryUpdate {
                        player_id: player_id.to_string(),
                        slots: inventory_update,
                        gold,
                    },
                )
                .await;

                tracing::info!(
                    "Player {} sold {}x{} to {} for {} gold",
                    player_id, quantity, item_id, npc_id, total_value
                );
            }
        }
    }

    /// Handle equipping an item from inventory
    pub async fn handle_equip(&self, player_id: &str, slot_index: u8) {
        use crate::data::item_def::EquipmentSlot;

        let slot_idx = slot_index as usize;

        // Get item info from inventory slot
        let item_info = {
            let players = self.players.read().await;
            let player = match players.get(player_id) {
                Some(p) if p.active && !p.is_dead => p,
                _ => return,
            };

            match player.inventory.slots.get(slot_idx) {
                Some(Some(slot)) => Some((
                    slot.item_id.clone(),
                    player.skills.combat.level,
                    player.equipped_head.clone(),
                    player.equipped_body.clone(),
                    player.equipped_weapon.clone(),
                    player.equipped_back.clone(),
                    player.equipped_feet.clone(),
                    player.equipped_ring.clone(),
                    player.equipped_gloves.clone(),
                    player.equipped_necklace.clone(),
                    player.equipped_belt.clone(),
                )),
                _ => None,
            }
        };

        let (item_id, combat_level, equipped_head, equipped_body, equipped_weapon, equipped_back, equipped_feet, equipped_ring, equipped_gloves, equipped_necklace, equipped_belt) = match item_info {
            Some(info) => info,
            None => {
                self.send_to_player(player_id, ServerMessage::EquipResult {
                    success: false,
                    slot_type: "unknown".to_string(),
                    item_id: None,
                    error: Some("No item in that slot".to_string()),
                }).await;
                return;
            }
        };

        // Get item definition
        let item_def = match self.item_registry.get(&item_id) {
            Some(def) => def,
            None => {
                self.send_to_player(player_id, ServerMessage::EquipResult {
                    success: false,
                    slot_type: "unknown".to_string(),
                    item_id: None,
                    error: Some("Item not found".to_string()),
                }).await;
                return;
            }
        };

        // Check if item is equippable and get its slot type
        let (equip_stats, equip_slot) = match &item_def.equipment {
            Some(stats) if stats.slot_type != EquipmentSlot::None => (stats, stats.slot_type),
            _ => {
                self.send_to_player(player_id, ServerMessage::EquipResult {
                    success: false,
                    slot_type: "unknown".to_string(),
                    item_id: None,
                    error: Some("Item cannot be equipped".to_string()),
                }).await;
                return;
            }
        };

        let slot_type_str = equip_slot.as_str().to_string();

        // Check skill level requirements
        // Both attack and defence requirements are checked against the unified combat level
        let level_required = equip_stats.attack_level_required.max(equip_stats.defence_level_required);
        if level_required > 0 && combat_level < level_required {
            self.send_to_player(player_id, ServerMessage::EquipResult {
                success: false,
                slot_type: slot_type_str,
                item_id: None,
                error: Some(format!("Requires Combat level {}", level_required)),
            }).await;
            return;
        }

        // Get currently equipped item for this slot
        let currently_equipped = match equip_slot {
            EquipmentSlot::Head => equipped_head,
            EquipmentSlot::Body => equipped_body,
            EquipmentSlot::Weapon => equipped_weapon,
            EquipmentSlot::Back => equipped_back,
            EquipmentSlot::Feet => equipped_feet,
            EquipmentSlot::Ring => equipped_ring,
            EquipmentSlot::Gloves => equipped_gloves,
            EquipmentSlot::Necklace => equipped_necklace,
            EquipmentSlot::Belt => equipped_belt,
            EquipmentSlot::None => None,
        };

        // Perform the equip operation
        let (inventory_update, gold, new_equipment) = {
            let mut players = self.players.write().await;
            let player = match players.get_mut(player_id) {
                Some(p) => p,
                None => return,
            };

            // If something is equipped in this slot, swap it to the inventory slot
            if let Some(ref old_item_id) = currently_equipped {
                player.inventory.slots[slot_idx] = Some(item::InventorySlot::new(old_item_id.clone(), 1));
            } else {
                player.inventory.slots[slot_idx] = None;
            }

            // Equip the new item to the appropriate slot
            match equip_slot {
                EquipmentSlot::Head => player.equipped_head = Some(item_id.clone()),
                EquipmentSlot::Body => player.equipped_body = Some(item_id.clone()),
                EquipmentSlot::Weapon => player.equipped_weapon = Some(item_id.clone()),
                EquipmentSlot::Back => player.equipped_back = Some(item_id.clone()),
                EquipmentSlot::Feet => player.equipped_feet = Some(item_id.clone()),
                EquipmentSlot::Ring => player.equipped_ring = Some(item_id.clone()),
                EquipmentSlot::Gloves => player.equipped_gloves = Some(item_id.clone()),
                EquipmentSlot::Necklace => player.equipped_necklace = Some(item_id.clone()),
                EquipmentSlot::Belt => player.equipped_belt = Some(item_id.clone()),
                EquipmentSlot::None => {}
            }

            (
                player.inventory.to_update(),
                player.inventory.gold,
                (
                    player.equipped_head.clone(),
                    player.equipped_body.clone(),
                    player.equipped_weapon.clone(),
                    player.equipped_back.clone(),
                    player.equipped_feet.clone(),
                    player.equipped_ring.clone(),
                    player.equipped_gloves.clone(),
                    player.equipped_necklace.clone(),
                    player.equipped_belt.clone(),
                ),
            )
        };

        tracing::info!("Player {} equipped {} to {} slot", player_id, item_id, slot_type_str);

        // Send success result
        self.send_to_player(player_id, ServerMessage::EquipResult {
            success: true,
            slot_type: slot_type_str,
            item_id: Some(item_id.clone()),
            error: None,
        }).await;

        // Send inventory update
        self.send_to_player(player_id, ServerMessage::InventoryUpdate {
            player_id: player_id.to_string(),
            slots: inventory_update,
            gold,
        }).await;

        // Broadcast equipment update to all players
        self.broadcast(ServerMessage::EquipmentUpdate {
            player_id: player_id.to_string(),
            equipped_head: new_equipment.0,
            equipped_body: new_equipment.1,
            equipped_weapon: new_equipment.2,
            equipped_back: new_equipment.3,
            equipped_feet: new_equipment.4,
            equipped_ring: new_equipment.5,
            equipped_gloves: new_equipment.6,
            equipped_necklace: new_equipment.7,
            equipped_belt: new_equipment.8,
        }).await;
    }

    /// Handle unequipping an item
    pub async fn handle_unequip(&self, player_id: &str, slot_type: &str) {
        // Validate slot type
        let valid_slots = ["head", "body", "weapon", "back", "feet", "ring", "gloves", "necklace", "belt"];
        if !valid_slots.contains(&slot_type) {
            self.send_to_player(player_id, ServerMessage::EquipResult {
                success: false,
                slot_type: slot_type.to_string(),
                item_id: None,
                error: Some("Unknown equipment slot".to_string()),
            }).await;
            return;
        }

        // Check if something is equipped and if inventory has space
        let equipped_item = {
            let players = self.players.read().await;
            let player = match players.get(player_id) {
                Some(p) if p.active && !p.is_dead => p,
                _ => return,
            };

            let equipped_ref = match slot_type {
                "head" => &player.equipped_head,
                "body" => &player.equipped_body,
                "weapon" => &player.equipped_weapon,
                "back" => &player.equipped_back,
                "feet" => &player.equipped_feet,
                "ring" => &player.equipped_ring,
                "gloves" => &player.equipped_gloves,
                "necklace" => &player.equipped_necklace,
                "belt" => &player.equipped_belt,
                _ => return,
            };

            match equipped_ref {
                Some(item_id) => {
                    // Check if inventory has space
                    if !player.inventory.has_space_for(item_id, 1, &self.item_registry) {
                        self.send_to_player(player_id, ServerMessage::EquipResult {
                            success: false,
                            slot_type: slot_type.to_string(),
                            item_id: None,
                            error: Some("Inventory full".to_string()),
                        }).await;
                        return;
                    }
                    Some(item_id.clone())
                }
                None => {
                    self.send_to_player(player_id, ServerMessage::EquipResult {
                        success: false,
                        slot_type: slot_type.to_string(),
                        item_id: None,
                        error: Some("Nothing equipped".to_string()),
                    }).await;
                    return;
                }
            }
        };

        let item_id = match equipped_item {
            Some(id) => id,
            None => return,
        };

        // Perform the unequip operation
        let (inventory_update, gold, new_equipment) = {
            let mut players = self.players.write().await;
            let player = match players.get_mut(player_id) {
                Some(p) => p,
                None => return,
            };

            // Clear equipped item from appropriate slot
            match slot_type {
                "head" => player.equipped_head = None,
                "body" => player.equipped_body = None,
                "weapon" => player.equipped_weapon = None,
                "back" => player.equipped_back = None,
                "feet" => player.equipped_feet = None,
                "ring" => player.equipped_ring = None,
                "gloves" => player.equipped_gloves = None,
                "necklace" => player.equipped_necklace = None,
                "belt" => player.equipped_belt = None,
                _ => {}
            }

            // Add to inventory
            player.inventory.add_item(&item_id, 1, &self.item_registry);

            (
                player.inventory.to_update(),
                player.inventory.gold,
                (
                    player.equipped_head.clone(),
                    player.equipped_body.clone(),
                    player.equipped_weapon.clone(),
                    player.equipped_back.clone(),
                    player.equipped_feet.clone(),
                    player.equipped_ring.clone(),
                    player.equipped_gloves.clone(),
                    player.equipped_necklace.clone(),
                    player.equipped_belt.clone(),
                ),
            )
        };

        tracing::info!("Player {} unequipped {} from {} slot", player_id, item_id, slot_type);

        // Send success result
        self.send_to_player(player_id, ServerMessage::EquipResult {
            success: true,
            slot_type: slot_type.to_string(),
            item_id: Some(item_id),
            error: None,
        }).await;

        // Send inventory update
        self.send_to_player(player_id, ServerMessage::InventoryUpdate {
            player_id: player_id.to_string(),
            slots: inventory_update,
            gold,
        }).await;

        // Broadcast equipment update to all players
        self.broadcast(ServerMessage::EquipmentUpdate {
            player_id: player_id.to_string(),
            equipped_head: new_equipment.0,
            equipped_body: new_equipment.1,
            equipped_weapon: new_equipment.2,
            equipped_back: new_equipment.3,
            equipped_feet: new_equipment.4,
            equipped_ring: new_equipment.5,
            equipped_gloves: new_equipment.6,
            equipped_necklace: new_equipment.7,
            equipped_belt: new_equipment.8,
        }).await;
    }

    /// Handle dropping an item from inventory to the ground
    pub async fn handle_drop_item(&self, player_id: &str, slot_index: u8, quantity: u32) {
        let slot_idx = slot_index as usize;

        // Get player position and item info
        let drop_info: Option<(i32, i32, String, i32)> = {
            let players = self.players.read().await;
            let player = match players.get(player_id) {
                Some(p) if p.active && !p.is_dead => p,
                _ => return,
            };

            match player.inventory.slots.get(slot_idx) {
                Some(Some(slot)) => {
                    let qty_to_drop = (quantity as i32).min(slot.quantity);
                    if qty_to_drop <= 0 {
                        return;
                    }
                    Some((player.x, player.y, slot.item_id.clone(), qty_to_drop))
                }
                _ => None,
            }
        };

        let (player_x, player_y, item_id, qty_to_drop) = match drop_info {
            Some(info) => info,
            None => return,
        };

        // Remove item from inventory (manipulate slot directly)
        let (inventory_update, gold) = {
            let mut players = self.players.write().await;
            if let Some(player) = players.get_mut(player_id) {
                if let Some(slot) = &mut player.inventory.slots[slot_idx] {
                    slot.quantity -= qty_to_drop;
                    if slot.quantity <= 0 {
                        player.inventory.slots[slot_idx] = None;
                    }
                }
                (player.inventory.to_update(), player.inventory.gold)
            } else {
                return;
            }
        };

        // Create ground item with owner protection
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Center item on tile (add 0.5 to tile coordinates)
        let drop_x = player_x as f32 + 0.5;
        let drop_y = player_y as f32 + 0.5;

        let ground_item = GroundItem::new(
            &uuid::Uuid::new_v4().to_string(),
            &item_id,
            drop_x,
            drop_y,
            qty_to_drop,
            Some(player_id.to_string()),
            current_time,
        );

        tracing::info!("Player {} dropped {}x {} (protected for 10s)", player_id, qty_to_drop, item_id);

        // Broadcast item drop
        self.broadcast(ServerMessage::ItemDropped {
            id: ground_item.id.clone(),
            item_id: item_id.clone(),
            x: drop_x,
            y: drop_y,
            quantity: qty_to_drop,
        }).await;

        // Store in ground_items
        {
            let mut items = self.ground_items.write().await;
            items.insert(ground_item.id.clone(), ground_item);
        }

        // Send inventory update to dropping player
        self.send_to_player(player_id, ServerMessage::InventoryUpdate {
            player_id: player_id.to_string(),
            slots: inventory_update,
            gold,
        }).await;
    }

    /// Swap two inventory slots
    pub async fn handle_swap_slots(&self, player_id: &str, from_slot: u8, to_slot: u8) {
        let from_idx = from_slot as usize;
        let to_idx = to_slot as usize;

        // Validate slot indices
        if from_idx >= 20 || to_idx >= 20 || from_idx == to_idx {
            return;
        }

        // Perform the swap and get inventory update
        let (inventory_update, gold) = {
            let mut players = self.players.write().await;
            let player = match players.get_mut(player_id) {
                Some(p) if p.active && !p.is_dead => p,
                _ => return,
            };

            // Swap the slots
            player.inventory.slots.swap(from_idx, to_idx);

            (player.inventory.to_update(), player.inventory.gold)
        };

        tracing::debug!("Player {} swapped slots {} <-> {}", player_id, from_slot, to_slot);

        // Send inventory update to the player
        self.send_to_player(player_id, ServerMessage::InventoryUpdate {
            player_id: player_id.to_string(),
            slots: inventory_update,
            gold,
        }).await;
    }

    pub async fn tick(&self) {
        let delta_time = 1.0 / TICK_RATE;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Update tick counter and get current tick for movement timing
        let current_tick = {
            let mut tick = self.tick.write().await;
            *tick += 1;
            *tick
        };

        // Handle player respawns
        let mut respawned_players = Vec::new();
        {
            let mut players = self.players.write().await;
            for player in players.values_mut() {
                if !player.active {
                    continue;
                }

                if player.ready_to_respawn(current_time) {
                    player.respawn();
                    respawned_players.push((player.id.clone(), player.x, player.y, player.hp));
                }
            }
        }

        // Broadcast respawns
        for (id, x, y, hp) in respawned_players {
            tracing::info!("Player {} respawned at ({}, {})", id, x, y);
            self.broadcast(ServerMessage::PlayerRespawned { id, x, y, hp }).await;
        }

        // Collect pending moves (id, target_x, target_y)
        // Use tick-based cooldown for deterministic timing (5 ticks = 250ms)
        let pending_moves: Vec<(String, i32, i32)> = {
            let players = self.players.read().await;
            players.values()
                .filter(|p| p.active && !p.is_dead)
                .filter(|p| p.move_dx != 0 || p.move_dy != 0)
                .filter(|p| current_tick - p.last_move_tick >= MOVE_COOLDOWN_TICKS)
                .map(|p| (p.id.clone(), p.x + p.move_dx, p.y + p.move_dy))
                .collect()
        };

        // Check walkability for each pending move (async world check)
        let mut valid_moves: Vec<(String, i32, i32)> = Vec::new();
        for (id, target_x, target_y) in pending_moves {
            if self.world.is_tile_walkable(target_x, target_y).await {
                valid_moves.push((id, target_x, target_y));
            }
        }

        // Apply valid moves and collect player updates
        let mut player_updates = Vec::new();
        {
            let mut players = self.players.write().await;

            // Apply valid moves
            for (id, target_x, target_y) in valid_moves {
                if let Some(player) = players.get_mut(&id) {
                    player.x = target_x;
                    player.y = target_y;
                    player.last_move_tick = current_tick;
                }
            }

            // Generate player updates
            for player in players.values() {
                if !player.active {
                    continue;
                }

                player_updates.push(PlayerUpdate {
                    id: player.id.clone(),
                    x: player.x,
                    y: player.y,
                    direction: player.direction as u8,
                    // Include velocity for client-side prediction
                    vel_x: player.move_dx,
                    vel_y: player.move_dy,
                    hp: player.hp,
                    max_hp: player.max_hp(),
                    combat_level: player.combat_level(),
                    hitpoints_level: player.skills.hitpoints.level,
                    combat_skill_level: player.skills.combat.level,
                    gold: player.inventory.gold,
                    gender: player.gender.clone(),
                    skin: player.skin.clone(),
                    equipped_head: player.equipped_head.clone(),
                    equipped_body: player.equipped_body.clone(),
                    equipped_weapon: player.equipped_weapon.clone(),
                    equipped_back: player.equipped_back.clone(),
                    equipped_feet: player.equipped_feet.clone(),
                    equipped_ring: player.equipped_ring.clone(),
                    equipped_gloves: player.equipped_gloves.clone(),
                    equipped_necklace: player.equipped_necklace.clone(),
                    equipped_belt: player.equipped_belt.clone(),
                    is_admin: player.is_admin,
                });
            }
        }

        // Get player positions for NPC AI (only alive players, grid positions)
        let player_positions: Vec<(String, i32, i32, i32)> = {
            let players = self.players.read().await;
            players.values()
                .filter(|p| p.active && p.is_alive())
                .map(|p| (p.id.clone(), p.x, p.y, p.hp))
                .collect()
        };

        let mut npc_updates = Vec::new();
        let mut respawned_npcs = Vec::new();
        let mut npc_attacks: Vec<(String, String, i32, i32)> = Vec::new(); // (npc_id, target_id, npc_level, max_hit)
        {
            let mut npcs = self.npcs.write().await;

            // Collect NPC positions for collision detection (only alive NPCs)
            let mut npc_positions: std::collections::HashMap<String, (i32, i32)> = npcs
                .values()
                .filter(|n| n.is_alive())
                .map(|n| (n.id.clone(), (n.x, n.y)))
                .collect();

            for npc in npcs.values_mut() {
                // Check for respawn
                if npc.ready_to_respawn(current_time) {
                    npc.respawn();
                    respawned_npcs.push((npc.id.clone(), npc.x, npc.y));
                    // Update position in collision map
                    npc_positions.insert(npc.id.clone(), (npc.x, npc.y));
                }

                // Get positions of other NPCs (excluding self) for collision detection
                let other_npc_positions: Vec<(i32, i32)> = npc_positions
                    .iter()
                    .filter(|(id, _)| *id != &npc.id)
                    .map(|(_, pos)| *pos)
                    .collect();

                // Run NPC AI update
                if let Some((target_id, max_hit)) = npc.update(delta_time, &player_positions, &other_npc_positions, current_time) {
                    // Store NPC level and max hit for hit/miss calculation during attack processing
                    npc_attacks.push((npc.id.clone(), target_id, npc.level, max_hit));
                }

                // Update position in collision map after movement
                if npc.is_alive() {
                    npc_positions.insert(npc.id.clone(), (npc.x, npc.y));
                }

                // Add to updates (all NPCs including dead ones for client awareness)
                npc_updates.push(NpcUpdate::from(&*npc));
            }
        }

        // Process NPC attacks on players using hit/miss mechanics
        for (npc_id, target_id, npc_level, max_hit) in npc_attacks {
            let (target_hp, target_x, target_y, died, damage): (i32, f32, f32, bool, i32) = {
                let mut players = self.players.write().await;
                if let Some(target) = players.get_mut(&target_id) {
                    if target.is_dead {
                        continue; // Already dead
                    }
                    // God mode prevents all damage
                    if target.is_god_mode {
                        continue;
                    }

                    // NPC uses its level as attack level, no equipment bonuses
                    let npc_attack_level = npc_level;
                    let npc_attack_bonus = 0;

                    // Player uses their combat skill level and equipment bonus
                    let player_defence_level = target.skills.combat.level;
                    let player_defence_bonus = target.defence_bonus(&self.item_registry);

                    // Roll hit/miss
                    if !calculate_hit(npc_attack_level, npc_attack_bonus, player_defence_level, player_defence_bonus) {
                        // Miss - deal 0 damage
                        tracing::debug!(
                            "NPC {} misses {} (atk {} vs def {} + {})",
                            npc_id, target_id, npc_attack_level, player_defence_level, player_defence_bonus
                        );
                        (target.hp, target.x as f32, target.y as f32, false, 0)
                    } else {
                        // Hit - roll damage and apply
                        let damage = roll_damage(max_hit);
                        target.hp = (target.hp - damage).max(0);
                        let died = target.hp <= 0;
                        if died {
                            target.die(current_time);
                        }
                        tracing::debug!(
                            "NPC {} hits {} for {} damage (max: {}, HP: {})",
                            npc_id, target_id, damage, max_hit, target.hp
                        );
                        (target.hp, target.x as f32, target.y as f32, died, damage)
                    }
                } else {
                    continue;
                }
            };

            // Broadcast damage event
            self.broadcast(ServerMessage::DamageEvent {
                source_id: npc_id.clone(),
                target_id: target_id.clone(),
                damage,
                target_hp,
                target_x,
                target_y,
            }).await;

            // Handle player death
            if died {
                tracing::info!("NPC {} killed player {}", npc_id, target_id);
                self.broadcast(ServerMessage::PlayerDied {
                    id: target_id.clone(),
                    killer_id: npc_id.clone(),
                }).await;
            }
        }

        // Broadcast respawns
        for (id, x, y) in respawned_npcs {
            self.broadcast(ServerMessage::NpcRespawned { id, x, y }).await;
        }

        // Check for expired items (60 second lifetime)
        let expired_items: Vec<String> = {
            let items = self.ground_items.read().await;
            items.iter()
                .filter(|(_, item)| item.is_expired(current_time))
                .map(|(id, _)| id.clone())
                .collect()
        };

        // Remove and broadcast despawned items
        for item_id in expired_items {
            let mut items = self.ground_items.write().await;
            if items.remove(&item_id).is_some() {
                drop(items);
                self.broadcast(ServerMessage::ItemDespawned { item_id }).await;
            }
        }

        // Check for shop restocks (every 60 seconds)
        {
            let last_restock = *self.last_shop_restock.read().await;
            if last_restock.elapsed().as_secs() >= 60 {
                self.restock_shops().await;
                let mut last = self.last_shop_restock.write().await;
                *last = std::time::Instant::now();
            }
        }

        // Broadcast state sync (always include NPCs even if no players)
        let tick = *self.tick.read().await;
        self.broadcast(ServerMessage::StateSync {
            tick,
            players: player_updates,
            npcs: npc_updates,
        })
        .await;
    }

    /// Restock all shops that have restock intervals
    async fn restock_shops(&self) {
        let mut shop_registry = self.shop_registry.write().await;
        let npcs = self.npcs.read().await;

        // Collect shops that need restocking by checking NPCs with merchant configs
        let mut restocked_shops = Vec::new();

        for proto in self.entity_registry.all() {
            if let Some(merchant_config) = &proto.merchant {
                // Check if this merchant has a restock interval
                if merchant_config.restock_interval_minutes.is_some() {
                    // Get and restock the shop
                    if let Some(shop) = shop_registry.get_mut(&merchant_config.shop_id) {
                        shop.restock();

                        // Find all NPCs of this type to broadcast stock updates
                        let npc_ids: Vec<String> = npcs
                            .iter()
                            .filter(|(_, npc)| {
                                npc.prototype_id.as_ref() == Some(&proto.id)
                            })
                            .map(|(npc_id, _)| npc_id.clone())
                            .collect();

                        // Collect stock updates for broadcasting
                        for npc_id in npc_ids {
                            for stock_item in &shop.stock {
                                restocked_shops.push((
                                    npc_id.clone(),
                                    stock_item.item_id.clone(),
                                    stock_item.current_quantity,
                                ));
                            }
                        }

                        tracing::info!(
                            "Restocked shop '{}' for entity type '{}'",
                            merchant_config.shop_id,
                            proto.id
                        );
                    }
                }
            }
        }

        drop(shop_registry);
        drop(npcs);

        // Broadcast all stock updates
        for (npc_id, item_id, quantity) in restocked_shops {
            self.broadcast_shop_stock_update(&npc_id, &item_id, quantity).await;
        }
    }

    /// Handle chunk request from client
    pub async fn handle_chunk_request(&self, chunk_x: i32, chunk_y: i32) -> Option<ServerMessage> {
        use crate::protocol::{ChunkLayerData, ChunkObjectData};

        let coord = ChunkCoord::new(chunk_x, chunk_y);
        if let Some(chunk) = self.world.get_chunk_data(coord).await {
            let layers: Vec<ChunkLayerData> = chunk.layers.iter().map(|layer| {
                ChunkLayerData {
                    layer_type: layer.layer_type as u8,
                    tiles: layer.tiles.clone(),
                }
            }).collect();

            let collision = chunk.pack_collision();

            let objects: Vec<ChunkObjectData> = chunk.objects.iter().map(|obj| {
                ChunkObjectData {
                    gid: obj.gid,
                    tile_x: obj.tile_x,
                    tile_y: obj.tile_y,
                    width: obj.width,
                    height: obj.height,
                }
            }).collect();

            Some(ServerMessage::ChunkData {
                chunk_x,
                chunk_y,
                layers,
                collision,
                objects,
            })
        } else {
            Some(ServerMessage::ChunkNotFound { chunk_x, chunk_y })
        }
    }

    /// Get the World reference for chunk operations
    pub fn world(&self) -> &Arc<World> {
        &self.world
    }

    /// Update player's current chunk and return true if changed
    pub async fn update_player_chunk(&self, player_id: &str, new_chunk: ChunkCoord) -> bool {
        let mut chunks = self.player_chunks.write().await;
        let old_chunk = chunks.get(player_id).copied();
        if old_chunk != Some(new_chunk) {
            chunks.insert(player_id.to_string(), new_chunk);
            return true;
        }
        false
    }

    /// Generate entity definitions message for client sync
    pub fn get_entity_definitions(&self) -> ServerMessage {
        use crate::protocol::ClientEntityDef;

        let entities: Vec<ClientEntityDef> = self.entity_registry
            .all()
            .map(|proto| ClientEntityDef {
                id: proto.id.clone(),
                display_name: proto.display_name.clone(),
                sprite: proto.sprite.clone(),
                animation_type: format!("{:?}", proto.animation_type).to_lowercase(),
                max_hp: proto.stats.max_hp,
            })
            .collect();

        ServerMessage::EntityDefinitions { entities }
    }
}
