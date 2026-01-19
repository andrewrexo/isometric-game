use serde::{Deserialize, Serialize};
use crate::render::animation::{PlayerAnimation, AnimationState};
use super::skills::Skills;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
            4 => Direction::DownLeft,
            5 => Direction::DownRight,
            6 => Direction::UpLeft,
            7 => Direction::UpRight,
            _ => Direction::Down,
        }
    }
}

impl Direction {
    pub fn from_velocity(dx: f32, dy: f32) -> Self {
        if dx == 0.0 && dy == 0.0 {
            return Direction::Down;
        }

        let angle = dy.atan2(dx);
        let octant = ((angle + std::f32::consts::PI) / (std::f32::consts::PI / 4.0)) as i32 % 8;

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

    pub fn to_unit_vector(&self) -> (f32, f32) {
        match self {
            Direction::Down => (0.0, 1.0),
            Direction::Up => (0.0, -1.0),
            Direction::Left => (-1.0, 0.0),
            Direction::Right => (1.0, 0.0),
            Direction::DownLeft => (-0.707, 0.707),
            Direction::DownRight => (0.707, 0.707),
            Direction::UpLeft => (-0.707, -0.707),
            Direction::UpRight => (0.707, -0.707),
        }
    }
}

// Movement speed in tiles per second (must match server: 250ms per tile = 15 frames at 60fps)
pub const TILES_PER_SECOND: f32 = 4.0;

// Visual interpolation speed - match server speed for smooth tile-to-tile movement
// 1000ms / 250ms = 4.0 tiles/sec, exactly 15 frames per tile at 60fps
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

    /// Set server-authoritative position (called when server sends state sync)
    /// Server sends grid positions (i32), we store as f32 for interpolation
    pub fn set_server_position(&mut self, new_x: f32, new_y: f32) {
        self.set_server_position_with_velocity(new_x, new_y, 0.0, 0.0);
    }

    /// Set server position with velocity for client-side prediction
    /// Simple approach: always trust server, predict only when caught up
    pub fn set_server_position_with_velocity(&mut self, new_x: f32, new_y: f32, vel_x: f32, vel_y: f32) {
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
            // Too far - hard snap
            self.x = new_x;
            self.y = new_y;
            self.target_x = new_x;
            self.target_y = new_y;
        } else {
            // Always update target to server position
            // Interpolation will smoothly move us there
            self.target_x = new_x;
            self.target_y = new_y;
        }

        // Update direction from velocity
        if vel_x != 0.0 || vel_y != 0.0 {
            self.direction = Direction::from_velocity(vel_x, vel_y);
        }
    }

    /// Smooth visual interpolation toward target position
    /// Simple prediction: when at server position with velocity, predict next tile
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
            // Move toward target at constant speed
            let move_dist = VISUAL_SPEED * delta;

            if dist <= move_dist {
                self.x = self.target_x;
                self.y = self.target_y;
            } else {
                self.x += (dx / dist) * move_dist;
                self.y += (dy / dist) * move_dist;
            }

            self.is_moving = true;
            actually_moving = true;

            // Update direction from actual movement vector, not stored velocity
            self.direction = Direction::from_velocity(dx, dy);
        }

        // Update animation state based on movement and actions
        self.update_animation(delta, actually_moving);
    }

    /// Update animation state and frame
    /// actually_moving: true only when visual position is actively changing
    fn update_animation(&mut self, delta: f32, actually_moving: bool) {
        // Only sync direction to animation when actually moving
        if actually_moving {
            self.animation.direction = self.direction;
        }

        // Handle action animations (attack, cast, etc) - they take priority
        if self.animation.state == AnimationState::Attacking
            || self.animation.state == AnimationState::Casting
            || self.animation.state == AnimationState::ShootingBow
        {
            self.animation.update(delta);
            // Return to idle/walking when action animation completes
            if self.animation.is_finished() {
                if actually_moving {
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
            // Only go to idle if not in a sitting state
            if self.animation.state != AnimationState::SittingGround
                && self.animation.state != AnimationState::SittingChair
            {
                self.animation.set_state(AnimationState::Idle);
            }
            // Don't update animation frame when idle (or update slowly)
        }
    }

    /// Trigger attack animation
    pub fn play_attack(&mut self) {
        self.animation.set_state(AnimationState::Attacking);
    }

    /// Trigger spell casting animation
    pub fn play_cast(&mut self) {
        self.animation.set_state(AnimationState::Casting);
    }

    /// Trigger bow shooting animation
    pub fn play_shoot_bow(&mut self) {
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

    /// Smooth interpolation toward target position (for non-local players)
    pub fn update(&mut self, delta: f32) {
        self.interpolate_visual(delta);
    }
}
