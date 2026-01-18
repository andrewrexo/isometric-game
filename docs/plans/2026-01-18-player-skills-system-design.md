# Player Skills System Design

## Overview

Implement a RuneScape-style skill system for combat. Players have individual skill levels that determine equipment access, combat effectiveness, and overall combat level.

## Core Combat Skills

| Skill | Purpose | Equipment Gate |
|-------|---------|----------------|
| **Hitpoints** | Max HP (1 HP per level) | None |
| **Attack** | Accuracy (hit chance) | Weapons |
| **Strength** | Max hit (damage) | None |
| **Defence** | Avoid hits | Armor |

All skills cap at level 99.

## Starting Stats

- Hitpoints: Level 10 (10 HP)
- Attack: Level 1
- Strength: Level 1
- Defence: Level 1
- Combat Level: 3

## XP Table

Uses RuneScape's XP formula. Total XP required to reach a level:

```rust
fn total_xp_for_level(level: i32) -> i64 {
    let mut total = 0.0;
    for l in 1..level {
        total += (l as f64 + 300.0 * 2.0_f64.powf(l as f64 / 7.0)) / 4.0;
    }
    total.floor() as i64
}
```

| Level | Total XP | Level | Total XP |
|-------|----------|-------|----------|
| 1 | 0 | 50 | 101,333 |
| 10 | 1,154 | 70 | 737,627 |
| 20 | 4,470 | 90 | 5,346,332 |
| 30 | 13,363 | 99 | 13,034,431 |

## Combat Level Formula

```rust
fn combat_level(hp: i32, attack: i32, strength: i32, defence: i32) -> i32 {
    let base = (defence + hp) as f64 / 4.0;
    let melee = (attack + strength) as f64 * 0.325;
    (base + melee).floor() as i32
}
```

- Level 1 Attack/Str/Def + Level 10 HP → Combat 3
- Level 99 all skills → Combat 126

## Hit/Miss Mechanics

### Attack Roll vs Defence Roll

```rust
fn calculate_hit(
    attack_level: i32,
    attack_bonus: i32,
    defence_level: i32,
    defence_bonus: i32,
) -> bool {
    let attack_roll = rand(0, attack_level * (attack_bonus + 64));
    let defence_roll = rand(0, defence_level * (defence_bonus + 64));
    attack_roll > defence_roll
}
```

Approximate hit chances (equal bonuses):
- Equal levels: ~50%
- 2x attack advantage: ~67%
- Maxed vs level 1: ~99%

### Damage Calculation

```rust
fn calculate_max_hit(strength_level: i32, strength_bonus: i32) -> i32 {
    let base = 1.3 + (strength_level as f64 / 10.0);
    let bonus = (strength_level * strength_bonus) as f64 / 640.0;
    (base + bonus).floor() as i32
}

fn roll_damage(max_hit: i32) -> i32 {
    rand(0, max_hit)  // Inclusive, can hit 0 (low hit) or max
}
```

Example max hits:
| Str Level | No Bonus | +10 Bonus | +50 Bonus |
|-----------|----------|-----------|-----------|
| 1 | 1 | 1 | 1 |
| 10 | 2 | 2 | 2 |
| 50 | 6 | 6 | 9 |
| 99 | 11 | 12 | 18 |

## Equipment Requirements

Equipment requires ONE skill level (never both):
- **Weapons** → Attack level
- **Armor** → Defence level

```rust
pub struct EquipmentStats {
    pub slot: EquipmentSlot,
    pub attack_level_required: i32,   // 0 if no requirement
    pub defence_level_required: i32,  // 0 if no requirement
    pub attack_bonus: i32,            // Accuracy
    pub strength_bonus: i32,          // Max hit
    pub defence_bonus: i32,           // Avoid hits
}
```

### Equipment Tiers

| Tier | Level Required | Example |
|------|----------------|---------|
| Bronze | 1 | Bronze sword, bronze armor |
| Iron | 10 | Iron sword, iron armor |
| Steel | 20 | Steel sword, steel armor |
| Mithril | 30 | Mithril sword, mithril armor |
| Adamant | 40 | Adamant sword, adamant armor |
| Rune | 50 | Rune sword, rune armor |
| Dragon | 60 | Dragon sword, dragon armor |

## XP Distribution

XP is awarded based on damage dealt:
- 4 XP per 1 damage to the combat skill (based on style)
- 1.33 XP per 1 damage to Hitpoints (always)

### Combat Styles (Future)

| Style | Combat XP | Invisible Bonus |
|-------|-----------|-----------------|
| Accurate | Attack | +3 Attack |
| Aggressive | Strength | +3 Strength |
| Defensive | Defence | +3 Defence |
| Controlled | Split evenly | None |

**For initial implementation:** Use "Controlled" style only (XP split evenly between Attack, Strength, Defence).

## Data Structures

### Server (Rust)

```rust
/// Individual skill data
#[derive(Clone, Serialize, Deserialize)]
pub struct Skill {
    pub level: i32,
    pub xp: i64,
}

impl Skill {
    pub fn new(level: i32) -> Self {
        Self {
            level,
            xp: total_xp_for_level(level),
        }
    }
}

/// All player skills
#[derive(Clone, Serialize, Deserialize)]
pub struct Skills {
    pub hitpoints: Skill,
    pub attack: Skill,
    pub strength: Skill,
    pub defence: Skill,
}

impl Skills {
    pub fn new() -> Self {
        Self {
            hitpoints: Skill::new(10),
            attack: Skill::new(1),
            strength: Skill::new(1),
            defence: Skill::new(1),
        }
    }

    pub fn combat_level(&self) -> i32 {
        let base = (self.defence.level + self.hitpoints.level) as f64 / 4.0;
        let melee = (self.attack.level + self.strength.level) as f64 * 0.325;
        (base + melee).floor() as i32
    }
}
```

### Protocol Messages

```rust
// Sent on skill XP gain
ServerMessage::SkillXp {
    player_id: String,
    skill: SkillType,      // "hitpoints", "attack", "strength", "defence"
    xp_gained: i32,
    total_xp: i64,
    level: i32,
}

// Sent on level up
ServerMessage::SkillLevelUp {
    player_id: String,
    skill: SkillType,
    new_level: i32,
}

// Skill type enum for extensibility
enum SkillType {
    Hitpoints,
    Attack,
    Strength,
    Defence,
    // Future: Ranged, Magic, Prayer, etc.
}
```

### Client State

```rust
pub struct PlayerSkills {
    pub hitpoints: SkillData,
    pub attack: SkillData,
    pub strength: SkillData,
    pub defence: SkillData,
}

pub struct SkillData {
    pub level: i32,
    pub xp: i64,
    pub xp_to_next_level: i64,
}
```

## Combat Flow

1. **Attacker initiates attack**
2. **Server calculates hit chance:**
   - Gather attacker's attack level + attack bonus from weapon
   - Gather defender's defence level + defence bonus from armor
   - Roll attack vs defence
3. **If hit:**
   - Calculate max hit from strength level + strength bonus
   - Roll damage 0 to max hit
   - Apply damage to defender
   - Award XP to attacker (split between Attack/Str/Def + HP)
4. **If miss:**
   - Show "0" or miss indicator
   - No XP awarded
5. **Broadcast result** to all players

## Database Schema Changes

Replace single level/exp columns with skills JSON:

```sql
-- Remove old columns
ALTER TABLE players DROP COLUMN level;
ALTER TABLE players DROP COLUMN exp;
ALTER TABLE players DROP COLUMN exp_to_next_level;
ALTER TABLE players DROP COLUMN max_hp;

-- Add skills column (JSON)
ALTER TABLE players ADD COLUMN skills TEXT NOT NULL DEFAULT '{}';
```

Skills stored as JSON:
```json
{
  "hitpoints": {"level": 10, "xp": 1154},
  "attack": {"level": 1, "xp": 0},
  "strength": {"level": 1, "xp": 0},
  "defence": {"level": 1, "xp": 0}
}
```

## UI Changes

### Character Panel

Display all skills with levels and XP progress bars:
```
┌─────────────────────────────┐
│ Combat Level: 3             │
├─────────────────────────────┤
│ Hitpoints  10  ████████░░  │
│ Attack      1  ░░░░░░░░░░  │
│ Strength    1  ░░░░░░░░░░  │
│ Defence     1  ░░░░░░░░░░  │
└─────────────────────────────┘
```

### Player Overhead

Show combat level instead of generic level:
```
PlayerName
Combat: 3
```

### XP Drops

Show skill-specific XP drops on hit:
```
+12 Attack
+12 Strength
+12 Defence
+12 Hitpoints
```

## Files to Modify

### Server (rust-server/)
1. `src/skills.rs` (new) - Skill struct, XP table, combat level calc
2. `src/game.rs` - Replace level/exp with Skills, update combat
3. `src/protocol.rs` - Add SkillXp, SkillLevelUp messages
4. `src/db.rs` - Update save/load for skills JSON
5. `data/items/equipment.toml` - Add attack/defence requirements

### Client (client/)
1. `src/game/skills.rs` (new) - Mirror skill data structures
2. `src/game/state.rs` - Add PlayerSkills to player state
3. `src/network/protocol.rs` - Handle new messages
4. `src/render/ui/` - Character panel, XP drops
5. `src/render/renderer.rs` - Combat level display

## Future Extensibility

The SkillType enum and Skills struct are designed for expansion:

```rust
pub struct Skills {
    // Combat
    pub hitpoints: Skill,
    pub attack: Skill,
    pub strength: Skill,
    pub defence: Skill,
    // Future combat
    pub ranged: Skill,
    pub magic: Skill,
    pub prayer: Skill,
    // Future gathering
    pub mining: Skill,
    pub fishing: Skill,
    pub woodcutting: Skill,
    // Future artisan
    pub smithing: Skill,
    pub cooking: Skill,
    pub crafting: Skill,
}
```

Combat level formula would expand to include ranged/magic:
```rust
let melee = (attack + strength) * 0.325;
let ranged = ranged_level * 0.4875;
let magic = magic_level * 0.4875;
let combat_style = melee.max(ranged).max(magic);
combat_level = base + combat_style
```
