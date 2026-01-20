# Entity Collision Design

NPCs and players should act as unwalkable tiles, preventing entities from walking through each other. This affects both movement validation and pathfinding.

## Behavior

- Players cannot walk into NPCs or other players
- NPCs cannot walk into players or other NPCs
- Pathfinding routes around entities
- When a path becomes blocked mid-movement, re-path around the obstacle

## Implementation

### 1. Server-Side Movement Validation

In `game.rs`, add a check before accepting any move:

```rust
fn is_position_free(&self, x: i32, y: i32, exclude_entity_id: Option<u64>) -> bool {
    // Check static tile collision
    if !self.world.is_tile_walkable(x, y) {
        return false;
    }

    // Check other players
    for (id, player) in &self.players {
        if Some(*id) == exclude_entity_id { continue; }
        if player.x == x && player.y == y {
            return false;
        }
    }

    // Check NPCs
    for npc in &self.npcs {
        if npc.x == x && npc.y == y {
            return false;
        }
    }

    true
}
```

Use this instead of `world.is_tile_walkable()` when validating player and NPC moves.

### 2. Client-Side Pathfinding

Update `pathfinding.rs` to accept occupied positions:

```rust
pub fn find_path(
    start: (i32, i32),
    goal: (i32, i32),
    chunk_manager: &ChunkManager,
    occupied: &HashSet<(i32, i32)>,
    max_distance: i32,
) -> Option<Vec<(i32, i32)>>
```

In the A* neighbor loop, skip occupied tiles (except the goal):

```rust
if neighbor != goal && occupied.contains(&neighbor) {
    continue;
}
```

Before calling pathfinding, build the occupied set from known entity positions (excluding self).

### 3. NPC Movement

In `game.rs` where NPCs are updated, add player positions to the `occupied_tiles` list:

```rust
let mut occupied_tiles: Vec<(i32, i32)> = /* existing NPC positions */;

// Add player positions
for player in self.players.values() {
    occupied_tiles.push((player.x, player.y));
}
```

No changes needed to `npc.rs` - it already respects `occupied_tiles`.

### 4. Client Re-Pathing

When following auto-path, before moving to the next waypoint:

1. Check if the next waypoint is occupied
2. If blocked, recalculate path from current position to original goal
3. If no path exists, cancel movement

```rust
if occupied.contains(&next_waypoint) {
    if let Some(new_path) = find_path(current, goal, chunk_manager, &occupied, max_distance) {
        path_state.waypoints = new_path;
        path_state.current_index = 0;
    } else {
        state.auto_path = None;
    }
}
```

## Files to Modify

- `rust-server/src/game.rs` - Movement validation, NPC occupied tiles
- `client/src/game/pathfinding.rs` - Accept occupied set parameter
- `client/src/input/handler.rs` - Build occupied set, re-pathing logic
