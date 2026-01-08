use super::entities::Direction;

// ============================================================================
// NPC Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Slime = 0,
}

impl NpcType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => NpcType::Slime,
            _ => NpcType::Slime,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            NpcType::Slime => "Slime",
        }
    }
}

// ============================================================================
// NPC State
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcState {
    Idle = 0,
    Chasing = 1,
    Attacking = 2,
    Returning = 3,
    Dead = 4,
}

impl NpcState {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => NpcState::Idle,
            1 => NpcState::Chasing,
            2 => NpcState::Attacking,
            3 => NpcState::Returning,
            4 => NpcState::Dead,
            _ => NpcState::Idle,
        }
    }
}

// ============================================================================
// NPC Entity
// ============================================================================

#[derive(Debug, Clone)]
pub struct Npc {
    pub id: String,
    pub npc_type: NpcType,
    /// Entity prototype ID (e.g., "slime", "elder_villager")
    pub entity_type: String,
    /// Display name from server
    pub display_name: String,
    pub x: f32,
    pub y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub direction: Direction,
    pub hp: i32,
    pub max_hp: i32,
    pub level: i32,
    pub state: NpcState,
    pub animation_frame: f32,
    /// Whether this NPC is hostile
    pub hostile: bool,
}

impl Npc {
    pub fn new(id: String, npc_type: NpcType, x: f32, y: f32) -> Self {
        Self {
            id,
            npc_type,
            entity_type: "slime".to_string(),
            display_name: npc_type.name().to_string(),
            x,
            y,
            target_x: x,
            target_y: y,
            direction: Direction::Down,
            hp: 50,
            max_hp: 50,
            level: 1,
            state: NpcState::Idle,
            animation_frame: 0.0,
            hostile: true,
        }
    }

    pub fn name(&self) -> String {
        format!("{} Lv.{}", self.display_name, self.level)
    }

    pub fn is_hostile(&self) -> bool {
        self.hostile
    }

    pub fn is_alive(&self) -> bool {
        self.state != NpcState::Dead
    }

    /// Update position from server data
    pub fn set_server_position(&mut self, new_x: f32, new_y: f32) {
        self.target_x = new_x;
        self.target_y = new_y;

        // Calculate direction from movement
        let dx = new_x - self.x;
        let dy = new_y - self.y;
        if dx.abs() > 0.01 || dy.abs() > 0.01 {
            self.direction = Direction::from_velocity(dx, dy);
        }
    }

    /// Smooth interpolation toward grid position
    /// Server moves NPCs at 2 tiles/sec (500ms per tile)
    pub fn update(&mut self, delta: f32) {
        const INTERPOLATION_SPEED: f32 = 2.0; // tiles/sec - match server speed for smooth movement

        if self.state == NpcState::Dead {
            return;
        }

        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();

        let move_dist = INTERPOLATION_SPEED * delta;

        if dist <= move_dist || dist < 0.01 {
            self.x = self.target_x;
            self.y = self.target_y;
        } else {
            self.x += (dx / dist) * move_dist;
            self.y += (dy / dist) * move_dist;

            // Animation while moving
            self.animation_frame += delta * 6.0;
            if self.animation_frame >= 4.0 {
                self.animation_frame = 0.0;
            }
        }
    }
}
