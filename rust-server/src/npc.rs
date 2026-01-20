use serde::Serialize;
use crate::game::Direction;

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
    pub sprite: String,
    pub damage: i32,
    pub attack_range: i32,
    pub aggro_range: i32,
    pub chase_range: i32,
    pub move_cooldown_ms: u64,
    pub attack_cooldown_ms: u64,
    pub respawn_time_ms: u64,
    pub exp_base: i32,
    pub hostile: bool,
    pub is_quest_giver: bool,
    pub is_merchant: bool,
}

#[derive(Debug, Clone)]
pub struct Npc {
    pub id: String,
    /// Reference to entity prototype ID (e.g., "pig", "elder_villager")
    pub prototype_id: String,
    /// Stats from prototype
    pub stats: PrototypeStats,
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
    /// Set to true on the tick when this NPC attacks, for client animation sync
    pub just_attacked: bool,
}

impl Npc {
    /// Create an NPC from an entity prototype
    pub fn from_prototype(
        id: &str,
        prototype_id: &str,
        prototype: &crate::entity::EntityPrototype,
        x: i32,
        y: i32,
        level: i32,
    ) -> Self {
        let stats = PrototypeStats {
            display_name: prototype.display_name.clone(),
            sprite: prototype.sprite.clone(),
            damage: prototype.stats.damage,
            attack_range: prototype.stats.attack_range,
            aggro_range: prototype.stats.aggro_range,
            chase_range: prototype.stats.chase_range,
            move_cooldown_ms: prototype.stats.move_cooldown_ms,
            attack_cooldown_ms: prototype.stats.attack_cooldown_ms,
            respawn_time_ms: prototype.stats.respawn_time_ms,
            exp_base: prototype.rewards.exp_base,
            hostile: prototype.behaviors.hostile,
            is_quest_giver: prototype.behaviors.quest_giver,
            is_merchant: prototype.behaviors.merchant,
        };

        Self {
            id: id.to_string(),
            prototype_id: prototype_id.to_string(),
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
            just_attacked: false,
            stats,
        }
    }

    // Helper methods to get stats
    fn get_damage(&self) -> i32 {
        self.stats.damage
    }

    fn get_attack_range(&self) -> i32 {
        self.stats.attack_range
    }

    fn get_aggro_range(&self) -> i32 {
        self.stats.aggro_range
    }

    fn get_chase_range(&self) -> i32 {
        self.stats.chase_range
    }

    fn get_move_cooldown_ms(&self) -> u64 {
        self.stats.move_cooldown_ms
    }

    fn get_attack_cooldown_ms(&self) -> u64 {
        self.stats.attack_cooldown_ms
    }

    fn get_respawn_time_ms(&self) -> u64 {
        self.stats.respawn_time_ms
    }

    pub fn is_hostile(&self) -> bool {
        self.stats.hostile
    }

    pub fn is_quest_giver(&self) -> bool {
        self.stats.is_quest_giver
    }

    pub fn is_merchant(&self) -> bool {
        self.stats.is_merchant
    }

    pub fn name(&self) -> String {
        format!("{} Lv.{}", self.stats.display_name, self.level)
    }

    pub fn exp_reward(&self) -> i32 {
        // Scale EXP by NPC level
        self.stats.exp_base * self.level
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

        // Build list of candidate moves in priority order
        let mut candidates: Vec<(i32, i32)> = Vec::with_capacity(4);

        // Primary direction: move along axis with greater distance
        if dx.abs() > dy.abs() {
            candidates.push((dx.signum(), 0)); // Primary: horizontal
            if dy != 0 {
                candidates.push((0, dy.signum())); // Secondary: vertical toward target
            }
        } else if dy.abs() > dx.abs() {
            candidates.push((0, dy.signum())); // Primary: vertical
            if dx != 0 {
                candidates.push((dx.signum(), 0)); // Secondary: horizontal toward target
            }
        } else {
            // Equal distance on both axes - try both
            if dx != 0 {
                candidates.push((dx.signum(), 0));
            }
            if dy != 0 {
                candidates.push((0, dy.signum()));
            }
        }

        // Add perpendicular moves as last resort (to go around obstacles)
        // These don't move us closer but allow pathfinding around blockers
        if dy == 0 && dx != 0 {
            // Moving horizontally, try vertical sidesteps
            candidates.push((0, 1));
            candidates.push((0, -1));
        } else if dx == 0 && dy != 0 {
            // Moving vertically, try horizontal sidesteps
            candidates.push((1, 0));
            candidates.push((-1, 0));
        }

        // Try each candidate move
        for (move_x, move_y) in candidates {
            let new_x = self.x + move_x;
            let new_y = self.y + move_y;

            // Check if tile is occupied by another NPC
            if occupied_tiles.iter().any(|(ox, oy)| *ox == new_x && *oy == new_y) {
                continue; // Try next candidate
            }

            // Found a valid move
            self.x = new_x;
            self.y = new_y;
            self.last_move_time = current_time;

            // Update facing direction
            self.direction = crate::game::Direction::from_velocity(move_x as f32, move_y as f32);

            return true;
        }

        false // All moves blocked
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
        // Reset attack flag each tick - will be set to true if we attack this tick
        self.just_attacked = false;

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
                    let movement_done = current_time - self.last_move_time >= self.get_move_cooldown_ms();

                    if spawn_dist > self.get_chase_range() {
                        // Too far from spawn, return home
                        self.state = NpcState::Returning;
                        self.target_id = None;
                    } else if Self::is_in_attack_range(self.x, self.y, tx, ty, self.get_attack_range()) && movement_done {
                        // In attack range (cardinal direction only) and movement completed
                        self.state = NpcState::Attacking;
                    } else if !Self::is_in_attack_range(self.x, self.y, tx, ty, self.get_attack_range()) {
                        // Not in range, move toward target (one tile at a time)
                        self.try_move_toward(tx, ty, current_time, other_npc_positions);
                    }
                    // If in range but movement not done, stay in Chasing and wait
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

                        // Attack only if movement has completed (movement cooldown expired)
                        // and attack cooldown is ready
                        let movement_done = current_time - self.last_move_time >= self.get_move_cooldown_ms();
                        let attack_ready = current_time - self.last_attack_time >= self.get_attack_cooldown_ms();

                        if movement_done && attack_ready {
                            self.last_attack_time = current_time;
                            self.just_attacked = true; // Signal client to play animation
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
    /// Entity prototype ID (e.g., "pig", "elder_villager") for client-side lookup
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
    /// Whether this NPC offers quests
    pub is_quest_giver: bool,
    /// Whether this NPC is a merchant
    pub is_merchant: bool,
    /// Movement speed in tiles per second (for client interpolation)
    pub move_speed: f32,
    /// True only on the tick when this NPC attacks (for animation sync)
    pub just_attacked: bool,
}

impl From<&Npc> for NpcUpdate {
    fn from(npc: &Npc) -> Self {
        // Convert move_cooldown_ms to tiles per second
        // e.g., 500ms per tile = 2.0 tiles/sec, 250ms = 4.0 tiles/sec
        let move_speed = if npc.stats.move_cooldown_ms > 0 {
            1000.0 / npc.stats.move_cooldown_ms as f32
        } else {
            0.0 // Non-moving NPCs (villagers, etc.)
        };

        Self {
            id: npc.id.clone(),
            entity_type: npc.stats.sprite.clone(),
            display_name: npc.stats.display_name.clone(),
            x: npc.x,
            y: npc.y,
            direction: npc.direction as u8,
            hp: npc.hp,
            max_hp: npc.max_hp,
            level: npc.level,
            state: npc.state as u8,
            hostile: npc.is_hostile(),
            is_quest_giver: npc.is_quest_giver(),
            is_merchant: npc.is_merchant(),
            move_speed,
            just_attacked: npc.just_attacked,
        }
    }
}
