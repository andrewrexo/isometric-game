# Interior Maps & Instance System Design

## Overview

Add support for interior maps (houses, dungeons, shops) that players can enter via portals. Interiors exist as separate map files with an instance system supporting public, private, and player-owned spaces.

## Instance Types

| Type | Persistence | Access | Use Case |
|------|-------------|--------|----------|
| **Public** | Forever | Everyone | Shops, taverns, shared spaces |
| **Private** | Until empty | Party members | Dungeons, quest instances |
| **Owned** | Forever (DB) | Owner + invited | Player housing |

## Map Structure & Storage

Interior maps stored separately from overworld:

```
maps/
  world_0/           # Overworld chunks
    chunk_0_0.json
    ...
  interiors/         # Interior maps
    blacksmith_shop.json
    tavern.json
    goblin_cave.json
    ...
```

### Interior Map Format

```json
{
  "id": "blacksmith_shop",
  "name": "Blacksmith's Workshop",
  "instance_type": "public",
  "size": { "width": 16, "height": 12 },
  "spawn_points": {
    "entrance": { "x": 8, "y": 11 },
    "back_door": { "x": 2, "y": 1 }
  },
  "layers": {
    "ground": [...],
    "objects": [...],
    "overhead": [...]
  },
  "collision": "base64_encoded_bits",
  "entities": [...],
  "portals": [
    {
      "id": "exit_main",
      "x": 8,
      "y": 11,
      "target_map": "world",
      "target_x": 45,
      "target_y": 32
    }
  ]
}
```

Key properties:
- `id` - Unique identifier for linking
- `instance_type` - "public", "private", or "owned"
- `spawn_points` - Named locations where players appear
- `portals` - Exit points linking back to overworld or other maps

## Portal System

### Overworld Portals

Map objects placed in the overworld with properties:
- `type`: `"portal"`
- `target_map`: Interior map ID (e.g., `"blacksmith_shop"`)
- `target_spawn`: Spawn point name (e.g., `"entrance"`)

### Interior Exit Portals

Defined in the interior map's `portals` array:
- Link to specific world coordinates
- Can exit to different locations than entrance (dungeon with separate in/out)

### Portal Flow

```
Player interacts with portal
  → Server validates access
  → Server checks instance type:
      Public: join/create single global instance
      Private: join party leader's instance or create new
      Owned: load from database or deny if not permitted
  → Server removes player from current map
  → Server adds player to instance at spawn point
  → Client receives MapTransition message
  → Client fades out (250ms) → loads map → fades in (250ms)
```

## Instance Management

### Server Data Structures

```rust
struct InstanceManager {
    // Public: one per map ID
    public_instances: HashMap<String, Instance>,

    // Private: keyed by owner + map ID
    private_instances: HashMap<(String, String), Instance>,

    // Owned: keyed by owner + map ID, persisted to DB
    owned_instances: HashMap<(String, String), Instance>,
}

struct Instance {
    map_id: String,
    players: HashSet<String>,
    npcs: HashMap<String, Npc>,
    ground_items: HashMap<String, GroundItem>,
    state: InstanceState,
}
```

### Public Instance Behavior

- Created on first player entry
- Persists indefinitely
- All players share same NPCs, items, state
- NPCs respawn normally

### Private Instance Behavior

- Created when party leader enters (or solo player)
- Party members can join via owner's ID
- Destroyed immediately when last player leaves
- Re-entering creates fresh instance

### Owned Instance Behavior

- Created when player purchases/is granted a house
- Tied to player ID in database
- State persisted: placed objects, storage contents, customizations
- Owner can enter anytime
- Guests need invite or permission setting
- Never destroyed

### Party Joining Logic

```
Player enters private instance portal:
  If player in party AND party leader has active instance:
    Join leader's instance
  Else:
    Create new instance (player becomes owner)
```

## Player Housing Persistence

```json
{
  "player_id": "abc123",
  "house_map": "small_cottage",
  "placed_objects": [
    { "item_id": "wooden_table", "x": 5, "y": 3 },
    { "item_id": "chest", "x": 2, "y": 2, "contents": [...] }
  ],
  "permissions": "private"
}
```

Interior map defines the layout/shell. Player data stores customizations on top.

## Client Transition

```rust
struct MapTransition {
    state: TransitionState,  // FadingOut, Loading, FadingIn, None
    progress: f32,           // 0.0 to 1.0
    target_map: Option<String>,
    target_spawn: Option<(f32, f32)>,
}

const FADE_OUT_DURATION: f32 = 0.25;  // 250ms
const FADE_IN_DURATION: f32 = 0.25;   // 250ms
```

Total transition time: ~500ms (snappy but smooth)

## Network Messages

### Client → Server

- `EnterPortal { portal_id }` - Request to enter a portal

### Server → Client

- `MapTransition { map_id, spawn_x, spawn_y, map_data }` - New map data and position
- `InstancePlayerJoined { player_id }` - Another player entered the instance
- `InstancePlayerLeft { player_id }` - Another player left the instance

## Map Editor Integration

### Portal Object Properties

- `target_map`: Dropdown of available interior maps
- `target_spawn`: Dropdown of spawn points in selected map
- **"Open Target Map"** button: Opens linked interior for editing

### Interior Map Management

- List view of all interiors in `maps/interiors/`
- Each interior shows its portals and exit destinations
- **"Find Entrances"** button: Shows all overworld portals linking to this interior
- Instance type selector: Public / Private / Owned

## Implementation Components

### Server

- `InstanceManager` - Tracks all active instances
- `Instance` struct - Per-instance state
- `InteriorMap` loader - Loads from `maps/interiors/`
- Housing persistence - Saves owned instance state to database
- Portal interaction handler - Validates and processes transitions

### Client

- `MapTransition` - Fade effect and map switching
- Portal interaction - Sends `EnterPortal` on door click
- Interior rendering - Same renderer, different map data source
