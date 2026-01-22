# Ranged Weapons Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add ranged weapon support with per-weapon range, line-of-sight checks, arrow projectiles, and ShootingBow animation.

**Architecture:** Server adds WeaponType enum and range field to items, extends handle_attack() with range/LOS logic. Client checks weapon_type for animation selection and renders projectiles from DamageEvent data.

**Tech Stack:** Rust server, Rust/macroquad client, TOML item definitions, MessagePack protocol

---

### Task 1: Add WeaponType enum and fields to server item_def.rs

**Files:**
- Modify: `rust-server/src/data/item_def.rs:63-87`

**Step 1: Add WeaponType enum after EquipmentSlot**

Add this after line 57 (after `EquipmentSlot` impl block):

```rust
// ============================================================================
// Weapon Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WeaponType {
    #[default]
    Melee,
    Ranged,
}
```

**Step 2: Add fields to EquipmentStats**

Add these fields to `EquipmentStats` struct after `defence_bonus`:

```rust
    /// Weapon type - determines animation and attack behavior
    #[serde(default)]
    pub weapon_type: WeaponType,

    /// Attack range in tiles (1 = melee adjacent, higher = ranged)
    #[serde(default = "default_range")]
    pub range: i32,
```

**Step 3: Add default_range function**

Add after the struct:

```rust
fn default_range() -> i32 { 1 }
```

**Step 4: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compiles with no errors

**Step 5: Commit**

```bash
git add rust-server/src/data/item_def.rs
git commit -m "$(cat <<'EOF'
Add WeaponType enum and range field to EquipmentStats

- WeaponType: Melee (default) or Ranged
- range: attack distance in tiles (default 1)
EOF
)"
```

---

### Task 2: Update equipment.toml with ranged weapon data

**Files:**
- Modify: `rust-server/data/items/equipment.toml:376-404`

**Step 1: Update long_bow definition**

Change lines 384-389 to:

```toml
[long_bow.equipment]
slot_type = "weapon"
weapon_type = "ranged"
range = 8
attack_level_required = 20
attack_bonus = 25
strength_bonus = 18
defence_bonus = 0
```

**Step 2: Update dark_bow definition**

Change lines 400-404 to:

```toml
[dark_bow.equipment]
slot_type = "weapon"
weapon_type = "ranged"
range = 10
attack_level_required = 50
attack_bonus = 48
strength_bonus = 55
defence_bonus = 0
```

**Step 3: Verify server loads definitions**

Run: `cd rust-server && cargo run`
Expected: Server starts without TOML parse errors

**Step 4: Commit**

```bash
git add rust-server/data/items/equipment.toml
git commit -m "$(cat <<'EOF'
Mark long_bow and dark_bow as ranged weapons

- long_bow: range 8 tiles
- dark_bow: range 10 tiles
EOF
)"
```

---

### Task 3: Add line-of-sight check to tilemap.rs

**Files:**
- Modify: `rust-server/src/tilemap.rs`

**Step 1: Add has_line_of_sight function**

Add this function to the `Tilemap` impl block:

```rust
    /// Check if there's a clear line of sight between two points (Bresenham's line)
    /// Returns true if no solid tiles block the path
    pub fn has_line_of_sight(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            // Don't check start position (attacker's tile)
            if (x != x0 || y != y0) && !self.is_tile_walkable(x, y) {
                return false;
            }

            if x == x1 && y == y1 {
                return true;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                if x == x1 { return true; }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == y1 { return true; }
                err += dx;
                y += sy;
            }
        }
    }
```

**Step 2: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add rust-server/src/tilemap.rs
git commit -m "$(cat <<'EOF'
Add line-of-sight check using Bresenham's algorithm

Used for ranged weapon attacks to detect wall blocking.
EOF
)"
```

---

### Task 4: Update handle_attack for ranged weapons

**Files:**
- Modify: `rust-server/src/game.rs:1169-1370`

**Step 1: Get weapon info after getting player stats**

After line 1201 (after the existing stat gathering), add weapon lookup:

```rust
        // Get weapon range and type
        let (weapon_range, weapon_type) = {
            let players = self.players.read().await;
            if let Some(player) = players.get(player_id) {
                if let Some(ref weapon_id) = player.equipped_weapon {
                    if let Some(item_def) = self.item_registry.get(weapon_id) {
                        if let Some(ref equip) = item_def.equipment {
                            (equip.range, equip.weapon_type)
                        } else {
                            (1, WeaponType::Melee)
                        }
                    } else {
                        (1, WeaponType::Melee)
                    }
                } else {
                    (1, WeaponType::Melee) // Unarmed = melee range 1
                }
            } else {
                return;
            }
        };
```

**Step 2: Add import for WeaponType at top of file**

Add to imports:

```rust
use crate::data::item_def::WeaponType;
```

**Step 3: Replace target finding logic**

Replace lines 1210-1261 (the target finding section) with ranged-aware version:

```rust
        // Find target based on weapon range
        let mut target_id: Option<String> = None;
        let mut is_npc = false;
        let mut target_tile_x = attacker_x;
        let mut target_tile_y = attacker_y;

        // Direction vectors for 8 directions
        let (dir_dx, dir_dy): (i32, i32) = match attacker_dir {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
            Direction::UpLeft => (-1, -1),
            Direction::UpRight => (1, -1),
            Direction::DownLeft => (-1, 1),
            Direction::DownRight => (1, 1),
        };

        // Scan tiles in facing direction up to weapon range
        for dist in 1..=weapon_range {
            let check_x = attacker_x + dir_dx * dist;
            let check_y = attacker_y + dir_dy * dist;

            // For ranged weapons, check line of sight
            if weapon_range > 1 && !self.tilemap.has_line_of_sight(attacker_x, attacker_y, check_x, check_y) {
                tracing::debug!("{} ranged attack blocked by wall at ({}, {})", attacker_name, check_x, check_y);
                break;
            }

            // Check NPCs at this tile
            {
                let npcs = self.npcs.read().await;
                for (npc_id, npc) in npcs.iter() {
                    if npc.is_alive() && npc.is_attackable() && npc.x == check_x && npc.y == check_y {
                        target_id = Some(npc_id.clone());
                        is_npc = true;
                        target_tile_x = check_x;
                        target_tile_y = check_y;
                        tracing::info!("{} found NPC target: {} at ({}, {}) range {}", attacker_name, npc.name(), check_x, check_y, dist);
                        break;
                    }
                }
            }
            if target_id.is_some() { break; }

            // Check players at this tile
            {
                let players = self.players.read().await;
                for (pid, player) in players.iter() {
                    if pid != player_id && player.active && player.hp > 0 && player.x == check_x && player.y == check_y {
                        target_id = Some(pid.clone());
                        is_npc = false;
                        target_tile_x = check_x;
                        target_tile_y = check_y;
                        tracing::info!("{} found player target: {} at ({}, {}) range {}", attacker_name, player.name, check_x, check_y, dist);
                        break;
                    }
                }
            }
            if target_id.is_some() { break; }
        }

        // No valid target found
        let target_id = match target_id {
            Some(id) => id,
            None => {
                tracing::debug!("{} attack missed - no target in range {} facing {:?}", attacker_name, weapon_range, attacker_dir);
                return;
            }
        };
```

**Step 4: Update target position for DamageEvent**

Find lines 1355-1357 and replace with:

```rust
        // Use actual target position for damage event (important for ranged projectiles)
        let target_x = target_tile_x as f32;
        let target_y = target_tile_y as f32;
```

**Step 5: Add projectile field to DamageEvent**

Update the DamageEvent broadcast around line 1360:

```rust
        // Determine projectile type for ranged attacks
        let projectile = if weapon_type == WeaponType::Ranged {
            Some("arrow".to_string())
        } else {
            None
        };

        // Broadcast damage event to all clients
        let damage_msg = ServerMessage::DamageEvent {
            source_id: player_id.to_string(),
            target_id: target_id.clone(),
            damage: actual_damage,
            target_hp,
            target_x,
            target_y,
            projectile,
        };
        self.broadcast(damage_msg).await;
```

**Step 6: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Error about projectile field - will fix in next task

---

### Task 5: Update DamageEvent in protocol.rs

**Files:**
- Modify: `rust-server/src/protocol.rs:132-139, 712-746`

**Step 1: Update DamageEvent struct**

Change the struct definition (around line 132):

```rust
    DamageEvent {
        source_id: String,
        target_id: String,
        damage: i32,
        target_hp: i32,
        target_x: f32,
        target_y: f32,
        projectile: Option<String>,
    },
```

**Step 2: Update DamageEvent encoding**

Update the encoding block (around line 712):

```rust
        ServerMessage::DamageEvent {
            source_id,
            target_id,
            damage,
            target_hp,
            target_x,
            target_y,
            projectile,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("source_id".into()),
                Value::String(source_id.clone().into()),
            ));
            map.push((
                Value::String("target_id".into()),
                Value::String(target_id.clone().into()),
            ));
            map.push((
                Value::String("damage".into()),
                Value::Integer((*damage as i64).into()),
            ));
            map.push((
                Value::String("target_hp".into()),
                Value::Integer((*target_hp as i64).into()),
            ));
            map.push((
                Value::String("target_x".into()),
                Value::F64(*target_x as f64),
            ));
            map.push((
                Value::String("target_y".into()),
                Value::F64(*target_y as f64),
            ));
            map.push((
                Value::String("projectile".into()),
                match projectile {
                    Some(p) => Value::String(p.clone().into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
```

**Step 3: Verify server compiles and runs**

Run: `cd rust-server && cargo build && cargo run`
Expected: Server starts successfully

**Step 4: Commit server changes**

```bash
git add rust-server/src/data/item_def.rs rust-server/src/tilemap.rs rust-server/src/game.rs rust-server/src/protocol.rs
git commit -m "$(cat <<'EOF'
Implement ranged weapon attack logic on server

- Scan tiles in facing direction up to weapon range
- Check line of sight for ranged weapons
- Add projectile field to DamageEvent for client rendering
EOF
)"
```

---

### Task 6: Update ClientItemDef with weapon_type and range

**Files:**
- Modify: `rust-server/src/protocol.rs:343-360, 1108-1146`

**Step 1: Add fields to ClientItemDef struct**

Add after `defence_bonus` (around line 359):

```rust
    pub weapon_type: Option<String>,
    pub range: Option<i32>,
```

**Step 2: Update ItemDefinitions encoding**

In the encoding block for ItemDefinitions (around line 1122), add after defence_bonus handling:

```rust
                    if let Some(ref slot) = i.equipment_slot {
                        imap.push((Value::String("equipment_slot".into()), Value::String(slot.clone().into())));
                    }
                    if let Some(ref wtype) = i.weapon_type {
                        imap.push((Value::String("weapon_type".into()), Value::String(wtype.clone().into())));
                    }
                    if let Some(r) = i.range {
                        imap.push((Value::String("range".into()), Value::Integer((r as i64).into())));
                    }
```

**Step 3: Update item sync code to populate new fields**

Find where `ClientItemDef` is constructed (search for `ClientItemDef {`) and add the new fields:

```rust
weapon_type: item.equipment.as_ref().map(|e| {
    match e.weapon_type {
        WeaponType::Melee => "melee".to_string(),
        WeaponType::Ranged => "ranged".to_string(),
    }
}),
range: item.equipment.as_ref().map(|e| e.range),
```

**Step 4: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compiles with no errors

**Step 5: Commit**

```bash
git add rust-server/src/protocol.rs
git commit -m "$(cat <<'EOF'
Send weapon_type and range to client in ItemDefinitions
EOF
)"
```

---

### Task 7: Update client ItemDefinition with weapon_type and range

**Files:**
- Modify: `client/src/game/state.rs` (find ItemDefinition struct and related code)

**Step 1: Find and update ItemDefinition struct**

Search for `pub struct ItemDefinition` in client code. Add fields:

```rust
    pub weapon_type: Option<String>,
    pub range: Option<i32>,
```

**Step 2: Find and update EquipmentStats if separate**

If there's a separate EquipmentStats struct, add the fields there instead.

**Step 3: Update parsing of ItemDefinitions message**

Find where itemDefinitions message is parsed in `client/src/network/client.rs` and add:

```rust
weapon_type: extract_string(&imap, "weapon_type"),
range: extract_i32(&imap, "range"),
```

**Step 4: Verify client compiles**

Run: `cd client && cargo check`
Expected: Compiles with no errors

**Step 5: Commit**

```bash
git add client/src/game/state.rs client/src/network/client.rs
git commit -m "$(cat <<'EOF'
Add weapon_type and range fields to client ItemDefinition
EOF
)"
```

---

### Task 8: Update client DamageEvent to include projectile

**Files:**
- Modify: `client/src/game/state.rs:198-204`
- Modify: `client/src/network/client.rs` (damageEvent handling)

**Step 1: Add projectile field to DamageEvent struct**

Update the struct:

```rust
pub struct DamageEvent {
    pub x: f32,
    pub y: f32,
    pub damage: i32,
    pub time: f64,
    pub target_id: String,
    pub source_id: Option<String>,
    pub projectile: Option<String>,
}
```

**Step 2: Update damageEvent parsing in network/client.rs**

Find where DamageEvent is created from damageEvent message and add:

```rust
source_id: extract_string(data, "source_id"),
projectile: extract_string(data, "projectile"),
```

**Step 3: Update all existing DamageEvent constructions**

Search for `DamageEvent {` and add the new fields with defaults:
- `source_id: None,`
- `projectile: None,`

For healing/regen events that don't come from server.

**Step 4: Verify client compiles**

Run: `cd client && cargo check`
Expected: Compiles with no errors

**Step 5: Commit**

```bash
git add client/src/game/state.rs client/src/network/client.rs
git commit -m "$(cat <<'EOF'
Add source_id and projectile fields to client DamageEvent
EOF
)"
```

---

### Task 9: Add Projectile struct and storage to GameState

**Files:**
- Modify: `client/src/game/state.rs`

**Step 1: Add Projectile struct**

Add near other event structs:

```rust
/// Active projectile for ranged attack visualization
pub struct Projectile {
    pub sprite: String,
    pub start_x: f32,
    pub start_y: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub start_time: f64,
    pub duration: f64,
}

impl Projectile {
    /// Get current position (0.0 to 1.0 progress)
    pub fn progress(&self, current_time: f64) -> f32 {
        let elapsed = current_time - self.start_time;
        (elapsed / self.duration).min(1.0) as f32
    }

    /// Check if projectile animation is complete
    pub fn is_complete(&self, current_time: f64) -> bool {
        current_time - self.start_time >= self.duration
    }

    /// Get current world position
    pub fn current_pos(&self, current_time: f64) -> (f32, f32) {
        let t = self.progress(current_time);
        let x = self.start_x + (self.end_x - self.start_x) * t;
        let y = self.start_y + (self.end_y - self.start_y) * t;
        (x, y)
    }
}
```

**Step 2: Add projectiles Vec to GameState**

Find GameState struct and add:

```rust
    pub projectiles: Vec<Projectile>,
```

**Step 3: Initialize in GameState::new()**

Add to initialization:

```rust
projectiles: Vec::new(),
```

**Step 4: Verify compilation**

Run: `cd client && cargo check`
Expected: Compiles with no errors

**Step 5: Commit**

```bash
git add client/src/game/state.rs
git commit -m "$(cat <<'EOF'
Add Projectile struct for ranged attack visualization
EOF
)"
```

---

### Task 10: Spawn projectiles from DamageEvent

**Files:**
- Modify: `client/src/network/client.rs` (damageEvent handler)

**Step 1: Spawn projectile when DamageEvent has projectile field**

In the damageEvent handling code, after creating the DamageEvent, add:

```rust
                    // Spawn projectile for ranged attacks
                    if let Some(ref projectile_type) = projectile {
                        if let Some(ref source_id) = source_id {
                            // Get source position
                            let source_pos = if let Some(player) = state.players.get(source_id) {
                                Some((player.x, player.y))
                            } else {
                                None
                            };

                            if let Some((src_x, src_y)) = source_pos {
                                state.projectiles.push(crate::game::Projectile {
                                    sprite: projectile_type.clone(),
                                    start_x: src_x,
                                    start_y: src_y,
                                    end_x: target_x,
                                    end_y: target_y,
                                    start_time: current_time,
                                    duration: 0.15, // Fast arrow travel
                                });
                            }
                        }
                    }
```

**Step 2: Verify compilation**

Run: `cd client && cargo check`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add client/src/network/client.rs
git commit -m "$(cat <<'EOF'
Spawn projectiles from ranged DamageEvents
EOF
)"
```

---

### Task 11: Add projectile rendering

**Files:**
- Modify: `client/src/render/mod.rs` or wherever entities are rendered

**Step 1: Find the render loop**

Search for where damage_events are rendered or where entities are drawn.

**Step 2: Add projectile update and rendering**

Add projectile cleanup and rendering (typically in the main render function):

```rust
        // Update and render projectiles
        let current_time = macroquad::time::get_time();
        state.projectiles.retain(|p| !p.is_complete(current_time));

        for projectile in &state.projectiles {
            let (world_x, world_y) = projectile.current_pos(current_time);
            let (screen_x, screen_y) = world_to_screen(world_x, world_y, camera_x, camera_y);

            // Calculate rotation angle from start to end
            let dx = projectile.end_x - projectile.start_x;
            let dy = projectile.end_y - projectile.start_y;
            let angle = dy.atan2(dx);

            // Draw arrow sprite rotated toward target
            // For now, draw a simple line or use arrow texture if available
            draw_line(
                screen_x - dx * 8.0,
                screen_y - dy * 8.0,
                screen_x + dx * 8.0,
                screen_y + dy * 8.0,
                2.0,
                BROWN,
            );
        }
```

**Step 3: Verify compilation and test visually**

Run: `cd client && cargo run`
Expected: Arrows visible when shooting bow at enemies

**Step 4: Commit**

```bash
git add client/src/render/
git commit -m "$(cat <<'EOF'
Render arrow projectiles for ranged attacks
EOF
)"
```

---

### Task 12: Update client attack animation selection

**Files:**
- Modify: `client/src/input/handler.rs` (around line 1580 where Attack is handled)

**Step 1: Find where attack animation is set**

Search for where `AnimationState::Attacking` is set on the local player.

**Step 2: Check weapon type and select animation**

When attack command is processed, check equipped weapon:

```rust
        // Set attack animation based on weapon type
        if let Some(player) = state.get_local_player_mut() {
            let anim_state = if let Some(ref weapon_id) = player.equipped_weapon {
                if let Some(item_def) = state.item_definitions.get(weapon_id) {
                    if item_def.weapon_type.as_deref() == Some("ranged") {
                        AnimationState::ShootingBow
                    } else {
                        AnimationState::Attacking
                    }
                } else {
                    AnimationState::Attacking
                }
            } else {
                AnimationState::Attacking
            };
            player.animation.set_state(anim_state);
        }
```

**Step 3: Verify compilation**

Run: `cd client && cargo check`
Expected: Compiles with no errors

**Step 4: Commit**

```bash
git add client/src/input/handler.rs
git commit -m "$(cat <<'EOF'
Select ShootingBow animation for ranged weapons
EOF
)"
```

---

### Task 13: Integration test

**Step 1: Start server**

Run: `cd rust-server && cargo run`

**Step 2: Start client**

Run: `cd client && cargo run`

**Step 3: Test melee attack**

- Equip a melee weapon (or no weapon)
- Attack an enemy at melee range
- Verify: Attack animation plays, damage appears

**Step 4: Test ranged attack**

- Equip long_bow
- Stand several tiles away from enemy
- Attack facing the enemy
- Verify:
  - ShootingBow animation plays
  - Arrow projectile flies to target
  - Damage number appears at target

**Step 5: Test line of sight**

- Stand behind a wall from enemy
- Try to attack
- Verify: Attack does not hit (blocked by wall)

**Step 6: Final commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
Complete ranged weapons implementation

- Server: WeaponType enum, range field, LOS check
- Client: ShootingBow animation, arrow projectiles
- Tested: melee, ranged, wall blocking
EOF
)"
```

---

## Summary

| Task | Description |
|------|-------------|
| 1 | Add WeaponType enum to server item_def.rs |
| 2 | Update equipment.toml with ranged data |
| 3 | Add line-of-sight to tilemap.rs |
| 4 | Update handle_attack for range/LOS |
| 5 | Update DamageEvent in protocol.rs |
| 6 | Send weapon_type/range to client |
| 7 | Client ItemDefinition fields |
| 8 | Client DamageEvent projectile field |
| 9 | Add Projectile struct |
| 10 | Spawn projectiles from events |
| 11 | Render projectiles |
| 12 | Animation selection |
| 13 | Integration test |
