//! Combat skills system for the client.
//!
//! This is a simplified version that tracks skill data received from the server.
//! All combat calculations happen server-side.
//!
//! Skills: Hitpoints, Combat
//! - Hitpoints: Max HP (1 HP per level, starts at 10)
//! - Combat: Combined attack/strength/defence skill for all combat

use serde::{Deserialize, Serialize};

/// Maximum skill level
pub const MAX_LEVEL: i32 = 99;

/// Skill types for combat
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillType {
    Hitpoints,
    Combat,
}

impl SkillType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillType::Hitpoints => "hitpoints",
            SkillType::Combat => "combat",
        }
    }

    pub fn from_str(s: &str) -> Option<SkillType> {
        match s.to_lowercase().as_str() {
            "hitpoints" => Some(SkillType::Hitpoints),
            "combat" => Some(SkillType::Combat),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SkillType::Hitpoints => "Hitpoints",
            SkillType::Combat => "Combat",
        }
    }
}

/// Calculate total XP required to reach a level using RuneScape formula.
/// Used for calculating XP progress within a level.
pub fn total_xp_for_level(level: i32) -> i64 {
    if level <= 1 {
        return 0;
    }
    let mut total = 0.0;
    for l in 1..level {
        total += (l as f64 + 300.0 * 2.0_f64.powf(l as f64 / 7.0)) / 4.0;
    }
    total.floor() as i64
}

/// Individual skill data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub level: i32,
    pub xp: i64,
}

impl Skill {
    pub fn new(level: i32) -> Self {
        Self {
            level: level.clamp(1, MAX_LEVEL),
            xp: total_xp_for_level(level),
        }
    }

    /// XP progress within current level (0.0 to 1.0)
    pub fn level_progress(&self) -> f32 {
        if self.level >= MAX_LEVEL {
            return 1.0;
        }
        let current_level_xp = total_xp_for_level(self.level);
        let next_level_xp = total_xp_for_level(self.level + 1);
        let xp_in_level = self.xp - current_level_xp;
        let xp_needed = next_level_xp - current_level_xp;
        (xp_in_level as f32 / xp_needed as f32).clamp(0.0, 1.0)
    }

    /// XP needed to reach next level
    pub fn xp_to_next_level(&self) -> i64 {
        if self.level >= MAX_LEVEL {
            return 0;
        }
        total_xp_for_level(self.level + 1) - self.xp
    }
}

impl Default for Skill {
    fn default() -> Self {
        Self::new(1)
    }
}

/// All combat skills for a player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills {
    pub hitpoints: Skill,
    pub combat: Skill,
}

impl Default for Skills {
    fn default() -> Self {
        Self::new()
    }
}

impl Skills {
    /// Create new skills with starting values (HP 10, Combat 3)
    pub fn new() -> Self {
        Self {
            hitpoints: Skill::new(10),
            combat: Skill::new(3),
        }
    }

    /// Calculate combat level: (Combat + Hitpoints) / 2
    /// Range: 5 (Combat 1 + HP 10) to 99 (both 99)
    pub fn combat_level(&self) -> i32 {
        ((self.combat.level + self.hitpoints.level) as f64 / 2.0).floor() as i32
    }

    /// Get a skill by type
    pub fn get(&self, skill_type: SkillType) -> &Skill {
        match skill_type {
            SkillType::Hitpoints => &self.hitpoints,
            SkillType::Combat => &self.combat,
        }
    }

    /// Get a mutable skill by type
    pub fn get_mut(&mut self, skill_type: SkillType) -> &mut Skill {
        match skill_type {
            SkillType::Hitpoints => &mut self.hitpoints,
            SkillType::Combat => &mut self.combat,
        }
    }

    /// Update a skill from server data
    pub fn update_skill(&mut self, skill_type: SkillType, xp: i64, level: i32) {
        let skill = self.get_mut(skill_type);
        skill.xp = xp;
        skill.level = level;
    }

    /// Total level (sum of all skill levels)
    pub fn total_level(&self) -> i32 {
        self.hitpoints.level + self.combat.level
    }
}
