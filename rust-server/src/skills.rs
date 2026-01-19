//! Combat skills system following RuneScape-style mechanics.
//!
//! Skills: Hitpoints, Combat
//! - Hitpoints: Max HP (1 HP per level, starts at 10)
//! - Combat: Combined attack/strength/defence skill for all combat

use rand::Rng;
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
}

/// Calculate total XP required to reach a level using RuneScape formula.
/// Level 1 = 0 XP, Level 2 = 83 XP, Level 99 = 13,034,431 XP
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

/// Calculate level from total XP (inverse of total_xp_for_level)
pub fn level_for_xp(xp: i64) -> i32 {
    // Binary search for efficiency
    let mut low = 1;
    let mut high = MAX_LEVEL;

    while low < high {
        let mid = (low + high + 1) / 2;
        if total_xp_for_level(mid) <= xp {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

/// Individual skill data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub level: i32,
    pub xp: i64,
}

impl Skill {
    /// Create a new skill at the given level with appropriate XP
    pub fn new(level: i32) -> Self {
        Self {
            level: level.clamp(1, MAX_LEVEL),
            xp: total_xp_for_level(level),
        }
    }

    /// Add XP to this skill, returning true if leveled up
    pub fn add_xp(&mut self, amount: i64) -> bool {
        if self.level >= MAX_LEVEL {
            return false;
        }

        self.xp += amount;
        let new_level = level_for_xp(self.xp).min(MAX_LEVEL);

        if new_level > self.level {
            self.level = new_level;
            true
        } else {
            false
        }
    }

    /// XP needed to reach next level
    pub fn xp_to_next_level(&self) -> i64 {
        if self.level >= MAX_LEVEL {
            return 0;
        }
        total_xp_for_level(self.level + 1) - self.xp
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

    /// Total level (sum of all skill levels)
    pub fn total_level(&self) -> i32 {
        self.hitpoints.level + self.combat.level
    }
}

/// Legacy skills format for database migration
#[derive(Debug, Clone, Deserialize)]
pub struct LegacySkills {
    pub hitpoints: Skill,
    pub attack: Skill,
    pub strength: Skill,
    pub defence: Skill,
}

impl LegacySkills {
    /// Convert legacy skills to new format by summing combat XP
    pub fn to_skills(self) -> Skills {
        let total_combat_xp = self.attack.xp + self.strength.xp + self.defence.xp;
        let combat_level = level_for_xp(total_combat_xp);
        Skills {
            hitpoints: self.hitpoints,
            combat: Skill {
                level: combat_level,
                xp: total_combat_xp,
            },
        }
    }
}

/// Calculate whether an attack hits using attack roll vs defence roll.
/// Returns true if the attack hits.
///
/// Formula: Roll attacker's combat (0 to combat_level * (attack_bonus + 64))
///          Roll defender's combat (0 to combat_level * (defence_bonus + 64))
///          Hit if attack_roll > defence_roll
pub fn calculate_hit(
    attacker_combat_level: i32,
    attack_bonus: i32,
    defender_combat_level: i32,
    defence_bonus: i32,
) -> bool {
    let mut rng = rand::thread_rng();

    let attack_max = attacker_combat_level * (attack_bonus + 64);
    let defence_max = defender_combat_level * (defence_bonus + 64);

    let attack_roll = rng.gen_range(0..=attack_max.max(1));
    let defence_roll = rng.gen_range(0..=defence_max.max(1));

    attack_roll > defence_roll
}

/// Calculate maximum hit based on combat level and equipment bonus.
///
/// Formula: 1.3 + (combat_level / 10) + (combat_level * strength_bonus / 640)
/// This gives roughly:
/// - Level 1, no bonus: 1
/// - Level 50, no bonus: 6
/// - Level 99, no bonus: 11
/// - Level 99, +50 bonus: 18
pub fn calculate_max_hit(combat_level: i32, strength_bonus: i32) -> i32 {
    let base = 1.3 + (combat_level as f64 / 10.0);
    let bonus = (combat_level * strength_bonus) as f64 / 640.0;
    (base + bonus).floor() as i32
}

/// Roll damage between 0 and max_hit (inclusive).
/// In RS, you can hit 0 even on a successful hit roll (a "low hit").
pub fn roll_damage(max_hit: i32) -> i32 {
    if max_hit <= 0 {
        return 0;
    }
    rand::thread_rng().gen_range(0..=max_hit)
}

/// XP awarded per damage dealt.
/// Combat skill XP = damage * 4
/// Hitpoints XP = damage * 1.33 (1/3 of combat XP)
pub const COMBAT_XP_PER_DAMAGE: f64 = 4.0;
pub const HITPOINTS_XP_PER_DAMAGE: f64 = 1.33;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xp_table() {
        // Known RS XP values
        assert_eq!(total_xp_for_level(1), 0);
        assert_eq!(total_xp_for_level(2), 83);
        assert!(total_xp_for_level(10) > 1000);
        assert!(total_xp_for_level(99) > 13_000_000);
    }

    #[test]
    fn test_level_for_xp() {
        assert_eq!(level_for_xp(0), 1);
        assert_eq!(level_for_xp(82), 1);
        assert_eq!(level_for_xp(83), 2);
        assert_eq!(level_for_xp(100), 2);

        // Round-trip test
        for level in 1..=99 {
            let xp = total_xp_for_level(level);
            assert_eq!(level_for_xp(xp), level);
        }
    }

    #[test]
    fn test_skill_add_xp() {
        let mut skill = Skill::new(1);
        assert_eq!(skill.level, 1);
        assert_eq!(skill.xp, 0);

        // Add enough XP to level up
        let leveled = skill.add_xp(100);
        assert!(leveled);
        assert_eq!(skill.level, 2);
    }

    #[test]
    fn test_combat_level() {
        let skills = Skills::new();
        // HP 10, Combat 3
        // combat_level = floor((3 + 10) / 2) = 6
        assert_eq!(skills.combat_level(), 6);

        // Max stats
        let max_skills = Skills {
            hitpoints: Skill::new(99),
            combat: Skill::new(99),
        };
        // combat_level = floor((99 + 99) / 2) = 99
        assert_eq!(max_skills.combat_level(), 99);
    }

    #[test]
    fn test_total_level() {
        let skills = Skills::new();
        // HP 10 + Combat 3 = 13
        assert_eq!(skills.total_level(), 13);
    }

    #[test]
    fn test_max_hit() {
        // Level 1, no bonus
        assert_eq!(calculate_max_hit(1, 0), 1);

        // Level 50, no bonus
        assert_eq!(calculate_max_hit(50, 0), 6);

        // Level 99, no bonus
        assert_eq!(calculate_max_hit(99, 0), 11);

        // Level 99, +50 bonus
        assert!(calculate_max_hit(99, 50) > 15);
    }

    #[test]
    fn test_legacy_migration() {
        let legacy = LegacySkills {
            hitpoints: Skill::new(50),
            attack: Skill::new(40),
            strength: Skill::new(35),
            defence: Skill::new(45),
        };

        let skills = legacy.to_skills();

        // Hitpoints preserved
        assert_eq!(skills.hitpoints.level, 50);

        // Combat XP is sum of attack + strength + defence XP
        let expected_xp = total_xp_for_level(40) + total_xp_for_level(35) + total_xp_for_level(45);
        assert_eq!(skills.combat.xp, expected_xp);
    }
}
