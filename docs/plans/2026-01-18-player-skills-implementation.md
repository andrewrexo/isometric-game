# Player Skills System - Implementation Plan

## Overview

Implement RuneScape-style combat skills: Hitpoints, Attack, Strength, Defence. Replace the single level/exp system with individual skill levels. Add hit/miss mechanics based on attack vs defence rolls.

## Phase 1: Server Core Skills Module

Create `rust-server/src/skills.rs` with core data structures and calculations.

### Files to Create/Modify
- **NEW**: `rust-server/src/skills.rs`

### Implementation Details

```rust
// XP table using RS formula
pub fn total_xp_for_level(level: i32) -> i64

// Level from total XP
pub fn level_for_xp(xp: i64) -> i32

// Combat level calculation
pub fn combat_level(skills: &Skills) -> i32

// Hit chance calculation
pub fn calculate_hit(attack_lvl: i32, attack_bonus: i32, defence_lvl: i32, defence_bonus: i32) -> bool

// Max hit calculation
pub fn calculate_max_hit(strength_lvl: i32, strength_bonus: i32) -> i32

// Skill and Skills structs
pub struct Skill { level: i32, xp: i64 }
pub struct Skills { hitpoints, attack, strength, defence }
```

### Checklist
- [ ] Create `skills.rs` with XP table function (RS formula)
- [ ] Add `level_for_xp()` to calculate level from total XP
- [ ] Add `Skill` struct with `new()`, `add_xp()`, `xp_to_next_level()` methods
- [ ] Add `Skills` struct with `new()` (HP 10, others 1), `combat_level()`
- [ ] Add `calculate_hit()` with attack roll vs defence roll
- [ ] Add `calculate_max_hit()` based on strength level + bonus
- [ ] Add `mod skills;` to `main.rs`

---

## Phase 2: Server Equipment Changes

Update equipment stats structure for skill-based requirements.

### Files to Modify
- `rust-server/src/data/item_def.rs`
- `rust-server/data/items/equipment.toml`

### Implementation Details

Change EquipmentStats:
```rust
pub struct EquipmentStats {
    pub slot_type: EquipmentSlot,
    pub attack_level_required: i32,   // For weapons
    pub defence_level_required: i32,  // For armor
    pub attack_bonus: i32,            // Accuracy
    pub strength_bonus: i32,          // Max hit
    pub defence_bonus: i32,           // Avoid hits
}
```

### Checklist
- [ ] Rename `level_required` to `attack_level_required` and `defence_level_required`
- [ ] Rename `damage_bonus` to `strength_bonus`
- [ ] Add `attack_bonus` field for accuracy
- [ ] Update equipment.toml with new field names
- [ ] Update `Player::damage_bonus()` → split into `attack_bonus()` and `strength_bonus()`
- [ ] Update equipment level check in equip handler

---

## Phase 3: Server Player Integration

Replace single level/exp fields with Skills struct in Player.

### Files to Modify
- `rust-server/src/game.rs`

### Implementation Details

Remove from Player struct:
- `level: i32`
- `exp: i32`
- `exp_to_next_level: i32`
- `max_hp: i32` (derived from hitpoints skill now)

Add to Player struct:
- `skills: Skills`

Update Player methods:
- `new()` - initialize with `Skills::new()`
- `max_hp()` - return `self.skills.hitpoints.level`
- Remove `award_exp()` - replace with skill-specific XP distribution
- Add `award_combat_xp(damage: i32)` - distribute XP to skills + HP

Update PlayerSaveData and PlayerUpdate structs similarly.

### Checklist
- [ ] Remove `level`, `exp`, `exp_to_next_level`, `max_hp` from `Player`
- [ ] Add `skills: Skills` to `Player`
- [ ] Update `Player::new()` to use `Skills::new()`, set `hp = 10`
- [ ] Add `Player::max_hp()` method returning hitpoints level
- [ ] Add `Player::combat_level()` method
- [ ] Add `Player::award_combat_xp(damage)` for XP distribution
- [ ] Update `PlayerSaveData` struct (skills fields)
- [ ] Update `PlayerUpdate` struct with skill levels and combat level
- [ ] Remove `exp_for_level()` function (replaced by skills module)

---

## Phase 4: Server Combat Overhaul

Update damage calculation to use skills and hit/miss mechanics.

### Files to Modify
- `rust-server/src/game.rs` (handle_attack)

### Implementation Details

Current flow:
1. Get damage_bonus from equipment
2. Calculate `base_attack_damage = BASE_DAMAGE + damage_bonus`
3. For players: `actual_dmg = (base_attack_damage - defense).max(1)`

New flow:
1. Get attacker's attack level, attack bonus (from equipment), strength level, strength bonus
2. Get defender's defence level, defence bonus
3. Roll hit: `calculate_hit(attack_lvl, attack_bonus, defence_lvl, defence_bonus)`
4. If miss: damage = 0, broadcast miss event
5. If hit: `max_hit = calculate_max_hit(strength_lvl, strength_bonus)`
6. Roll damage: `damage = rand(0, max_hit)`
7. Apply damage, award XP to attacker

### Checklist
- [ ] Update `handle_attack()` to get attacker's skill levels
- [ ] Update `handle_attack()` to get defender's skill levels (for PvP)
- [ ] Add hit roll using `calculate_hit()`
- [ ] Add damage roll using `calculate_max_hit()` + random
- [ ] On hit: apply damage, call `award_combat_xp(damage)`
- [ ] On miss: broadcast DamageEvent with damage=0 (client shows "MISS")
- [ ] Remove `BASE_DAMAGE` constant (now calculated from skills)
- [ ] NPCs: use their level as pseudo-attack/defence level

---

## Phase 5: Server Protocol Updates

Update network messages for skill system.

### Files to Modify
- `rust-server/src/protocol.rs`

### Implementation Details

Replace:
```rust
ExpGained { player_id, amount, total_exp, exp_to_next_level }
LevelUp { player_id, new_level, new_max_hp }
```

With:
```rust
SkillXp { player_id, skill: String, xp_gained: i32, total_xp: i64, level: i32 }
SkillLevelUp { player_id, skill: String, new_level: i32 }
```

Update PlayerUpdate to include all skill levels and combat level.

### Checklist
- [ ] Replace `ExpGained` with `SkillXp` message
- [ ] Replace `LevelUp` with `SkillLevelUp` message
- [ ] Add skill fields to `PlayerUpdate` (or add `skills` object)
- [ ] Update `encode_server_message()` for new message types
- [ ] Update combat flow to send skill-specific XP messages

---

## Phase 6: Server Database Migration

Update database schema for skills storage.

### Files to Modify
- `rust-server/src/db.rs`

### Implementation Details

Add skills_json column storing:
```json
{
  "hitpoints": {"level": 10, "xp": 1154},
  "attack": {"level": 1, "xp": 0},
  "strength": {"level": 1, "xp": 0},
  "defence": {"level": 1, "xp": 0}
}
```

### Checklist
- [ ] Add migration to add `skills_json` column with default new player skills
- [ ] Update `CharacterData` struct to use skills
- [ ] Update `save_character()` to serialize skills to JSON
- [ ] Update `get_character()` to deserialize skills from JSON
- [ ] Update `reserve_player_with_data()` to load skills into Player
- [ ] Update `get_player_save_data()` to include skills
- [ ] Remove old level/exp/max_hp columns in migration

---

## Phase 7: Client Skills Module

Mirror server skills data structures on client.

### Files to Create/Modify
- **NEW**: `client/src/game/skills.rs`
- `client/src/game/mod.rs`

### Checklist
- [ ] Create `skills.rs` with `Skill` and `Skills` structs
- [ ] Add XP table function for progress bar calculation
- [ ] Add `combat_level()` calculation
- [ ] Export from `game/mod.rs`

---

## Phase 8: Client Player Integration

Update client Player struct for skills.

### Files to Modify
- `client/src/game/entities.rs`

### Checklist
- [ ] Remove `level`, `exp`, `exp_to_next_level` from `Player`
- [ ] Add `skills: Skills` to `Player`
- [ ] Add `combat_level()` method
- [ ] Update `Player::new()` defaults

---

## Phase 9: Client Protocol Handling

Handle new skill messages from server.

### Files to Modify
- `client/src/network/client.rs`
- `client/src/network/messages.rs`

### Checklist
- [ ] Handle `skillXp` message - update player skill XP/level
- [ ] Handle `skillLevelUp` message - create level up event
- [ ] Update `StateSync` handling for skill fields in PlayerUpdate
- [ ] Update `PlayerJoined` handling if skills are included

---

## Phase 10: Client UI Updates

Update UI to display skills.

### Files to Modify
- `client/src/game/state.rs` (LevelUpEvent)
- `client/src/render/ui/bottom_bar.rs` (exp bar)
- `client/src/render/renderer.rs` (player overhead)

### Checklist
- [ ] Update `LevelUpEvent` to include skill name
- [ ] Update exp bar to show selected skill or combat level
- [ ] Update player overhead to show combat level instead of level
- [ ] Show skill-specific floating XP text

---

## Implementation Order

1. **Phase 1**: Server skills module (foundation)
2. **Phase 2**: Equipment stat changes (needed for combat)
3. **Phase 3**: Server player integration
4. **Phase 4**: Server combat overhaul
5. **Phase 5**: Protocol updates
6. **Phase 6**: Database migration
7. **Phase 7**: Client skills module
8. **Phase 8**: Client player integration
9. **Phase 9**: Client protocol handling
10. **Phase 10**: Client UI updates

---

## Testing Checklist

- [ ] New character starts with HP 10, Attack/Str/Def 1, Combat 3
- [ ] Combat level calculates correctly at various skill levels
- [ ] Hit/miss mechanics work (test with varying attack/defence)
- [ ] Damage scales with strength level
- [ ] XP is awarded to all combat skills on kill
- [ ] Level ups trigger for each skill independently
- [ ] Equipment requirements check correct skill (attack for weapons, defence for armor)
- [ ] Database saves/loads skills correctly
- [ ] Client displays all skills and combat level
- [ ] Floating XP text shows skill name
