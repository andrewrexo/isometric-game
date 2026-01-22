# Ranged Weapons Design

## Overview

Add ranged weapon support to the combat system. Bows can attack targets at a distance with visible arrow projectiles, line-of-sight checks, and appropriate animations.

## Data Model

### New Fields in EquipmentStats

```toml
# equipment.toml example
[long_bow]
display_name = "Long Bow"
sprite = "long_bow"
category = "equipment"
max_stack = 1
base_price = 250
sellable = true

[long_bow.equipment]
slot_type = "weapon"
weapon_type = "ranged"        # NEW: "melee" (default) or "ranged"
range = 8                     # NEW: attack range in tiles (default: 1)
attack_level_required = 20
attack_bonus = 25
strength_bonus = 18
defence_bonus = 0
# ammo_type = "arrow"         # FUTURE: optional, for ammo system
```

### Rust Struct Changes (item_def.rs)

```rust
pub enum WeaponType {
    Melee,   // default
    Ranged,
}

pub struct EquipmentStats {
    // ... existing fields ...
    pub weapon_type: WeaponType,  // defaults to Melee
    pub range: i32,               // defaults to 1
}
```

- `weapon_type` drives client animation selection
- `range` determines attack distance on server
- Melee weapons default to `weapon_type = "melee"` and `range = 1` if not specified
- Existing items work unchanged

## Server Combat Logic

### Updated handle_attack() Flow

1. Check cooldown
2. Get equipped weapon's range and weapon_type
3. Find target within range (scan tiles in facing direction)
4. Line-of-sight check - reject if wall blocks path
5. Calculate hit/damage (same formula as melee, isolated for future changes)
6. Apply damage
7. Send combat event (include projectile info for ranged)

### Line of Sight Check

- Bresenham's line algorithm from attacker to target
- Check each tile along the path against collision map
- If any solid tile (wall) is hit before target, attack fails
- Entities don't block shots
- Future: piercing ammo can track and damage entities along path

### Target Finding for Ranged

- Scan tiles in facing direction up to weapon's range
- Find first valid target (NPC or player) along that line

## Client Animation & Projectiles

### Animation Selection (input/handler.rs)

```rust
// When attack is triggered:
// 1. Look up equipped_weapon in item_definitions
// 2. Check weapon_type field
// 3. Set animation state accordingly

match weapon_type {
    WeaponType::Ranged => AnimationState::ShootingBow,
    WeaponType::Melee => AnimationState::Attacking,
}
```

### Projectile System (new)

```rust
pub struct Projectile {
    pub sprite: String,        // "arrow" for now
    pub start_pos: Vec2,       // world position
    pub end_pos: Vec2,         // target world position
    pub progress: f32,         // 0.0 to 1.0
    pub speed: f32,            // tiles per second
}
```

### Projectile Flow

1. Server sends `CombatEvent` with `projectile: Some("arrow")` for ranged attacks
2. Client spawns `Projectile` from attacker position to target position
3. Each frame: lerp position based on progress, render arrow sprite rotated toward target
4. When progress reaches 1.0, remove projectile (damage already applied)

## Network Messages

### Updated CombatEvent

```rust
pub struct CombatEvent {
    pub attacker_id: u32,
    pub target_id: u32,
    pub damage: i32,
    pub is_hit: bool,
    // NEW fields:
    pub projectile: Option<String>,  // None for melee, Some("arrow") for ranged
    pub target_pos: (f32, f32),      // target world position for projectile endpoint
}
```

## Files to Modify

| File | Changes |
|------|---------|
| `rust-server/src/data/item_def.rs` | Add `WeaponType` enum, `weapon_type` and `range` fields |
| `rust-server/data/items/equipment.toml` | Add `weapon_type = "ranged"` and `range` to bows |
| `rust-server/src/game.rs` | Update `handle_attack()` with range check, line-of-sight |
| `rust-server/src/skills.rs` | Isolate damage calc for future ranged adjustments |
| `shared/src/messages.rs` (or equivalent) | Update `CombatEvent` struct |
| `client/src/input/handler.rs` | Select animation based on weapon_type |
| `client/src/render/` (new file) | Add projectile rendering system |
| `client/src/game/` | Add projectile state management |

## Future Considerations

- **Ammunition system**: Optional `ammo_type` field on weapons, arrows as consumables
- **Piercing ammo**: Pass through and damage multiple entities along path
- **Different damage formula**: Ranged-specific calculations if needed
- **Minimum range**: Some weapons can't fire point-blank

## Assets Needed

- `arrow.png` sprite for projectile rendering
