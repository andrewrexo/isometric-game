use serde::Serialize;
use crate::game::Direction;

// ============================================================================
// NPC Types and Stats
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Slime,
}

impl NpcType {
    pub fn stats(&self) -> NpcStats {
        match self {
            NpcType::Slime => NpcStats {
                name: "Slime",
                max_hp: 50,
                damage: 5,
                attack_range: 1,      // Must be adjacent
                aggro_range: 5,       // Aggro within 5 tiles
                chase_range: 8,       // Chase up to 8 tiles from spawn
                move_cooldown_ms: 500, // Moves every 500ms (2 tiles/sec, slower than player)
                attack_cooldown_ms: 1500,
                respawn_time_ms: 10000,
                exp_reward: 25,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct NpcStats {
    pub name: &'static str,
    pub max_hp: i32,
    pub damage: i32,
    pub attack_range: i32,    // Grid tiles
    pub aggro_range: i32,     // Grid tiles
    pub chase_range: i32,     // Grid tiles from spawn
    pub move_cooldown_ms: u64, // Time between grid moves
    pub attack_cooldown_ms: u64,
    pub respawn_time_ms: u64,
    pub exp_reward: i32,
}

// ============================================================================
// NPC State
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcState {
    Idle,
    Chasing,
    Attacking,
    Returning,
    Dead,
}

// ============================================================================
// NPC Entity
// ============================================================================

/// Stats from prototype used for AI behavior
#[derive(Debug, Clone)]
pub struct PrototypeStats {
    pub display_name: String,
    pub damage: i32,
    pub attack_range: i32,
    pub aggro_range: i32,
    pub chase_range: i32,
    pub move_cooldown_ms: u64,
    pub attack_cooldown_ms: u64,
    pub respawn_time_ms: u64,
    pub exp_base: i32,
    pub hostile: bool,
}

#[derive(Debug, Clone)]
pub struct Npc {
    pub id: String,
    pub npc_type: NpcType,
    /// Reference to entity prototype ID (e.g., "slime", "slime_king")
    pub prototype_id: Option<String>,
    /// Stats from prototype (overrides npc_type stats when present)
    pub proto_stats: Option<PrototypeStats>,
    // Grid position (server authoritative)
    pub x: i32,
    pub y: i32,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub direction: Direction,
    pub hp: i32,
    pub max_hp: i32,
    pub level: i32,
    pub state: NpcState,
    pub target_id: Option<String>, // Player it's aggro'd on
    pub last_attack_time: u64,
    pub last_move_time: u64, // For grid movement cooldown
    pub death_time: u64, // When the NPC died (for respawn)
}

impl Npc {
    pub fn new(id: &str, npc_type: NpcType, x: i32, y: i32, level: i32) -> Self {
        let stats = npc_type.stats();
        Self {
            id: id.to_string(),
            npc_type,
            prototype_id: None,
            proto_stats: None,
            x,
            y,
            spawn_x: x,
            spawn_y: y,
            direction: Direction::Down,
            hp: stats.max_hp,
            max_hp: stats.max_hp,
            level,
            state: NpcState::Idle,
            target_id: None,
            last_attack_time: 0,
            last_move_time: 0,
            death_time: 0,
        }
    }

    /// Create an NPC from an entity prototype
    pub fn from_prototype(
        id: &str,
        prototype_id: &str,
        prototype: &crate::entity::EntityPrototype,
        x: i32,
        y: i32,
        level: i32,
    ) -> Self {
        let proto_stats = PrototypeStats {
            display_name: prototype.display_name.clone(),
            damage: prototype.stats.damage,
            attack_range: prototype.stats.attack_range,
            aggro_range: prototype.stats.aggro_range,
            chase_range: prototype.stats.chase_range,
            move_cooldown_ms: prototype.stats.move_cooldown_ms,
            attack_cooldown_ms: prototype.stats.attack_cooldown_ms,
            respawn_time_ms: prototype.stats.respawn_time_ms,
            exp_base: prototype.rewards.exp_base,
            hostile: prototype.behaviors.hostile,
        };

        Self {
            id: id.to_string(),
            npc_type: NpcType::Slime, // Fallback type for compatibility
            prototype_id: Some(prototype_id.to_string()),
            proto_stats: Some(proto_stats),
            x,
            y,
            spawn_x: x,
            spawn_y: y,
            direction: Direction::Down,
            hp: prototype.stats.max_hp,
            max_hp: prototype.stats.max_hp,
            level,
            state: NpcState::Idle,
            target_id: None,
            last_attack_time: 0,
            last_move_time: 0,
            death_time: 0,
        }
    }

    // Helper methods to get stats (prefer prototype stats over npc_type stats)
    fn get_damage(&self) -> i32 {
        self.proto_stats.as_ref().map(|s| s.damage).unwrap_or_else(|| self.npc_type.stats().damage)
    }

    fn get_attack_range(&self) -> i32 {
        self.proto_stats.as_ref().map(|s| s.attack_range).unwrap_or_else(|| self.npc_type.stats().attack_range)
    }

    fn get_aggro_range(&self) -> i32 {
        self.proto_stats.as_ref().map(|s| s.aggro_range).unwrap_or_else(|| self.npc_type.stats().aggro_range)
    }

    fn get_chase_range(&self) -> i32 {
        self.proto_stats.as_ref().map(|s| s.chase_range).unwrap_or_else(|| self.npc_type.stats().chase_range)
    }

    fn get_move_cooldown_ms(&self) -> u64 {
        self.proto_stats.as_ref().map(|s| s.move_cooldown_ms).unwrap_or_else(|| self.npc_type.stats().move_cooldown_ms)
    }

    fn get_attack_cooldown_ms(&self) -> u64 {
        self.proto_stats.as_ref().map(|s| s.attack_cooldown_ms).unwrap_or_else(|| self.npc_type.stats().attack_cooldown_ms)
    }

    fn get_respawn_time_ms(&self) -> u64 {
        self.proto_stats.as_ref().map(|s| s.respawn_time_ms).unwrap_or_else(|| self.npc_type.stats().respawn_time_ms)
    }

    pub fn is_hostile(&self) -> bool {
        self.proto_stats.as_ref().map(|s| s.hostile).unwrap_or(true)
    }

    pub fn name(&self) -> String {
        if let Some(ref stats) = self.proto_stats {
            format!("{} Lv.{}", stats.display_name, self.level)
        } else {
            let stats = self.npc_type.stats();
            format!("{} Lv.{}", stats.name, self.level)
        }
    }

    pub fn exp_reward(&self) -> i32 {
        let base = self.proto_stats.as_ref().map(|s| s.exp_base).unwrap_or_else(|| self.npc_type.stats().exp_reward);
        // Scale EXP by NPC level
        base * self.level
    }

    pub fn is_alive(&self) -> bool {
        self.state != NpcState::Dead
    }

    /// Take damage and return true if the NPC died
    pub fn take_damage(&mut self, damage: i32, current_time: u64) -> bool {
        self.hp = (self.hp - damage).max(0);
        if self.hp <= 0 {
            self.state = NpcState::Dead;
            self.death_time = current_time;
            self.target_id = None;
            true
        } else {
            false
        }
    }

    /// Check if ready to respawn
    pub fn ready_to_respawn(&self, current_time: u64) -> bool {
        if self.state != NpcState::Dead {
            return false;
        }
        current_time - self.death_time >= self.get_respawn_time_ms()
    }

    /// Respawn the NPC at its spawn point
    pub fn respawn(&mut self) {
        self.x = self.spawn_x;
        self.y = self.spawn_y;
        self.hp = self.max_hp;
        self.state = NpcState::Idle;
        self.target_id = None;
        self.last_attack_time = 0;
        self.last_move_time = 0;
    }

    /// Calculate grid distance (Chebyshev - allows diagonal)
    fn grid_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
        (x1 - x2).abs().max((y1 - y2).abs())
    }

    /// Check if target is within attack range AND in a cardinal direction (not diagonal)
    fn is_in_attack_range(x1: i32, y1: i32, x2: i32, y2: i32, range: i32) -> bool {
        let dx = (x1 - x2).abs();
        let dy = (y1 - y2).abs();
        // Must be cardinal (one axis is 0) and within range
        (dx == 0 || dy == 0) && (dx + dy) <= range
    }

    /// Try to move one tile toward target position (grid-based)
    /// Returns true if moved
    fn try_move_toward(
        &mut self,
        target_x: i32,
        target_y: i32,
        current_time: u64,
        occupied_tiles: &[(i32, i32)],
    ) -> bool {
        // Check movement cooldown
        if current_time - self.last_move_time < self.get_move_cooldown_ms() {
            return false;
        }

        let dx = target_x - self.x;
        let dy = target_y - self.y;

        // Already at target
        if dx == 0 && dy == 0 {
            return false;
        }

        // Move one tile (cardinal directions only for cleaner movement)
        let (move_x, move_y) = if dx.abs() > dy.abs() {
            (dx.signum(), 0)
        } else if dy != 0 {
            (0, dy.signum())
        } else {
            (dx.signum(), 0)
        };

        let new_x = self.x + move_x;
        let new_y = self.y + move_y;

        // Check if target tile is occupied by another NPC
        if occupied_tiles.iter().any(|(ox, oy)| *ox == new_x && *oy == new_y) {
            return false;
        }

        self.x = new_x;
        self.y = new_y;
        self.last_move_time = current_time;

        // Update facing direction
        self.direction = crate::game::Direction::from_velocity(move_x as f32, move_y as f32);

        true
    }

    /// Update NPC AI state and movement
    /// Returns Some((target_id, damage)) if the NPC attacks a player
    pub fn update(
        &mut self,
        _delta: f32, // Not used for grid movement
        players: &[(String, i32, i32, i32)], // (id, x, y, hp) - grid positions
        other_npc_positions: &[(i32, i32)],  // positions of other NPCs (excluding self)
        current_time: u64,
    ) -> Option<(String, i32)> {
        if self.state == NpcState::Dead {
            return None;
        }

        // Non-hostile NPCs never attack or aggro
        if !self.is_hostile() {
            return None;
        }

        let mut attack_result = None;

        match self.state {
            NpcState::Idle => {
                // Look for players in aggro range
                let aggro_range = self.get_aggro_range();
                if aggro_range <= 0 {
                    return None; // No aggro range = peaceful NPC
                }

                let mut nearest: Option<(String, i32)> = None;
                for (player_id, px, py, hp) in players {
                    if *hp <= 0 {
                        continue; // Skip dead players
                    }
                    let dist = Self::grid_distance(self.x, self.y, *px, *py);
                    if dist <= aggro_range {
                        if nearest.is_none() || dist < nearest.as_ref().unwrap().1 {
                            nearest = Some((player_id.clone(), dist));
                        }
                    }
                }

                if let Some((target_id, _)) = nearest {
                    self.target_id = Some(target_id);
                    self.state = NpcState::Chasing;
                }
            }

            NpcState::Chasing => {
                // Check if target is still valid
                let target_pos = self.target_id.as_ref().and_then(|tid| {
                    players.iter()
                        .find(|(id, _, _, hp)| id == tid && *hp > 0)
                        .map(|(_, x, y, _)| (*x, *y))
                });

                if let Some((tx, ty)) = target_pos {
                    let spawn_dist = Self::grid_distance(self.x, self.y, self.spawn_x, self.spawn_y);

                    if spawn_dist > self.get_chase_range() {
                        // Too far from spawn, return home
                        self.state = NpcState::Returning;
                        self.target_id = None;
                    } else if Self::is_in_attack_range(self.x, self.y, tx, ty, self.get_attack_range()) {
                        // In attack range (cardinal direction only)
                        self.state = NpcState::Attacking;
                    } else {
                        // Move toward target (one tile at a time)
                        self.try_move_toward(tx, ty, current_time, other_npc_positions);
                    }
                } else {
                    // Target lost (died or disconnected)
                    self.state = NpcState::Returning;
                    self.target_id = None;
                }
            }

            NpcState::Attacking => {
                // Check if target is still in range
                let target_info = self.target_id.as_ref().and_then(|tid| {
                    players.iter()
                        .find(|(id, _, _, hp)| id == tid && *hp > 0)
                        .map(|(id, x, y, _)| (id.clone(), *x, *y))
                });

                if let Some((target_id, tx, ty)) = target_info {
                    if !Self::is_in_attack_range(self.x, self.y, tx, ty, self.get_attack_range()) {
                        // Target moved out of range or not in cardinal direction, chase again
                        self.state = NpcState::Chasing;
                    } else {
                        // Face target
                        let dx = tx - self.x;
                        let dy = ty - self.y;
                        if dx != 0 || dy != 0 {
                            self.direction = crate::game::Direction::from_velocity(dx as f32, dy as f32);
                        }

                        // Attack if cooldown is ready
                        if current_time - self.last_attack_time >= self.get_attack_cooldown_ms() {
                            self.last_attack_time = current_time;
                            attack_result = Some((target_id, self.get_damage()));
                        }
                    }
                } else {
                    // Target lost
                    self.state = NpcState::Returning;
                    self.target_id = None;
                }
            }

            NpcState::Returning => {
                let dist = Self::grid_distance(self.x, self.y, self.spawn_x, self.spawn_y);

                if dist == 0 {
                    // Reached spawn, go idle and heal
                    self.hp = self.max_hp;
                    self.state = NpcState::Idle;
                } else {
                    // Move toward spawn (one tile at a time)
                    self.try_move_toward(self.spawn_x, self.spawn_y, current_time, other_npc_positions);
                }
            }

            NpcState::Dead => {}
        }

        attack_result
    }
}

// ============================================================================
// NPC Update for Network Sync
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct NpcUpdate {
    pub id: String,
    pub npc_type: u8,
    /// Entity prototype ID (e.g., "slime", "slime_king") for client-side lookup
    pub entity_type: String,
    /// Display name to show above NPC
    pub display_name: String,
    pub x: i32,  // Grid position
    pub y: i32,  // Grid position
    pub direction: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub level: i32,
    pub state: u8,
    /// Whether this NPC is hostile
    pub hostile: bool,
}

impl From<&Npc> for NpcUpdate {
    fn from(npc: &Npc) -> Self {
        let display_name = npc.proto_stats.as_ref()
            .map(|s| s.display_name.clone())
            .unwrap_or_else(|| npc.npc_type.stats().name.to_string());

        Self {
            id: npc.id.clone(),
            npc_type: npc.npc_type as u8,
            entity_type: npc.prototype_id.clone().unwrap_or_else(|| "slime".to_string()),
            display_name,
            x: npc.x,
            y: npc.y,
            direction: npc.direction as u8,
            hp: npc.hp,
            max_hp: npc.max_hp,
            level: npc.level,
            state: npc.state as u8,
            hostile: npc.is_hostile(),
        }
    }
}
