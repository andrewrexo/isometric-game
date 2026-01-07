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

#[derive(Debug, Clone)]
pub struct Npc {
    pub id: String,
    pub npc_type: NpcType,
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

    pub fn name(&self) -> String {
        let stats = self.npc_type.stats();
        format!("{} Lv.{}", stats.name, self.level)
    }

    pub fn exp_reward(&self) -> i32 {
        let stats = self.npc_type.stats();
        // Scale EXP by NPC level
        stats.exp_reward * self.level
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
        let stats = self.npc_type.stats();
        current_time - self.death_time >= stats.respawn_time_ms
    }

    /// Respawn the NPC at its spawn point
    pub fn respawn(&mut self) {
        let stats = self.npc_type.stats();
        self.x = self.spawn_x;
        self.y = self.spawn_y;
        self.hp = stats.max_hp;
        self.state = NpcState::Idle;
        self.target_id = None;
        self.last_attack_time = 0;
        self.last_move_time = 0;
    }

    /// Calculate grid distance (Chebyshev - allows diagonal)
    fn grid_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
        (x1 - x2).abs().max((y1 - y2).abs())
    }

    /// Try to move one tile toward target position (grid-based)
    /// Returns true if moved
    fn try_move_toward(&mut self, target_x: i32, target_y: i32, current_time: u64) -> bool {
        let stats = self.npc_type.stats();

        // Check movement cooldown
        if current_time - self.last_move_time < stats.move_cooldown_ms {
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

        self.x += move_x;
        self.y += move_y;
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
        current_time: u64,
    ) -> Option<(String, i32)> {
        if self.state == NpcState::Dead {
            return None;
        }

        let stats = self.npc_type.stats();
        let mut attack_result = None;

        match self.state {
            NpcState::Idle => {
                // Look for players in aggro range
                let mut nearest: Option<(String, i32)> = None;
                for (player_id, px, py, hp) in players {
                    if *hp <= 0 {
                        continue; // Skip dead players
                    }
                    let dist = Self::grid_distance(self.x, self.y, *px, *py);
                    if dist <= stats.aggro_range {
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
                    let dist = Self::grid_distance(self.x, self.y, tx, ty);
                    let spawn_dist = Self::grid_distance(self.x, self.y, self.spawn_x, self.spawn_y);

                    if spawn_dist > stats.chase_range {
                        // Too far from spawn, return home
                        self.state = NpcState::Returning;
                        self.target_id = None;
                    } else if dist <= stats.attack_range {
                        // In attack range
                        self.state = NpcState::Attacking;
                    } else {
                        // Move toward target (one tile at a time)
                        self.try_move_toward(tx, ty, current_time);
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
                    let dist = Self::grid_distance(self.x, self.y, tx, ty);

                    if dist > stats.attack_range {
                        // Target moved out of range, chase again
                        self.state = NpcState::Chasing;
                    } else {
                        // Face target
                        let dx = tx - self.x;
                        let dy = ty - self.y;
                        if dx != 0 || dy != 0 {
                            self.direction = crate::game::Direction::from_velocity(dx as f32, dy as f32);
                        }

                        // Attack if cooldown is ready
                        if current_time - self.last_attack_time >= stats.attack_cooldown_ms {
                            self.last_attack_time = current_time;
                            attack_result = Some((target_id, stats.damage));
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
                    self.hp = stats.max_hp;
                    self.state = NpcState::Idle;
                } else {
                    // Move toward spawn (one tile at a time)
                    self.try_move_toward(self.spawn_x, self.spawn_y, current_time);
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
    pub x: i32,  // Grid position
    pub y: i32,  // Grid position
    pub direction: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub level: i32,
    pub state: u8,
}

impl From<&Npc> for NpcUpdate {
    fn from(npc: &Npc) -> Self {
        Self {
            id: npc.id.clone(),
            npc_type: npc.npc_type as u8,
            x: npc.x,
            y: npc.y,
            direction: npc.direction as u8,
            hp: npc.hp,
            max_hp: npc.max_hp,
            level: npc.level,
            state: npc.state as u8,
        }
    }
}
