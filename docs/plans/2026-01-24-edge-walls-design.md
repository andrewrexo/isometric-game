# Edge-Aligned Walls Design

## Overview

Add support for walls that align to tile edges (down and right) rather than filling entire tiles. This enables building room boundaries, building exteriors, and other architectural elements in the isometric game.

## Design Decisions

- **Down + Right edges only** - Walls only go on the bottom-left (down) and bottom-right (right) edges of tiles, which are the visible edges from the isometric camera angle
- **Manual edge selection** - User picks "Down Wall" or "Right Wall" tool in the mapper; no automatic detection
- **Separate walls array** - Walls stored in their own array in the chunk data, separate from mapObjects
- **Bottom vertex anchoring** - Both wall types anchor at the tile's bottom vertex so corners connect naturally
- **No collision for now** - Walls are purely visual; collision can be added later

## Data Structures

### Map File Format

Add a `walls` array to the chunk JSON:

```json
{
  "version": 2,
  "coord": { "cx": 0, "cy": 0 },
  "layers": { "ground": [...], "objects": [...], "overhead": [...] },
  "collision": "base64...",
  "mapObjects": [...],
  "walls": [
    { "gid": 101, "x": 5, "y": 10, "edge": "down" },
    { "gid": 200, "x": 5, "y": 10, "edge": "right" },
    { "gid": 200, "x": 6, "y": 10, "edge": "right" }
  ]
}
```

### Protocol Structures (Rust)

```rust
#[derive(Serialize, Deserialize, Clone)]
pub enum WallEdge {
    Down,
    Right,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChunkWallData {
    pub gid: u32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub edge: WallEdge,
}

// Add to ChunkData:
pub struct ChunkData {
    // ... existing fields
    pub walls: Vec<ChunkWallData>,
}
```

### Mapper Structures (TypeScript)

```typescript
interface SimplifiedWall {
  gid: number;
  x: number;
  y: number;
  edge: "down" | "right";
}

interface SimplifiedChunk {
  // ... existing fields
  walls: SimplifiedWall[];
}
```

## Rendering Logic

### Positioning Formula

Both wall types anchor at the tile's bottom vertex:

```rust
// Get the bottom vertex of the tile in screen coordinates
let (tile_center_x, tile_center_y) = world_to_screen(tile_x, tile_y, camera);
let bottom_vertex_x = tile_center_x;
let bottom_vertex_y = tile_center_y + (TILE_HEIGHT / 2.0);  // +16px to bottom point

match edge {
    WallEdge::Down => {
        // Sprite's bottom-right corner at bottom vertex
        draw_x = bottom_vertex_x - scaled_width;
        draw_y = bottom_vertex_y - scaled_height;
    }
    WallEdge::Right => {
        // Sprite's bottom-left corner at bottom vertex
        draw_x = bottom_vertex_x;
        draw_y = bottom_vertex_y - scaled_height;
    }
}
```

### Render Order

Walls render after the ground layer but use depth sorting with other objects. A wall at `(x, y)` has depth `x + y`, same as entities/objects at that tile.

### Visual Reference

```
        /\
       /  \
      /    \
     /  tile \
    /    ◆    \
   /____________\
  ↙              ↘
DOWN            RIGHT
wall            wall
```

Both walls share the bottom vertex as their anchor point.

## Implementation Components

| Component | Changes |
|-----------|---------|
| Map format | Add `walls` array to chunk JSON |
| Server | Parse walls from JSON, add to ChunkData protocol |
| Client | Receive walls, render at tile edges with depth sorting |
| Mapper | Two wall tools (down/right), wall sprite palette, export walls array |

## Mapper UI

1. User selects a wall sprite from the object palette (filtered to wall GIDs)
2. User selects either "Down Wall" or "Right Wall" tool
3. Click on a tile → wall is added to the chunk's `walls` array
4. Click again on same tile+edge → removes the wall (toggle behavior)

## Future Considerations

- Edge-based collision (blocking movement across specific tile edges)
- Wall metadata file (`walls.json`) for sprite dimensions if needed
- Corner pieces for L-shaped wall connections
