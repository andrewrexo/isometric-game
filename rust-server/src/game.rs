use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::chunk::ChunkCoord;
use crate::entity::{EntityPrototype, EntityRegistry};
use crate::data::ItemRegistry;
use crate::item::{self, GroundItem, Inventory, GOLD_ITEM_ID};
use crate::npc::{Npc, NpcType, NpcUpdate};
use crate::protocol::{ServerMessage, QuestObjectiveData};
use crate::quest::{QuestRegistry, QuestRunner, PlayerQuestState, QuestEvent};
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
const ATTACK_RANGE: f32 = 1.5; // Maximum distance to attack (in tiles)
const ATTACK_COOLDOWN_MS: u64 = 800; // 1 second between attacks
const BASE_DAMAGE: i32 = 10; // Base damage per attack

// ============================================================================
// Player Save Data (for database persistence)
// ============================================================================

#[derive(Debug, Clone)]
pub struct PlayerSaveData {
    pub x: f32,
    pub y: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub level: i32,
    pub exp: i32,
    pub exp_to_next_level: i32,
    pub gold: i32,
    pub inventory_json: String,
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
    pub max_hp: i32,
    pub level: i32,
    pub exp: i32,
    pub exp_to_next_level: i32,
    pub active: bool, // Whether WebSocket is connected
    pub target_id: Option<String>, // Currently targeted entity (player or NPC)
    pub last_attack_time: u64, // Timestamp of last attack (ms)
    pub is_dead: bool,
    pub death_time: u64, // When the player died (for respawn timer)
    pub inventory: Inventory,
}

const PLAYER_RESPAWN_TIME_MS: u64 = 5000; // 5 seconds to respawn

/// Calculate EXP required for a given level
fn exp_for_level(level: i32) -> i32 {
    // Simple formula: 100 * level^1.5
    (100.0 * (level as f32).powf(1.5)) as i32
}

impl Player {
    pub fn new(id: &str, name: &str, spawn_x: i32, spawn_y: i32) -> Self {
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
            hp: STARTING_HP,
            max_hp: STARTING_HP,
            level: 1,
            exp: 0,
            exp_to_next_level: exp_for_level(1),
            active: false,
            target_id: None,
            last_attack_time: 0,
            is_dead: false,
            death_time: 0,
            inventory: Inventory::new(),
        }
    }

    /// Award EXP and handle level up. Returns true if leveled up.
    pub fn award_exp(&mut self, amount: i32) -> bool {
        self.exp += amount;

        // Check for level up
        if self.exp >= self.exp_to_next_level {
            self.exp -= self.exp_to_next_level;
            self.level += 1;
            self.exp_to_next_level = exp_for_level(self.level);

            // Level up bonuses: +10 max HP, full heal
            self.max_hp += 10;
            self.hp = self.max_hp;

            tracing::info!("{} leveled up to {}! (Max HP: {})", self.name, self.level, self.max_hp);
            return true;
        }
        false
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
        self.hp = self.max_hp;
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
    pub level: i32,
    pub exp: i32,
    pub exp_to_next_level: i32,
    pub gold: i32,
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

    pub async fn reserve_player(&self, player_id: &str, name: &str) {
        let (spawn_x, spawn_y) = self.world.get_spawn_position().await;
        let mut players = self.players.write().await;
        let player = Player::new(player_id, name, spawn_x, spawn_y);
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
        max_hp: i32,
        level: i32,
        exp: i32,
        exp_to_next_level: i32,
        gold: i32,
        inventory_json: &str,
    ) {
        let mut players = self.players.write().await;
        let mut player = Player::new(player_id, name, x, y);

        // Restore saved stats
        player.hp = hp;
        player.max_hp = max_hp;
        player.level = level;
        player.exp = exp;
        player.exp_to_next_level = exp_to_next_level;
        player.inventory.gold = gold;

        // Restore inventory from JSON - support both old (u8) and new (String) formats
        if let Ok(slots) = serde_json::from_str::<Vec<(usize, String, i32)>>(inventory_json) {
            // New format: (slot_idx, item_id, quantity)
            for (slot_idx, item_id, quantity) in slots {
                if slot_idx < player.inventory.slots.len() {
                    player.inventory.slots[slot_idx] = Some(item::InventorySlot::new(item_id, quantity));
                }
            }
        } else if let Ok(slots) = serde_json::from_str::<Vec<(usize, u8, i32)>>(inventory_json) {
            // Legacy format: (slot_idx, item_type_u8, quantity) - migrate to string IDs
            for (slot_idx, item_type_u8, quantity) in slots {
                if slot_idx < player.inventory.slots.len() {
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
            "Restored player {} at ({}, {}) with {} HP, level {}, {} gold",
            name, x, y, hp, level, gold
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
            let inventory_slots: Vec<(usize, String, i32)> = p.inventory.slots
                .iter()
                .enumerate()
                .filter_map(|(idx, slot)| {
                    slot.as_ref().map(|s| (idx, s.item_id.clone(), s.quantity))
                })
                .collect();
            let inventory_json = serde_json::to_string(&inventory_slots).unwrap_or_else(|_| "[]".to_string());

            PlayerSaveData {
                x: p.x as f32,
                y: p.y as f32,
                hp: p.hp,
                max_hp: p.max_hp,
                level: p.level,
                exp: p.exp,
                exp_to_next_level: p.exp_to_next_level,
                gold: p.inventory.gold,
                inventory_json,
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

    pub async fn handle_chat(&self, player_id: &str, text: &str) {
        let players = self.players.read().await;
        if let Some(player) = players.get(player_id) {
            let sanitized = text.trim().chars().take(200).collect::<String>();
            if !sanitized.is_empty() {
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
    }

    pub async fn handle_attack(&self, player_id: &str) {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Get attacker info
        let (attacker_name, attacker_x, attacker_y, attacker_dir, last_attack) = {
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

            (player.name.clone(), player.x, player.y, player.direction, player.last_attack_time)
        };

        // Check cooldown
        if current_time - last_attack < ATTACK_COOLDOWN_MS {
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

        // Update attacker's last attack time
        {
            let mut players = self.players.write().await;
            if let Some(player) = players.get_mut(player_id) {
                player.last_attack_time = current_time;
            }
        }

        // Apply damage to target
        let (target_hp, target_name, target_died) = if is_npc {
            let mut npcs = self.npcs.write().await;
            if let Some(npc) = npcs.get_mut(&target_id) {
                let died = npc.take_damage(BASE_DAMAGE, current_time);
                let name = npc.name();
                tracing::info!(
                    "{} deals {} damage to {} (HP: {})",
                    attacker_name, BASE_DAMAGE, name, npc.hp
                );
                (npc.hp, name, died)
            } else {
                return;
            }
        } else {
            let mut players = self.players.write().await;
            if let Some(target) = players.get_mut(&target_id) {
                if target.is_dead {
                    return; // Already dead
                }
                target.hp = (target.hp - BASE_DAMAGE).max(0);
                let name = target.name.clone();
                let died = target.hp <= 0;
                if died {
                    target.die(current_time);
                }
                tracing::info!(
                    "{} deals {} damage to {} (HP: {})",
                    attacker_name, BASE_DAMAGE, name, target.hp
                );
                (target.hp, name, died)
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
            damage: BASE_DAMAGE,
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

                // Award EXP to killer
                let _leveled_up = if exp_reward > 0 {
                    let mut players = self.players.write().await;
                    if let Some(killer) = players.get_mut(player_id) {
                        let leveled = killer.award_exp(exp_reward);

                        // Send ExpGained message
                        let exp_msg = ServerMessage::ExpGained {
                            player_id: player_id.to_string(),
                            amount: exp_reward,
                            total_exp: killer.exp,
                            exp_to_next_level: killer.exp_to_next_level,
                        };
                        drop(players);
                        self.broadcast(exp_msg).await;

                        if leveled {
                            // Get updated player info for LevelUp message
                            let players = self.players.read().await;
                            if let Some(killer) = players.get(player_id) {
                                let level_msg = ServerMessage::LevelUp {
                                    player_id: player_id.to_string(),
                                    new_level: killer.level,
                                    new_max_hp: killer.max_hp,
                                };
                                drop(players);
                                self.broadcast(level_msg).await;
                            }
                        }
                        leveled
                    } else {
                        false
                    }
                } else {
                    false
                };

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
                    // Broadcast item drop
                    let drop_msg = ServerMessage::ItemDropped {
                        id: item.id.clone(),
                        item_id: item.item_id.clone(),
                        x: item.x,
                        y: item.y,
                        quantity: item.quantity,
                    };
                    self.broadcast(drop_msg).await;

                    // Store in ground_items
                    let mut items = self.ground_items.write().await;
                    items.insert(item.id.clone(), item);
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
                                player.hp = (player.hp + amount).min(player.max_hp);
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

        // Check level requirement
        if player.level < recipe.level_required {
            drop(players);
            self.send_to_player(
                player_id,
                ServerMessage::CraftResult {
                    success: false,
                    recipe_id: recipe_id.to_string(),
                    error: Some(format!("Requires level {}", recipe.level_required)),
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
                    max_hp: player.max_hp,
                    level: player.level,
                    exp: player.exp,
                    exp_to_next_level: player.exp_to_next_level,
                    gold: player.inventory.gold,
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
        let mut npc_attacks: Vec<(String, String, i32)> = Vec::new(); // (npc_id, target_id, damage)
        {
            let mut npcs = self.npcs.write().await;
            for npc in npcs.values_mut() {
                // Check for respawn
                if npc.ready_to_respawn(current_time) {
                    npc.respawn();
                    respawned_npcs.push((npc.id.clone(), npc.x, npc.y));
                }

                // Run NPC AI update
                if let Some((target_id, damage)) = npc.update(delta_time, &player_positions, current_time) {
                    npc_attacks.push((npc.id.clone(), target_id, damage));
                }

                // Add to updates (all NPCs including dead ones for client awareness)
                npc_updates.push(NpcUpdate::from(&*npc));
            }
        }

        // Process NPC attacks on players
        for (npc_id, target_id, damage) in npc_attacks {
            let (target_hp, target_x, target_y, died): (i32, f32, f32, bool) = {
                let mut players = self.players.write().await;
                if let Some(target) = players.get_mut(&target_id) {
                    if target.is_dead {
                        continue; // Already dead
                    }
                    target.hp = (target.hp - damage).max(0);
                    let died = target.hp <= 0;
                    if died {
                        target.die(current_time);
                    }
                    (target.hp, target.x as f32, target.y as f32, died)
                } else {
                    continue;
                }
            };

            tracing::debug!(
                "NPC {} attacks {} for {} damage (HP: {})",
                npc_id, target_id, damage, target_hp
            );

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

        // Broadcast state sync (always include NPCs even if no players)
        let tick = *self.tick.read().await;
        self.broadcast(ServerMessage::StateSync {
            tick,
            players: player_updates,
            npcs: npc_updates,
        })
        .await;
    }

    /// Handle chunk request from client
    pub async fn handle_chunk_request(&self, chunk_x: i32, chunk_y: i32) -> Option<ServerMessage> {
        use crate::protocol::ChunkLayerData;

        let coord = ChunkCoord::new(chunk_x, chunk_y);
        if let Some(chunk) = self.world.get_chunk_data(coord).await {
            let layers: Vec<ChunkLayerData> = chunk.layers.iter().map(|layer| {
                ChunkLayerData {
                    layer_type: layer.layer_type as u8,
                    tiles: layer.tiles.clone(),
                }
            }).collect();

            let collision = chunk.pack_collision();

            Some(ServerMessage::ChunkData {
                chunk_x,
                chunk_y,
                layers,
                collision,
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
