use serde::{Deserialize, Serialize};
use crate::render::animation::{PlayerAnimation, AnimationState};
use super::skills::Skills;
use super::item_registry::ItemRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Down = 0,
    Left = 1,
    Up = 2,
    Right = 3,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::Down
    }
}

impl Direction {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Direction::Down,
            1 => Direction::Left,
            2 => Direction::Up,
            3 => Direction::Right,
            _ => Direction::Down,
        }
    }
}

impl Direction {
    pub fn from_velocity(dx: f32, dy: f32) -> Self {
        if dx == 0.0 && dy == 0.0 {
            return Direction::Down;
        }

        // 4 quadrants: vertical takes priority when |dy| > |dx|
        if dy.abs() > dx.abs() {
            if dy < 0.0 { Direction::Up } else { Direction::Down }
        } else {
            if dx < 0.0 { Direction::Left } else { Direction::Right }
        }
    }

    pub fn to_unit_vector(&self) -> (f32, f32) {
        match self {
            Direction::Down => (0.0, 1.0),
            Direction::Up => (0.0, -1.0),
            Direction::Left => (-1.0, 0.0),
            Direction::Right => (1.0, 0.0),
        }
    }
}

// Movement speed in tiles per second (must match server: 250ms per tile = 15 frames at 60fps)
pub const TILES_PER_SECOND: f32 = 4.0;

// Linear interpolation speed - must match server movement rate
// Server: 250ms per tile = 4 tiles per second
const VISUAL_SPEED: f32 = 4.0;

#[derive(Debug, Clone)]
pub struct Player {
    pub id: String,
    pub name: String,

    // Rendered position (smoothly interpolated each frame)
    pub x: f32,
    pub y: f32,

    // Server-authoritative position (for local player reconciliation)
    pub server_x: f32,
    pub server_y: f32,

    // Target for other players' interpolation
    pub target_x: f32,
    pub target_y: f32,

    // Movement velocity (-1, 0, or 1 per axis, normalized for diagonal)
    pub vel_x: f32,
    pub vel_y: f32,

    pub direction: Direction,
    pub is_moving: bool,

    // Stats
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub skills: Skills,

    // Death state
    pub is_dead: bool,
    pub death_time: f64, // When the player died (game time)

    // Appearance
    pub gender: String, // "male" or "female"
    pub skin: String,   // "tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"
    pub hair_style: Option<i32>, // 0-2 (or None for bald)
    pub hair_color: Option<i32>, // 0-6 (color variant index)

    // Equipment
    pub equipped_head: Option<String>,   // Item ID of equipped helmet/hat
    pub equipped_body: Option<String>,   // Item ID of equipped body armor
    pub equipped_weapon: Option<String>, // Item ID of equipped weapon (sword/bow/etc)
    pub equipped_back: Option<String>,   // Item ID of equipped back item (cape/quiver/etc)
    pub equipped_feet: Option<String>,   // Item ID of equipped boots
    pub equipped_ring: Option<String>,   // Item ID of equipped ring
    pub equipped_gloves: Option<String>, // Item ID of equipped gloves
    pub equipped_necklace: Option<String>, // Item ID of equipped necklace
    pub equipped_belt: Option<String>,   // Item ID of equipped belt

    // Admin status
    pub is_admin: bool,

    // Animation
    pub animation: PlayerAnimation,

    // Last time this player took damage (for health bar visibility)
    pub last_damage_time: f64,

    // Client-side prediction for local player
    pub predicted_x: f32,
    pub predicted_y: f32,
    pub has_pending_prediction: bool,
}

impl Player {
    pub fn new(id: String, name: String, x: f32, y: f32, gender: String, skin: String) -> Self {
        Self {
            id,
            name,
            x,
            y,
            server_x: x,
            server_y: y,
            target_x: x,
            target_y: y,
            vel_x: 0.0,
            vel_y: 0.0,
            direction: Direction::Down,
            is_moving: false,
            hp: 10,
            max_hp: 10,
            mp: 50,
            max_mp: 50,
            skills: Skills::new(),
            is_dead: false,
            death_time: 0.0,
            gender,
            skin,
            hair_style: None,
            hair_color: None,
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
            animation: PlayerAnimation::new(),
            last_damage_time: 0.0,
            predicted_x: 0.0,
            predicted_y: 0.0,
            has_pending_prediction: false,
        }
    }

    pub fn die(&mut self) {
        self.is_dead = true;
        self.death_time = macroquad::time::get_time();
        self.hp = 0;
    }

    /// Get the combat level (calculated from skills)
    pub fn combat_level(&self) -> i32 {
        self.skills.combat_level()
    }

    /// Get all equipped item IDs as an array for iteration
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

    /// Calculate total attack bonus from all equipped items
    pub fn attack_bonus(&self, item_registry: &ItemRegistry) -> i32 {
        let mut bonus = 0;
        for equipped in self.all_equipped() {
            if let Some(item_id) = equipped {
                let def = item_registry.get_or_placeholder(item_id);
                if let Some(equip) = &def.equipment {
                    bonus += equip.attack_bonus;
                }
            }
        }
        bonus
    }

    /// Calculate total strength bonus from all equipped items
    pub fn strength_bonus(&self, item_registry: &ItemRegistry) -> i32 {
        let mut bonus = 0;
        for equipped in self.all_equipped() {
            if let Some(item_id) = equipped {
                let def = item_registry.get_or_placeholder(item_id);
                if let Some(equip) = &def.equipment {
                    bonus += equip.strength_bonus;
                }
            }
        }
        bonus
    }

    /// Calculate total defence bonus from all equipped items
    pub fn defence_bonus(&self, item_registry: &ItemRegistry) -> i32 {
        let mut bonus = 0;
        for equipped in self.all_equipped() {
            if let Some(item_id) = equipped {
                let def = item_registry.get_or_placeholder(item_id);
                if let Some(equip) = &def.equipment {
                    bonus += equip.defence_bonus;
                }
            }
        }
        bonus
    }

    pub fn respawn(&mut self, x: f32, y: f32, hp: i32) {
        self.is_dead = false;
        self.death_time = 0.0;
        self.hp = hp;
        self.max_hp = hp;
        self.x = x;
        self.y = y;
        self.server_x = x;
        self.server_y = y;
        self.target_x = x;
        self.target_y = y;
    }

    /// Apply local input for client-side prediction (local player only)
    /// Predicts when at tile center with no server velocity
    pub fn apply_local_input(&mut self, dx: f32, dy: f32) {
        if dx == 0.0 && dy == 0.0 {
            return;
        }

        // Predict if at tile center and server says we're not moving
        // This covers: starting from stop, AND direction changes at tile boundaries
        let at_tile_center = (self.x - self.x.round()).abs() < 0.05
                          && (self.y - self.y.round()).abs() < 0.05;
        let server_stopped = self.vel_x == 0.0 && self.vel_y == 0.0;

        if at_tile_center && server_stopped {
            let tile_x = self.x.round();
            let tile_y = self.y.round();
            self.predicted_x = tile_x + dx;
            self.predicted_y = tile_y + dy;
            self.target_x = self.predicted_x;
            self.target_y = self.predicted_y;
            self.has_pending_prediction = true;
            self.direction = Direction::from_velocity(dx, dy);
        }
    }

    /// Set server-authoritative position (called when server sends state sync)
    /// Server sends grid positions (i32), we store as f32 for interpolation
    pub fn set_server_position(&mut self, new_x: f32, new_y: f32) {
        self.set_server_position_with_velocity(new_x, new_y, 0.0, 0.0, false);
    }

    /// Set server position with velocity for client-side prediction
    /// is_local_player: if true, uses prediction-aware reconciliation
    pub fn set_server_position_with_velocity(&mut self, new_x: f32, new_y: f32, vel_x: f32, vel_y: f32, is_local_player: bool) {
        // Update authoritative server position and velocity
        self.server_x = new_x;
        self.server_y = new_y;
        self.vel_x = vel_x;
        self.vel_y = vel_y;

        // Calculate distance from visual to server
        let dx = self.x - new_x;
        let dy = self.y - new_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist > 2.0 {
            // Too far - hard snap (teleport/major desync)
            self.x = new_x;
            self.y = new_y;
            self.target_x = new_x;
            self.target_y = new_y;
            self.has_pending_prediction = false;
        } else if is_local_player && self.has_pending_prediction {
            // Local player with pending prediction (first move from stop)
            let server_moving = vel_x != 0.0 || vel_y != 0.0;

            if server_moving {
                // Server confirmed movement - clear prediction flag, now follow server
                // But keep current target if we're already moving toward it
                self.has_pending_prediction = false;
            } else {
                // Server stopped - check if we reached our prediction
                let at_predicted = (self.x - self.predicted_x).abs() < 0.1
                                && (self.y - self.predicted_y).abs() < 0.1;
                if at_predicted {
                    // Check if server agrees with where we ended up
                    let server_disagrees = (new_x - self.predicted_x).abs() > 0.01
                                        || (new_y - self.predicted_y).abs() > 0.01;
                    if server_disagrees {
                        // Server rejected our move (hit a wall) - snap to server
                        self.target_x = new_x;
                        self.target_y = new_y;
                    }
                    self.has_pending_prediction = false;
                }
            }
        } else {
            // Remote player OR local player following server
            // When server has velocity, predict next tile; otherwise target is server position
            if vel_x != 0.0 || vel_y != 0.0 {
                self.target_x = new_x + vel_x;
                self.target_y = new_y + vel_y;
            } else {
                self.target_x = new_x;
                self.target_y = new_y;
            }
        }

        // Update direction from velocity
        if vel_x != 0.0 || vel_y != 0.0 {
            self.direction = Direction::from_velocity(vel_x, vel_y);
        }
    }

    /// Smooth visual interpolation toward target position
    /// Uses server velocity for continuous movement prediction
    pub fn interpolate_visual(&mut self, delta: f32) {
        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();

        // Track if we're actually moving this frame (not just predicting)
        let actually_moving;

        if dist < 0.01 {
            // Reached target - snap exactly
            self.x = self.target_x;
            self.y = self.target_y;
            actually_moving = false;

            // Check if we're at the server position
            let at_server = (self.x - self.server_x).abs() < 0.01
                         && (self.y - self.server_y).abs() < 0.01;

            if at_server && (self.vel_x != 0.0 || self.vel_y != 0.0) {
                // At server position with velocity - predict next tile
                self.target_x = self.server_x + self.vel_x;
                self.target_y = self.server_y + self.vel_y;
                self.is_moving = true;
            } else {
                // Either no velocity, or waiting for server to catch up
                self.is_moving = false;
            }
        } else {
            // Linear interpolation - constant speed movement
            let move_dist = VISUAL_SPEED * delta;

            if dist <= move_dist {
                // Close enough - snap to target
                self.x = self.target_x;
                self.y = self.target_y;
            } else {
                // Move at constant speed toward target
                self.x += (dx / dist) * move_dist;
                self.y += (dy / dist) * move_dist;
            }

            self.is_moving = true;
            actually_moving = true;

            // Only update direction from movement vector if no server velocity
            // (velocity is more authoritative than visual movement direction)
            let in_action = matches!(
                self.animation.state,
                AnimationState::Attacking | AnimationState::Casting | AnimationState::ShootingBow
            );
            if self.vel_x == 0.0 && self.vel_y == 0.0 && !in_action {
                self.direction = Direction::from_velocity(dx, dy);
            }
        }

        // Check if movement direction matches velocity direction
        // This prevents "moonwalking" - sprite changing direction before visual moves that way
        let movement_matches_velocity = if self.vel_x != 0.0 || self.vel_y != 0.0 {
            let movement_dir = Direction::from_velocity(
                self.target_x - self.x,
                self.target_y - self.y
            );
            let velocity_dir = Direction::from_velocity(self.vel_x, self.vel_y);
            movement_dir == velocity_dir
        } else {
            true // No velocity = trust movement direction
        };

        // Update animation state based on movement and actions
        self.update_animation(delta, actually_moving, movement_matches_velocity);
    }

    /// Update animation state and frame
    /// actually_moving: true only when visual position is actively changing
    /// movement_aligned: true only when movement direction matches velocity direction
    fn update_animation(&mut self, delta: f32, actually_moving: bool, movement_aligned: bool) {
        // Check if player has velocity (intending to move, even if at tile boundary)
        let has_velocity = self.vel_x != 0.0 || self.vel_y != 0.0;

        // Handle action animations (attack, cast, etc) - they take priority
        // Direction is locked in when the action starts (in play_attack/play_cast/play_shoot_bow)
        // Don't sync direction during actions - visual interpolation may still be catching up
        let in_action_animation = self.animation.state == AnimationState::Attacking
            || self.animation.state == AnimationState::Casting
            || self.animation.state == AnimationState::ShootingBow;

        // Sync direction to animation when safe (not moonwalking):
        // 1. When moving: only if movement direction matches velocity (prevents moonwalking)
        // 2. When stationary: always sync (handles Face commands from server)
        // 3. Never during action animations (direction locked when action started)
        let should_sync_direction = if in_action_animation {
            false
        } else if actually_moving {
            movement_aligned // Only sync when movement catches up to velocity direction
        } else if !has_velocity {
            true // Stationary with no velocity = safe to sync (Face commands)
        } else {
            false // At tile boundary with velocity - wait for movement to start
        };

        if should_sync_direction {
            self.animation.direction = self.direction;
        }

        if in_action_animation {
            self.animation.update(delta);
            // Return to idle/walking when action animation completes
            if self.animation.is_finished() {
                if actually_moving || has_velocity {
                    self.animation.set_state(AnimationState::Walking);
                } else {
                    self.animation.set_state(AnimationState::Idle);
                }
            }
            return;
        }

        // Handle movement animations - only animate when actually moving
        if actually_moving {
            self.animation.set_state(AnimationState::Walking);
            self.animation.update(delta);
        } else {
            // Only go to idle if not in a sitting state AND we have no velocity
            // Having velocity means we're at a tile boundary waiting for server confirmation
            // during continuous movement - keep Walking state to prevent jitter
            if self.animation.state != AnimationState::SittingGround
                && self.animation.state != AnimationState::SittingChair
                && !has_velocity
            {
                self.animation.set_state(AnimationState::Idle);
            }
            // Don't update animation frame when idle or waiting at tile boundary
        }
    }

    /// Trigger attack animation
    pub fn play_attack(&mut self) {
        // Sync animation direction to player direction before attacking
        // This ensures attack faces the current intended direction, even if
        // the player just changed direction while standing still
        self.animation.direction = self.direction;
        self.animation.set_state(AnimationState::Attacking);
    }

    /// Trigger spell casting animation
    pub fn play_cast(&mut self) {
        self.animation.direction = self.direction;
        self.animation.set_state(AnimationState::Casting);
    }

    /// Trigger bow shooting animation
    pub fn play_shoot_bow(&mut self) {
        self.animation.direction = self.direction;
        self.animation.set_state(AnimationState::ShootingBow);
    }

    /// Sit on ground
    pub fn sit_ground(&mut self) {
        self.animation.set_state(AnimationState::SittingGround);
    }

    /// Sit on chair
    pub fn sit_chair(&mut self) {
        self.animation.set_state(AnimationState::SittingChair);
    }

    /// Stand up from sitting
    pub fn stand_up(&mut self) {
        self.animation.set_state(AnimationState::Idle);
    }

    /// Smooth interpolation toward target position
    pub fn update(&mut self, delta: f32) {
        self.interpolate_visual(delta);
    }
}
