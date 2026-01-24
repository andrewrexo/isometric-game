# Edge-Aligned Walls Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add support for walls that align to tile edges (down and right) for building room boundaries and architectural elements.

**Architecture:** Walls are stored in a separate `walls` array in chunk data, sent via protocol to client, and rendered anchored at the tile's bottom vertex. The mapper gets two wall tools for placing down/right walls.

**Tech Stack:** Rust (server + client), TypeScript/React (mapper)

---

## Task 1: Add Wall Types to Server Chunk Module

**Files:**
- Modify: `rust-server/src/chunk.rs:26-39`

**Step 1: Add WallEdge enum and Wall struct after MapObject**

Add after line 39 (after the MapObject struct):

```rust
/// Wall edge direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallEdge {
    Down,
    Right,
}

/// Wall placed on a tile edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    /// Global tile ID for wall sprite
    pub gid: u32,
    /// World tile X coordinate
    pub tile_x: i32,
    /// World tile Y coordinate
    pub tile_y: i32,
    /// Which edge of the tile this wall is on
    pub edge: WallEdge,
}
```

**Step 2: Add walls field to Chunk struct**

Modify the Chunk struct (around line 147) to add a `walls` field after `objects`:

```rust
    /// Walls placed on tile edges
    pub walls: Vec<Wall>,
```

**Step 3: Initialize walls in Chunk::new()**

In `Chunk::new()` (around line 160), add `walls: Vec::new(),` after `objects: Vec::new(),`

**Step 4: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add rust-server/src/chunk.rs
git commit -m "feat(server): add Wall and WallEdge types to chunk module"
```

---

## Task 2: Add Wall Data to Server Protocol

**Files:**
- Modify: `rust-server/src/protocol.rs:316-331`

**Step 1: Add ChunkWallData struct after ChunkObjectData**

Add after line 331:

```rust
/// Wall data for chunk transmission
#[derive(Debug, Clone, Serialize)]
pub struct ChunkWallData {
    pub gid: u32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub edge: String, // "down" or "right"
}
```

**Step 2: Add walls field to ChunkData variant**

Find the `ChunkData` variant in `ServerMessage` (around line 234) and add `walls`:

```rust
    ChunkData {
        chunk_x: i32,
        chunk_y: i32,
        layers: Vec<ChunkLayerData>,
        collision: Vec<u8>,
        objects: Vec<ChunkObjectData>,
        walls: Vec<ChunkWallData>,  // Add this line
    },
```

**Step 3: Update encode_server_message for ChunkData**

Find the ChunkData encoding section (around line 1011) and add walls encoding after objects:

```rust
            // Encode walls
            let wall_values: Vec<Value> = walls
                .iter()
                .map(|w| {
                    let mut wmap = Vec::new();
                    wmap.push((
                        Value::String("gid".into()),
                        Value::Integer((w.gid as i64).into()),
                    ));
                    wmap.push((
                        Value::String("tileX".into()),
                        Value::Integer((w.tile_x as i64).into()),
                    ));
                    wmap.push((
                        Value::String("tileY".into()),
                        Value::Integer((w.tile_y as i64).into()),
                    ));
                    wmap.push((
                        Value::String("edge".into()),
                        Value::String(w.edge.clone().into()),
                    ));
                    Value::Map(wmap)
                })
                .collect();
            map.push((Value::String("walls".into()), Value::Array(wall_values)));
```

**Step 4: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add rust-server/src/protocol.rs
git commit -m "feat(server): add wall data to chunk protocol"
```

---

## Task 3: Parse Walls from JSON in World Loader

**Files:**
- Modify: `rust-server/src/world.rs:104-217`

**Step 1: Add Wall and WallEdge imports**

At the top of world.rs, add `Wall, WallEdge` to the chunk imports:

```rust
use crate::chunk::{Chunk, ChunkCoord, ChunkLayer, ChunkLayerType, EntitySpawn, MapObject, Wall, WallEdge, CHUNK_SIZE, local_to_world, world_to_local};
```

**Step 2: Parse walls in parse_simplified_json**

In `parse_simplified_json` (around line 105), add wall parsing after mapObjects parsing (before `Ok(chunk)`):

```rust
        // Parse walls
        if let Some(walls_array) = value["walls"].as_array() {
            for wall_value in walls_array {
                if let (Some(gid), Some(x), Some(y), Some(edge_str)) = (
                    wall_value["gid"].as_u64(),
                    wall_value["x"].as_i64(),
                    wall_value["y"].as_i64(),
                    wall_value["edge"].as_str(),
                ) {
                    let edge = match edge_str {
                        "down" => WallEdge::Down,
                        "right" => WallEdge::Right,
                        _ => continue,
                    };
                    chunk.walls.push(Wall {
                        gid: gid as u32,
                        tile_x: coord.x * CHUNK_SIZE as i32 + x as i32,
                        tile_y: coord.y * CHUNK_SIZE as i32 + y as i32,
                        edge,
                    });
                }
            }
        }
```

**Step 3: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add rust-server/src/world.rs
git commit -m "feat(server): parse walls from chunk JSON files"
```

---

## Task 4: Send Walls in Chunk Response

**Files:**
- Modify: `rust-server/src/game.rs` (find handle_request_chunk or similar)

**Step 1: Find where ChunkData is constructed**

Search for `ServerMessage::ChunkData` construction in the codebase.

**Step 2: Add walls to ChunkData response**

Where ChunkData is built, add walls conversion:

```rust
walls: chunk.walls.iter().map(|w| ChunkWallData {
    gid: w.gid,
    tile_x: w.tile_x,
    tile_y: w.tile_y,
    edge: match w.edge {
        WallEdge::Down => "down".to_string(),
        WallEdge::Right => "right".to_string(),
    },
}).collect(),
```

**Step 3: Verify compilation**

Run: `cd rust-server && cargo check`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add rust-server/src/game.rs
git commit -m "feat(server): include walls in chunk data response"
```

---

## Task 5: Add Wall Types to Client Chunk Module

**Files:**
- Modify: `client/src/game/chunk.rs:39-52`

**Step 1: Add WallEdge enum and Wall struct**

Add after MapObject struct (line 52):

```rust
/// Wall edge direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallEdge {
    Down,
    Right,
}

/// Wall placed on a tile edge
#[derive(Debug, Clone)]
pub struct Wall {
    /// Global tile ID for wall sprite
    pub gid: u32,
    /// World tile X coordinate
    pub tile_x: i32,
    /// World tile Y coordinate
    pub tile_y: i32,
    /// Which edge of the tile this wall is on
    pub edge: WallEdge,
}
```

**Step 2: Add walls field to client Chunk struct**

In the client Chunk struct (around line 99), add after `objects`:

```rust
    /// Walls placed on tile edges
    pub walls: Vec<Wall>,
```

**Step 3: Initialize walls in Chunk::new()**

In `Chunk::new()`, add `walls: Vec::new(),` after `objects: Vec::new(),`

**Step 4: Update ChunkManager::load_chunk to accept walls**

Modify the `load_chunk` function signature to accept walls:

```rust
pub fn load_chunk(
    &mut self,
    chunk_x: i32,
    chunk_y: i32,
    layers: Vec<(u8, Vec<u32>)>,
    collision: &[u8],
    objects: Vec<MapObject>,
    walls: Vec<Wall>,
)
```

And add `chunk.walls = walls;` in the function body.

**Step 5: Verify compilation**

Run: `cd client && cargo check`
Expected: Compilation succeeds (may have warnings about unused walls parameter in callers)

**Step 6: Commit**

```bash
git add client/src/game/chunk.rs
git commit -m "feat(client): add Wall and WallEdge types to chunk module"
```

---

## Task 6: Parse Walls from Server Message

**Files:**
- Modify: `client/src/network/protocol.rs` or wherever chunk messages are handled

**Step 1: Find where chunk data is parsed from server**

Search for where `chunkData` message is handled.

**Step 2: Parse walls array from message**

Add wall parsing logic:

```rust
let walls: Vec<Wall> = msg_data
    .get("walls")
    .and_then(|v| v.as_array())
    .map(|arr| {
        arr.iter()
            .filter_map(|w| {
                let gid = w.get("gid")?.as_u64()? as u32;
                let tile_x = w.get("tileX")?.as_i64()? as i32;
                let tile_y = w.get("tileY")?.as_i64()? as i32;
                let edge_str = w.get("edge")?.as_str()?;
                let edge = match edge_str {
                    "down" => WallEdge::Down,
                    "right" => WallEdge::Right,
                    _ => return None,
                };
                Some(Wall { gid, tile_x, tile_y, edge })
            })
            .collect()
    })
    .unwrap_or_default();
```

**Step 3: Pass walls to chunk manager**

Update the call to `chunk_manager.load_chunk()` to include the parsed walls.

**Step 4: Verify compilation**

Run: `cd client && cargo check`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add client/src/network/
git commit -m "feat(client): parse walls from server chunk data"
```

---

## Task 7: Add Wall Rendering to Client Renderer

**Files:**
- Modify: `client/src/render/renderer.rs`

**Step 1: Add render_wall method**

Add a new method after `render_map_object` (around line 2892):

```rust
/// Render a wall on a tile edge
fn render_wall(&self, wall: &Wall, camera: &Camera) {
    let zoom = camera.zoom;

    // Get the tile's center screen position
    let (tile_center_x, tile_center_y) = world_to_screen(
        wall.tile_x as f32 + 0.5,
        wall.tile_y as f32 + 0.5,
        camera
    );

    // Bottom vertex is at center + half tile height
    let bottom_vertex_x = tile_center_x;
    let bottom_vertex_y = tile_center_y + (TILE_HEIGHT / 2.0) * zoom;

    // Try to get the sprite for this gid
    if let Some(texture) = self.get_object_sprite(wall.gid) {
        let tex_width = texture.width();
        let tex_height = texture.height();

        let scaled_width = (tex_width * zoom).round();
        let scaled_height = (tex_height * zoom).round();

        let (draw_x, draw_y) = match wall.edge {
            WallEdge::Down => {
                // Bottom-right corner of sprite at bottom vertex
                (bottom_vertex_x - scaled_width, bottom_vertex_y - scaled_height)
            }
            WallEdge::Right => {
                // Bottom-left corner of sprite at bottom vertex
                (bottom_vertex_x, bottom_vertex_y - scaled_height)
            }
        };

        draw_texture_ex(
            texture,
            draw_x.round(),
            draw_y.round(),
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(scaled_width, scaled_height)),
                ..Default::default()
            },
        );
    }
}
```

**Step 2: Add Wall and WallEdge imports**

At the top of renderer.rs, add `Wall, WallEdge` to the imports from game module.

**Step 3: Call render_wall in the render loop**

Find where map objects are rendered (around line 724) and add wall rendering nearby:

```rust
// Render walls
for wall in &chunk.walls {
    self.render_wall(wall, &state.camera);
}
```

**Step 4: Verify compilation**

Run: `cd client && cargo check`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add client/src/render/renderer.rs
git commit -m "feat(client): add wall rendering with edge-aligned positioning"
```

---

## Task 8: Add Wall Types to Mapper TypeScript Types

**Files:**
- Modify: `mapper/src/types/index.ts`

**Step 1: Add WallEdge type and Wall interface**

Add after SimplifiedMapObject (around line 253):

```typescript
export type WallEdge = 'down' | 'right';

export interface Wall {
  id: string;
  gid: number;
  x: number; // Local tile X within chunk
  y: number; // Local tile Y within chunk
  edge: WallEdge;
}

export interface SimplifiedWall {
  gid: number;
  x: number;
  y: number;
  edge: WallEdge;
}
```

**Step 2: Add walls to Chunk interface**

In the Chunk interface (around line 122), add after `mapObjects`:

```typescript
  walls: Wall[];
```

**Step 3: Add walls to SimplifiedChunk interface**

In SimplifiedChunk (around line 223), add after `mapObjects`:

```typescript
  walls: SimplifiedWall[];
```

**Step 4: Commit**

```bash
git add mapper/src/types/index.ts
git commit -m "feat(mapper): add Wall and WallEdge types"
```

---

## Task 9: Update Mapper ChunkManager for Walls

**Files:**
- Modify: `mapper/src/core/ChunkManager.ts`

**Step 1: Update imports**

Add `Wall, SimplifiedWall` to the imports from types.

**Step 2: Initialize walls in parseSimplifiedFormat**

In `parseSimplifiedFormat` (around line 182), add wall parsing:

```typescript
walls: (data.walls || []).map((w, i) => ({
  id: `wall_${i}`,
  gid: w.gid,
  x: w.x,
  y: w.y,
  edge: w.edge,
})),
```

**Step 3: Initialize walls in parseTiledFormat**

In `parseTiledFormat` (around line 48), add `walls: [],` to the chunk initialization.

**Step 4: Export walls in exportChunkData**

In `exportChunkData` (around line 340), add walls export:

```typescript
const walls: SimplifiedWall[] = chunk.walls.map((w) => ({
  gid: w.gid,
  x: w.x,
  y: w.y,
  edge: w.edge,
}));
```

And add `walls,` to the returned object.

**Step 5: Initialize walls in createEmptyChunk**

In `createEmptyChunk` (around line 392), add `walls: [],` to the chunk.

**Step 6: Verify build**

Run: `cd mapper && npm run build`
Expected: Build succeeds

**Step 7: Commit**

```bash
git add mapper/src/core/ChunkManager.ts
git commit -m "feat(mapper): add wall support to ChunkManager"
```

---

## Task 10: Add Wall Rendering to Mapper IsometricRenderer

**Files:**
- Modify: `mapper/src/core/IsometricRenderer.ts`

**Step 1: Add renderWalls method**

Add after `renderMapObjects` method (around line 265):

```typescript
private renderWalls(chunk: Chunk, viewport: Viewport): void {
  if (!this.ctx) return;

  // Sort walls by depth (x + y)
  const sortedWalls = [...chunk.walls].sort((a, b) => {
    return (a.x + a.y) - (b.x + b.y);
  });

  for (const wall of sortedWalls) {
    const worldCoord = chunkLocalToWorld(chunk.coord, { lx: wall.x, ly: wall.y });
    const screen = worldToScreen(worldCoord, viewport);

    // Get the object definition from objectLoader using the gid
    const objDef = objectLoader.getObjectByGid(wall.gid);

    if (objDef?.image) {
      const scaledWidth = objDef.image.width * viewport.zoom;
      const scaledHeight = objDef.image.height * viewport.zoom;

      // Bottom vertex of tile
      const bottomVertexX = screen.sx;
      const bottomVertexY = screen.sy + TILE_HEIGHT * viewport.zoom;

      let drawX: number;
      let drawY: number;

      if (wall.edge === 'down') {
        // Bottom-right corner of sprite at bottom vertex
        drawX = bottomVertexX - scaledWidth;
        drawY = bottomVertexY - scaledHeight;
      } else {
        // Bottom-left corner of sprite at bottom vertex (right edge)
        drawX = bottomVertexX;
        drawY = bottomVertexY - scaledHeight;
      }

      this.ctx.drawImage(
        objDef.image,
        0,
        0,
        objDef.image.width,
        objDef.image.height,
        drawX,
        drawY,
        scaledWidth,
        scaledHeight
      );
    }
  }
}
```

**Step 2: Add Wall import**

Import `Wall` from types if not already imported.

**Step 3: Call renderWalls in render method**

In the `render` method (around line 72), add wall rendering after map objects:

```typescript
// Render walls
if (this.options.showMapObjects) {
  for (const chunk of sortedChunks) {
    this.renderWalls(chunk, viewport);
  }
}
```

**Step 4: Verify build**

Run: `cd mapper && npm run build`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add mapper/src/core/IsometricRenderer.ts
git commit -m "feat(mapper): add wall rendering with edge-aligned positioning"
```

---

## Task 11: Add Wall Tool to Mapper (UI Component)

**Files:**
- Create: `mapper/src/components/WallTool.tsx` (or add to existing tool component)

**Step 1: Add wall tool types**

In the Tool type definition, add:

```typescript
WallDown: 'wallDown',
WallRight: 'wallRight',
```

**Step 2: Create wall placement handler**

Add wall placement logic that:
- On click, adds a wall to the chunk's walls array at the clicked tile
- Uses the currently selected wall GID from the object palette
- Sets the edge based on which tool is active (down or right)
- Toggle behavior: clicking same tile+edge removes the wall

**Step 3: Add wall tool buttons to toolbar**

Add two buttons for "Down Wall" and "Right Wall" tools.

**Step 4: Verify build**

Run: `cd mapper && npm run build`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add mapper/src/
git commit -m "feat(mapper): add wall placement tools (down/right)"
```

---

## Task 12: Integration Test

**Step 1: Create a test wall in a map file**

Manually add a wall to an existing chunk JSON:

```json
"walls": [
  { "gid": 101, "x": 5, "y": 5, "edge": "down" },
  { "gid": 200, "x": 5, "y": 5, "edge": "right" }
]
```

**Step 2: Start the server**

Run: `cd rust-server && cargo run`

**Step 3: Start the client**

Run: `cd client && cargo run`

**Step 4: Verify walls render at tile edges**

Navigate to the tile with walls and verify:
- Down wall renders on the bottom-left edge
- Right wall renders on the bottom-right edge
- Both walls meet at the bottom vertex of the tile

**Step 5: Test in mapper**

Run: `cd mapper && npm run dev`

Verify:
- Walls load and display correctly
- Wall tools can place new walls
- Walls export correctly to JSON

**Step 6: Commit test data**

```bash
git add rust-server/maps/
git commit -m "test: add sample wall data for integration testing"
```

---

## Summary

| Task | Component | Description |
|------|-----------|-------------|
| 1 | Server | Add Wall/WallEdge types to chunk.rs |
| 2 | Server | Add wall data to protocol |
| 3 | Server | Parse walls from JSON |
| 4 | Server | Send walls in chunk response |
| 5 | Client | Add Wall/WallEdge types |
| 6 | Client | Parse walls from server |
| 7 | Client | Render walls at edges |
| 8 | Mapper | Add TypeScript types |
| 9 | Mapper | Update ChunkManager |
| 10 | Mapper | Add wall rendering |
| 11 | Mapper | Add wall tools |
| 12 | All | Integration test |
