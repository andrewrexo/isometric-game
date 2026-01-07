use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::item::{self, GroundItem, Inventory, ItemType};
use crate::npc::{Npc, NpcState, NpcType, NpcUpdate};
use crate::protocol::ServerMessage;
use crate::tilemap::Tilemap;

// ============================================================================
// Constants
// ============================================================================

const TICK_RATE: f32 = 20.0;

// Grid-based movement: time between tile moves (250ms = 4 tiles/sec)
const MOVE_COOLDOWN_MS: u64 = 250;

const MAP_WIDTH: u32 = 32;
const MAP_HEIGHT: u32 = 32;
const STARTING_HP: i32 = 100;

// Combat constants
const ATTACK_RANGE: f32 = 1.5; // Maximum distance to attack (in tiles)
const ATTACK_COOLDOWN_MS: u64 = 1000; // 1 second between attacks
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
    pub last_move_time: u64, // For movement cooldown
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
            last_move_time: 0,
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
    tilemap: Tilemap,
    tick: RwLock<u64>,
    broadcast_tx: broadcast::Sender<ServerMessage>,
}

impl GameRoom {
    pub fn new(name: &str) -> Self {
        let (tx, _) = broadcast::channel(256);
        let tilemap = Tilemap::new_test_map(MAP_WIDTH, MAP_HEIGHT);

        // Spawn initial NPCs (grid positions)
        let mut npcs = HashMap::new();
        let npc_spawns: Vec<(i32, i32)> = vec![
            (10, 10),
            (12, 8),
            (8, 12),
            (20, 15),
            (15, 20),
        ];
        for (i, (x, y)) in npc_spawns.iter().enumerate() {
            let id = format!("npc_{}", i);
            let npc = Npc::new(&id, NpcType::Slime, *x, *y, 1);
            npcs.insert(id, npc);
        }

        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            players: RwLock::new(HashMap::new()),
            npcs: RwLock::new(npcs),
            ground_items: RwLock::new(HashMap::new()),
            tilemap,
            tick: RwLock::new(0),
            broadcast_tx: tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }

    pub async fn broadcast(&self, msg: ServerMessage) {
        // Ignore send errors (no receivers)
        let _ = self.broadcast_tx.send(msg);
    }

    pub async fn reserve_player(&self, player_id: &str, name: &str) {
        let mut players = self.players.write().await;
        let (spawn_x, spawn_y) = self.tilemap.get_safe_spawn();
        let player = Player::new(player_id, name, spawn_x, spawn_y);
        players.insert(player_id.to_string(), player);
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

        // Restore inventory from JSON
        if let Ok(slots) = serde_json::from_str::<Vec<(usize, u8, i32)>>(inventory_json) {
            for (slot_idx, item_type, quantity) in slots {
                if slot_idx < player.inventory.slots.len() {
                    let item = item::ItemType::from_u8(item_type);
                    player.inventory.slots[slot_idx] = Some(item::InventorySlot {
                        item_type: item,
                        quantity,
                    });
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
            // Serialize inventory to JSON
            let inventory_slots: Vec<(usize, u8, i32)> = p.inventory.slots
                .iter()
                .enumerate()
                .filter_map(|(idx, slot)| {
                    slot.as_ref().map(|s| (idx, s.item_type as u8, s.quantity))
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
                // Get EXP reward from NPC
                let exp_reward = {
                    let npcs = self.npcs.read().await;
                    npcs.get(&target_id).map(|n| n.exp_reward()).unwrap_or(0)
                };

                // Award EXP to killer
                let leveled_up = if exp_reward > 0 {
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

                // Spawn item drops
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let npc_type = {
                    let npcs = self.npcs.read().await;
                    npcs.get(&target_id).map(|n| n.npc_type).unwrap_or(NpcType::Slime)
                };

                let drops = item::generate_drops(npc_type, target_x, target_y, player_id, current_time);
                for item in drops {
                    // Broadcast item drop
                    let drop_msg = ServerMessage::ItemDropped {
                        id: item.id.clone(),
                        item_type: item.item_type as u8,
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

                Some((item.item_type, item.quantity))
            }).flatten()
        };

        if let Some((item_type, quantity)) = item_info {
            // Remove item from ground
            let removed = {
                let mut items = self.ground_items.write().await;
                items.remove(item_id).is_some()
            };

            if removed {
                tracing::debug!("Player {} picked up {} x{}", player_id, item_type.name(), quantity);

                // Add to player's inventory
                let (leftover, inventory_update, gold) = {
                    let mut players = self.players.write().await;
                    if let Some(player) = players.get_mut(player_id) {
                        let leftover = player.inventory.add_item(item_type, quantity);
                        (leftover, player.inventory.to_update(), player.inventory.gold)
                    } else {
                        return;
                    }
                };

                // Broadcast pickup
                let pickup_msg = ServerMessage::ItemPickedUp {
                    item_id: item_id.to_string(),
                    player_id: player_id.to_string(),
                };
                self.broadcast(pickup_msg).await;

                // Send inventory update to the player who picked up
                let inv_msg = ServerMessage::InventoryUpdate {
                    slots: inventory_update,
                    gold,
                };
                self.broadcast(inv_msg).await; // TODO: Send only to this player

                // If some items couldn't fit, drop them back on ground
                if leftover > 0 {
                    tracing::debug!("Inventory full, dropping {} back", leftover);
                    // Could spawn a new ground item here
                }
            }
        }
    }

    pub async fn handle_use_item(&self, player_id: &str, slot_index: u8) {
        // Get player and try to use item
        let (used_item, effect, inventory_update, gold) = {
            let mut players = self.players.write().await;
            if let Some(player) = players.get_mut(player_id) {
                // Dead players can't use items
                if player.is_dead {
                    return;
                }

                if let Some(item_type) = player.inventory.use_item(slot_index as usize) {
                    // Apply item effect
                    let effect = match item_type {
                        ItemType::HealthPotion => {
                            let heal = 30;
                            player.hp = (player.hp + heal).min(player.max_hp);
                            format!("heal:{}", heal)
                        }
                        ItemType::ManaPotion => {
                            // Mana not implemented yet
                            "mana:20".to_string()
                        }
                        _ => "none".to_string(),
                    };

                    let update = player.inventory.to_update();
                    (Some(item_type), effect, update, player.inventory.gold)
                } else {
                    return;
                }
            } else {
                return;
            }
        };

        if let Some(item_type) = used_item {
            tracing::debug!("Player {} used {} ({})", player_id, item_type.name(), effect);

            // Send item used message
            let used_msg = ServerMessage::ItemUsed {
                slot: slot_index,
                item_type: item_type as u8,
                effect,
            };
            self.broadcast(used_msg).await;

            // Send inventory update
            let inv_msg = ServerMessage::InventoryUpdate {
                slots: inventory_update,
                gold,
            };
            self.broadcast(inv_msg).await;
        }
    }

    pub async fn tick(&self) {
        let delta_time = 1.0 / TICK_RATE;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Update tick counter
        {
            let mut tick = self.tick.write().await;
            *tick += 1;
        }

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

        // Update player positions (grid-based, only alive players)
        let mut player_updates = Vec::new();
        {
            let mut players = self.players.write().await;
            for player in players.values_mut() {
                if !player.active {
                    continue;
                }

                // Dead players don't move
                if player.is_dead {
                    player_updates.push(PlayerUpdate {
                        id: player.id.clone(),
                        x: player.x,
                        y: player.y,
                        direction: player.direction as u8,
                        hp: player.hp,
                        max_hp: player.max_hp,
                        level: player.level,
                        exp: player.exp,
                        exp_to_next_level: player.exp_to_next_level,
                        gold: player.inventory.gold,
                    });
                    continue;
                }

                // Grid-based movement with cooldown
                if (player.move_dx != 0 || player.move_dy != 0)
                    && current_time - player.last_move_time >= MOVE_COOLDOWN_MS
                {
                    let target_x = player.x + player.move_dx;
                    let target_y = player.y + player.move_dy;

                    // Check if target tile is walkable
                    if self.tilemap.is_tile_walkable(target_x, target_y) {
                        player.x = target_x;
                        player.y = target_y;
                        player.last_move_time = current_time;
                    }
                }

                player_updates.push(PlayerUpdate {
                    id: player.id.clone(),
                    x: player.x,
                    y: player.y,
                    direction: player.direction as u8,
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
}
