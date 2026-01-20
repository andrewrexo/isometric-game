# Mapper: Custom Isometric Map Editor

## Overview

A bespoke web-based map editor for the isometric game, replacing Tiled with a tailored solution that understands our chunk system, entity registry, and export format natively.

## Tech Stack

- **Framework**: React + TypeScript
- **Build**: Vite
- **Rendering**: Canvas 2D
- **State**: Zustand or React Context

## Architecture

```
mapper/
├── src/
│   ├── components/
│   │   ├── Canvas/           # Isometric viewport
│   │   ├── TilePalette/      # Tile selection
│   │   ├── EntityPanel/      # NPC placement
│   │   ├── LayerPanel/       # Layer visibility
│   │   ├── PropertiesPanel/  # Entity/selection properties
│   │   └── Toolbar/          # Tool buttons
│   ├── core/
│   │   ├── ChunkManager.ts   # Load, track, export chunks
│   │   ├── TilesetLoader.ts  # PNG sprite sheet loading
│   │   ├── IsometricRenderer.ts
│   │   ├── EntityRegistry.ts # Parse NPC TOML definitions
│   │   └── History.ts        # Undo/redo stack
│   ├── state/
│   │   └── store.ts          # Global editor state
│   ├── types/
│   │   └── index.ts          # TypeScript interfaces
│   └── App.tsx
├── public/
│   └── assets/               # Symlink to game assets
├── mapper-config.json        # Tileset definitions
└── vite.config.ts
```

## Data Model

### World & Chunks

```typescript
interface World {
  chunks: Map<string, Chunk>;  // key: "x,y"
  bounds: { minX: number; maxX: number; minY: number; maxY: number };
}

interface Chunk {
  coord: { x: number; y: number };
  layers: {
    ground: Uint32Array;    // 1024 tile IDs
    objects: Uint32Array;
    overhead: Uint32Array;
  };
  collision: BitSet;          // 1024 bits
  entitySpawns: EntitySpawn[];
  mapObjects: MapObject[];
  dirty: boolean;
}

interface EntitySpawn {
  entityId: string;
  x: number;                  // local 0-31
  y: number;
  level: number;
  uniqueId?: string;
  facing?: "north" | "south" | "east" | "west";
}

interface MapObject {
  gid: number;
  x: number;
  y: number;
  width: number;
  height: number;
}
```

### Coordinate System

- **World coords**: Continuous, can be negative
- **Chunk coords**: Which chunk the tile belongs to
- **Local coords**: 0-31 within a chunk

Conversion: `worldX = chunkX * 32 + localX`

## Tileset Configuration

```json
{
  "tilesets": [
    {
      "id": "terrain",
      "path": "assets/tiles_eo.png",
      "tileWidth": 64,
      "tileHeight": 32,
      "columns": 16,
      "firstGid": 1
    },
    {
      "id": "objects",
      "path": "assets/objects.png",
      "tileWidth": 64,
      "tileHeight": 32,
      "columns": 16,
      "firstGid": 1249
    }
  ]
}
```

## Isometric Rendering

Screen position from world coords:
```
screenX = (worldX - worldY) * (tileWidth / 2) + offsetX
screenY = (worldX + worldY) * (tileHeight / 2) + offsetY
```

Draw order: back-to-front, top-to-bottom for correct depth sorting.

## Chunk Visibility

- Grid overlay showing 32-tile chunk boundaries
- Chunk coordinate labels in corners
- Status bar showing current chunk under cursor
- Optional alternating tint for chunk clarity

## Editing Tools

| Tool | Key | Description |
|------|-----|-------------|
| Select | V | Select tiles/entities, marquee selection |
| Paint | B | Paint selected tile |
| Fill | G | Flood-fill contiguous area |
| Eraser | E | Clear tiles (set to 0) |
| Collision | C | Toggle walkable/blocked |
| Entity | N | Place entity spawns |
| Eyedropper | I | Pick tile from canvas |

### Layers

- Ground
- Objects
- Overhead
- Collision (overlay)
- Entities (overlay)

### Stamps/Prefabs

1. Marquee select region
2. Ctrl+C or "Save as Stamp"
3. Stamps saved to `mapper/stamps/*.json`
4. Select stamp → paint to place

## Entity Placement

- Load NPC definitions from `rust-server/data/entities/npcs/*.toml`
- Searchable dropdown grouped by type (Hostile, Quest Giver, Merchant)
- Click to place, click placed entity to edit properties
- Property inspector for level, uniqueId, facing

## Export Format

Simplified JSON per chunk:

```json
{
  "version": 1,
  "coord": { "x": 0, "y": 0 },
  "size": 32,
  "layers": {
    "ground": [1, 1, 2, ...],
    "objects": [0, 0, 1450, ...],
    "overhead": [0, 0, 0, ...]
  },
  "collision": "Base64EncodedBitset...",
  "entitySpawns": [
    {
      "entityId": "elder_villager",
      "x": 16,
      "y": 14,
      "level": 1,
      "uniqueId": "elder_main",
      "facing": "south"
    }
  ],
  "mapObjects": [
    { "gid": 1448, "x": 5, "y": 12, "width": 80, "height": 138 }
  ]
}
```

## UI Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  File  Edit  View  Tools                          [Chunk: (0, 0)]   │
├────────┬───────────────────────────────────────────────┬────────────┤
│ Tools  │                                               │  Layers    │
│  [V]   │                                               │  ☑ Ground  │
│  [B]   │         Isometric Canvas                      │  ☑ Objects │
│  [G]   │                                               │  ☑ Overhead│
│  [E]   │                                               │  ☑ Collision│
│  [C]   │                                               │  ☑ Entities│
│  [N]   │                                               ├────────────┤
│  [I]   │                                               │  Entities  │
├────────┴───────────────────────────────────────────────┤  [Search]  │
│  Tile Palette                              [terrain ▼] │  • pig     │
│  [1][2][3][4][5][6][7][8][9][10][11][12]...           │  • wolf    │
└────────────────────────────────────────────────────────┴────────────┘
```

## Keyboard Shortcuts

- `1-3`: Switch active layer
- `Space+drag`: Pan viewport
- `Scroll`: Zoom (0.25x - 4x)
- `Ctrl+S`: Save dirty chunks
- `Ctrl+Z`: Undo
- `Ctrl+Shift+Z`: Redo
- `Alt+click`: Eyedropper (any tool)
- `Home`: Recenter view

## Server-Side Changes

Update `rust-server/src/chunk.rs` to parse new format:
- Add version detection
- Parse simplified JSON structure
- Base64 decode collision bitset
- Local coords for entity spawns (no pixel conversion needed)
