# Simple Server-Authoritative Movement

## Problem

The current client-side prediction system is over-engineered and causes:
- Diagonal corrections on direction changes
- Direction jitter from client/server state fighting
- Complex code that's hard to reason about

## Solution

Remove client-side prediction entirely. Server is the single source of truth, client just animates smoothly.

## Core Principle

Server sends position + velocity. Client sets target and interpolates. That's it.

---

## Server Changes

### Remove velocity tracking fields

The `vel_x/vel_y` fields we added to Player are unnecessary. Just send `move_dx/move_dy` directly.

**File: `rust-server/src/game.rs`**

Remove from Player struct:
```rust
// DELETE these lines
pub vel_x: i32,
pub vel_y: i32,
```

Remove from Player::new():
```rust
// DELETE these lines
vel_x: 0,
vel_y: 0,
```

### Send move intent directly

In the tick() function, send `move_dx/move_dy` as velocity:

```rust
player_updates.push(PlayerUpdate {
    // ...
    vel_x: player.move_dx,  // Intent, not separate vel field
    vel_y: player.move_dy,
    // ...
});
```

### Remove velocity management code

Delete all the velocity setting/clearing code we added:
- "Apply valid moves and update velocity" section
- "Clear velocity for players who want to stop" section
- `vel_x = 0; vel_y = 0;` from `die()`, `handle_face()`, attack handler

### Keep direction update in handle_move()

This is correct - direction should update immediately when player presses a key:
```rust
if move_dx != 0 || move_dy != 0 {
    player.direction = Direction::from_velocity(move_dx as f32, move_dy as f32);
}
```

---

## Client Changes

### Remove prediction fields from Player struct

**File: `client/src/game/entities.rs`**

```rust
// DELETE these fields
pub predicted_x: f32,
pub predicted_y: f32,
pub has_pending_prediction: bool,
```

And remove from `Player::new()`.

### Delete apply_local_input() method

Remove the entire method - we don't predict anymore.

### Simplify set_server_position_with_velocity()

Replace the complex method with simple logic:

```rust
pub fn set_server_state(&mut self, x: f32, y: f32, vel_x: f32, vel_y: f32, dir: Direction) {
    self.server_x = x;
    self.server_y = y;
    self.vel_x = vel_x;
    self.vel_y = vel_y;
    self.direction = dir;

    // Teleport detection (>2 tiles = snap)
    let dist = ((self.x - x).powi(2) + (self.y - y).powi(2)).sqrt();
    if dist > 2.0 {
        self.x = x;
        self.y = y;
    }

    // Target = next tile if moving, current tile if stopped
    if vel_x != 0.0 || vel_y != 0.0 {
        self.target_x = x + vel_x;
        self.target_y = y + vel_y;
    } else {
        self.target_x = x;
        self.target_y = y;
    }
}
```

### Simplify interpolate_visual()

Remove all prediction logic, just interpolate toward target:

```rust
pub fn interpolate_visual(&mut self, delta: f32) {
    let dx = self.target_x - self.x;
    let dy = self.target_y - self.y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < 0.01 {
        self.x = self.target_x;
        self.y = self.target_y;
        self.is_moving = false;
    } else {
        let move_dist = VISUAL_SPEED * delta;
        if dist <= move_dist {
            self.x = self.target_x;
            self.y = self.target_y;
        } else {
            self.x += (dx / dist) * move_dist;
            self.y += (dy / dist) * move_dist;
        }
        self.is_moving = true;
    }

    self.update_animation(delta);
}
```

### Simplify input handler

**File: `client/src/input/handler.rs`**

Remove the `apply_local_input()` call - input just sends commands to server, doesn't touch local state.

### Simplify state.rs update

**File: `client/src/game/state.rs`**

- Remove `input_dx, input_dy` parameters from `update()`
- Remove direction setting from local input
- Just call `interpolate_visual(delta)` for all players

### Simplify network client

**File: `client/src/network/client.rs`**

- Direction always comes from server for all players
- Remove special cases for local player direction
- Call simplified `set_server_state()` method

---

## Animation Handling

Direction for animation comes from server. The `update_animation()` method should:
- Use `self.direction` (from server) for sprite direction
- Use `self.is_moving` for walk vs idle animation
- Keep existing action animation logic (attack, cast, etc.)

---

## Summary

| File | Changes |
|------|---------|
| `rust-server/src/game.rs` | Remove `vel_x/vel_y` fields, send `move_dx/move_dy` directly, remove velocity management |
| `client/src/game/entities.rs` | Remove prediction fields, delete `apply_local_input()`, simplify state/interpolate methods |
| `client/src/input/handler.rs` | Remove `apply_local_input()` call |
| `client/src/game/state.rs` | Simplify update, remove input direction parameters |
| `client/src/network/client.rs` | Simplify state sync, direction from server for all |

## Future Optimization

- Consider lowering tick rate from 20Hz to 10Hz once movement is stable
