use serde::{Deserialize, Serialize};

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

// Movement speed in tiles per second (must match server: 250ms per tile = 4 tiles/sec)
pub const TILES_PER_SECOND: f32 = 4.0;

// Visual interpolation speed (slightly faster than actual movement for responsiveness)
const VISUAL_SPEED: f32 = 8.0;

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
    pub level: i32,
    pub exp: i32,
    pub exp_to_next_level: i32,

    // Death state
    pub is_dead: bool,
    pub death_time: f64, // When the player died (game time)

    // Animation
    pub animation_frame: f32,
}

impl Player {
    pub fn new(id: String, name: String, x: f32, y: f32) -> Self {
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
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            level: 1,
            exp: 0,
            exp_to_next_level: 100,
            is_dead: false,
            death_time: 0.0,
            animation_frame: 0.0,
        }
    }

    pub fn die(&mut self) {
        self.is_dead = true;
        self.death_time = macroquad::time::get_time();
        self.hp = 0;
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
        // Store server position as interpolation target
        self.server_x = new_x;
        self.server_y = new_y;
        self.target_x = new_x;
        self.target_y = new_y;

        // Calculate direction from movement delta
        let dx = new_x - self.x;
        let dy = new_y - self.y;

        if dx.abs() > 0.1 || dy.abs() > 0.1 {
            self.direction = Direction::from_velocity(dx, dy);
            self.is_moving = true;
        }
    }

    /// Smooth visual interpolation toward server grid position
    /// Call every frame to update visual position
    pub fn interpolate_visual(&mut self, delta: f32) {
        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 0.01 {
            // At target - snap exactly and stop moving
            self.x = self.target_x;
            self.y = self.target_y;
            self.is_moving = false;
            self.animation_frame = 0.0;
        } else {
            // Smoothly move toward target
            let move_dist = VISUAL_SPEED * delta;

            if dist <= move_dist {
                self.x = self.target_x;
                self.y = self.target_y;
            } else {
                self.x += (dx / dist) * move_dist;
                self.y += (dy / dist) * move_dist;
            }

            self.is_moving = true;

            // Animation while moving
            self.animation_frame += delta * 8.0;
            if self.animation_frame >= 4.0 {
                self.animation_frame = 0.0;
            }
        }
    }

    /// Smooth interpolation toward target position (for non-local players)
    pub fn update(&mut self, delta: f32) {
        self.interpolate_visual(delta);
    }
}
