use super::entities::Direction;
use crate::render::animation::{NpcAnimation, NpcAnimationState};

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
    /// Entity prototype ID (e.g., "pig", "elder_villager") - determines sprite
    pub entity_type: String,
    /// Display name from server (e.g., "Piggy Lv.1")
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
    pub animation: NpcAnimation,
    /// Whether this NPC is hostile
    pub hostile: bool,
    /// Track if we've played the attack animation for current attack cycle
    attack_anim_played: bool,
    /// Timer to track when to allow the next attack animation (seconds)
    attack_cooldown_timer: f32,
}

impl Npc {
    pub fn new(id: String, entity_type: String, x: f32, y: f32) -> Self {
        Self {
            id,
            entity_type,
            display_name: String::new(), // Set by server
            x,
            y,
            target_x: x,
            target_y: y,
            direction: Direction::Down,
            hp: 1,
            max_hp: 1,
            level: 1,
            state: NpcState::Idle,
            animation: NpcAnimation::default(),
            hostile: true,
            attack_anim_played: false,
            attack_cooldown_timer: 0.0,
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
        // Direction is updated during interpolation, not here
        // This prevents snappy direction changes when server sends new targets
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
        let is_moving = dist > 0.01;

        if dist <= move_dist || dist < 0.01 {
            self.x = self.target_x;
            self.y = self.target_y;
        } else {
            self.x += (dx / dist) * move_dist;
            self.y += (dy / dist) * move_dist;

            // Update direction from actual movement vector (anti-moonwalk)
            self.direction = Direction::from_velocity(dx, dy);
        }

        // Update attack cooldown timer
        if self.attack_cooldown_timer > 0.0 {
            self.attack_cooldown_timer -= delta;
            if self.attack_cooldown_timer <= 0.0 {
                // Cooldown expired, ready for next attack animation
                self.attack_anim_played = false;
                self.attack_cooldown_timer = 0.0;
            }
        }

        // Update animation state based on NPC state
        // For attacking: play attack animation once, then show idle until cooldown expires
        let anim_state = match self.state {
            NpcState::Attacking => {
                if self.attack_anim_played {
                    // Already played attack animation, show idle until cooldown expires
                    NpcAnimationState::Idle
                } else {
                    NpcAnimationState::Attacking
                }
            }
            NpcState::Chasing if is_moving => NpcAnimationState::Walking,
            NpcState::Returning if is_moving => NpcAnimationState::Walking,
            _ => NpcAnimationState::Idle,
        };
        self.animation.set_state(anim_state);
        self.animation.update(delta);

        // Mark attack animation as played when it finishes, start cooldown timer
        if self.state == NpcState::Attacking && self.animation.is_finished() && !self.attack_anim_played {
            self.attack_anim_played = true;
            // Server attack cooldown is 2 seconds, so wait ~1.5s before allowing next animation
            self.attack_cooldown_timer = 1.5;
        }
        // Reset flag immediately when not attacking
        if self.state != NpcState::Attacking {
            self.attack_anim_played = false;
            self.attack_cooldown_timer = 0.0;
        }
    }
}
