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
    /// Visual position (smoothly interpolated)
    pub x: f32,
    pub y: f32,
    /// Server-authoritative position
    pub server_x: f32,
    pub server_y: f32,
    /// Target for interpolation
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
    /// Whether this NPC offers quests
    pub is_quest_giver: bool,
    /// Whether this NPC is a merchant
    pub is_merchant: bool,
    /// Movement speed in tiles per second (from server, for interpolation)
    pub move_speed: f32,
    /// Last time this NPC took damage (for health bar visibility)
    pub last_damage_time: f64,
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
            server_x: x,
            server_y: y,
            direction: Direction::Down,
            hp: 1,
            max_hp: 1,
            level: 1,
            state: NpcState::Idle,
            animation: NpcAnimation::default(),
            hostile: true,
            is_quest_giver: false,
            is_merchant: false,
            move_speed: 2.0, // Default, will be set by server
            last_damage_time: 0.0,
        }
    }

    pub fn name(&self) -> String {
        // Don't show level for friendly NPCs (quest givers and merchants)
        if self.is_quest_giver || self.is_merchant {
            self.display_name.clone()
        } else {
            format!("{} Lv.{}", self.display_name, self.level)
        }
    }

    pub fn is_hostile(&self) -> bool {
        self.hostile
    }

    pub fn is_alive(&self) -> bool {
        self.state != NpcState::Dead
    }

    /// Trigger attack animation - called when damage event is received
    pub fn trigger_attack_animation(&mut self) {
        self.animation.set_state(NpcAnimationState::Attacking);
    }

    /// Update position from server - matches player logic exactly
    pub fn set_server_position(&mut self, new_x: f32, new_y: f32) {
        self.server_x = new_x;
        self.server_y = new_y;

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
            // Update target to server position
            self.target_x = new_x;
            self.target_y = new_y;
        }
    }

    /// Smooth visual interpolation - matches player logic exactly
    /// Uses direction for prediction when NPC is moving
    pub fn update(&mut self, delta: f32) {
        if self.state == NpcState::Dead {
            return;
        }

        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();

        let actually_moving;

        if dist < 0.01 {
            // Reached target - snap exactly
            self.x = self.target_x;
            self.y = self.target_y;
            actually_moving = false;
        } else {
            // Move toward target faster than server speed to always catch up
            // 1.25x ensures we arrive before next server update even with network jitter
            let speed = (self.move_speed * 1.25).max(2.0);
            let move_dist = speed * delta;

            if dist <= move_dist {
                self.x = self.target_x;
                self.y = self.target_y;
            } else {
                self.x += (dx / dist) * move_dist;
                self.y += (dy / dist) * move_dist;
            }

            actually_moving = true;
        }

        // Handle animation states
        // Attack animation is triggered by trigger_attack_animation() when damage event received
        if self.animation.state == NpcAnimationState::Attacking {
            // Let attack animation play through
            if self.animation.is_finished() {
                // Animation done, return to normal
                if actually_moving {
                    self.animation.set_state(NpcAnimationState::Walking);
                } else {
                    self.animation.set_state(NpcAnimationState::Idle);
                }
            }
        } else {
            // Normal movement animation (not in attack animation)
            if actually_moving {
                self.animation.set_state(NpcAnimationState::Walking);
            } else {
                self.animation.set_state(NpcAnimationState::Idle);
            }
        }

        self.animation.update(delta);
    }
}
