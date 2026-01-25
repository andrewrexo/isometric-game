//! XP globe notifications - circular progress indicators for skill XP gains

use macroquad::prelude::*;
use crate::game::SkillType;

// ============================================================================
// Constants
// ============================================================================

pub const GLOBE_SIZE: f32 = 40.0;
pub const GLOBE_SPACING: f32 = 4.0;
const ICON_SIZE: f32 = 24.0;
const RING_THICKNESS: f32 = 3.0;
const VISIBLE_DURATION: f64 = 3.0;  // Seconds before fade starts
const FADE_OUT_DURATION: f64 = 0.5; // Seconds to fully fade

// UI icons sprite sheet: 24x24 icons in 10 columns
const UI_ICON_SIZE: f32 = 24.0;

// ============================================================================
// XP Globe
// ============================================================================

/// A single XP globe notification
pub struct XpGlobe {
    pub skill_type: SkillType,
    pub current_xp: i64,
    pub xp_for_next_level: i64,
    pub level: i32,
    pub last_updated: f64,
}

impl XpGlobe {
    pub fn new(skill_type: SkillType, current_xp: i64, xp_for_next_level: i64, level: i32) -> Self {
        Self {
            skill_type,
            current_xp,
            xp_for_next_level,
            level,
            last_updated: macroquad::time::get_time(),
        }
    }

    /// Calculate progress toward next level (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.xp_for_next_level <= 0 {
            return 1.0;
        }
        let current_level_xp = crate::game::skills::total_xp_for_level(self.level);
        let xp_in_level = self.current_xp - current_level_xp;
        let xp_needed = self.xp_for_next_level - current_level_xp;
        if xp_needed <= 0 {
            return 1.0;
        }
        (xp_in_level as f32 / xp_needed as f32).clamp(0.0, 1.0)
    }

    /// Get opacity based on time since last update
    pub fn opacity(&self, current_time: f64) -> f32 {
        let age = current_time - self.last_updated;
        if age < VISIBLE_DURATION {
            1.0
        } else {
            let fade_progress = (age - VISIBLE_DURATION) / FADE_OUT_DURATION;
            (1.0 - fade_progress as f32).clamp(0.0, 1.0)
        }
    }

    /// Check if globe should be removed
    pub fn is_expired(&self, current_time: f64) -> bool {
        current_time - self.last_updated > VISIBLE_DURATION + FADE_OUT_DURATION
    }
}

// ============================================================================
// XP Globes Manager
// ============================================================================

/// Manages active XP globe notifications
#[derive(Default)]
pub struct XpGlobesManager {
    pub globes: Vec<XpGlobe>,
}

impl XpGlobesManager {
    pub fn new() -> Self {
        Self { globes: Vec::new() }
    }

    /// Handle an XP gain event
    pub fn on_xp_gain(&mut self, skill_type: SkillType, current_xp: i64, xp_for_next_level: i64, level: i32) {
        // Check if globe for this skill already exists
        if let Some(globe) = self.globes.iter_mut().find(|g| g.skill_type == skill_type) {
            // Update existing globe
            globe.current_xp = current_xp;
            globe.xp_for_next_level = xp_for_next_level;
            globe.level = level;
            globe.last_updated = macroquad::time::get_time();
        } else {
            // Create new globe (insert at beginning so it appears on the left)
            self.globes.insert(0, XpGlobe::new(skill_type, current_xp, xp_for_next_level, level));
        }
    }

    /// Update globes, removing expired ones
    pub fn update(&mut self) {
        let current_time = macroquad::time::get_time();
        self.globes.retain(|globe| !globe.is_expired(current_time));
    }
}
