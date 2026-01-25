# Interior Maps & Instance System - Phase 1 & 2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable players to enter interior maps via portals with public/private instance support.

**Architecture:** Interior maps are separate map files loaded by an InstanceManager. Portals teleport players between the overworld and instances. Public instances are shared globally, private instances are per-party and destroyed when empty.

**Tech Stack:** Rust (server + client), JSON map files, WebSocket messages

---

## Phase 1: Interior Map Loading & Portal Transitions

### Task 1: Interior Map File Structure (Server)

**Files:**
- Create: `rust-server/src/interior.rs`
- Modify: `rust-server/src/main.rs` (add module)

**Step 1: Create interior map data structures**

In `rust-server/src/interior.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of instance this interior creates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceType {
    Public,
    Private,
}

/// A spawn point inside an interior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPoint {
    pub x: f32,
    pub y: f32,
}

/// A portal/exit inside an interior that leads elsewhere
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteriorPortal {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub target_map: String,       // "world" for overworld, or another interior ID
    pub target_x: f32,            // World coordinates for overworld exits
    pub target_y: f32,
    pub target_spawn: Option<String>,  // Spawn point name for interior targets
}

/// Definition of an interior map (loaded from JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteriorMapDef {
    pub id: String,
    pub name: String,
    pub instance_type: InstanceType,
    pub size: InteriorSize,
    pub spawn_points: HashMap<String, SpawnPoint>,
    pub portals: Vec<InteriorPortal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteriorSize {
    pub width: u32,
    pub height: u32,
}

impl InteriorMapDef {
    /// Load an interior map definition from a JSON file
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read interior file {}: {}", path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse interior JSON {}: {}", path, e))
    }

    /// Get a spawn point by name
    pub fn get_spawn_point(&self, name: &str) -> Option<&SpawnPoint> {
        self.spawn_points.get(name)
    }

    /// Get portal at a given tile position
    pub fn get_portal_at(&self, x: i32, y: i32) -> Option<&InteriorPortal> {
        self.portals.iter().find(|p| {
            x >= p.x && x < p.x + p.width && y >= p.y && y < p.y + p.height
        })
    }
}
```

**Step 2: Add module to main.rs**

In `rust-server/src/main.rs`, add after other mod declarations (~line 5):

```rust
mod interior;
```

**Step 3: Run to verify it compiles**

Run: `cd rust-server && cargo build`
Expected: Compiles with no errors

**Step 4: Commit**

```bash
git add rust-server/src/interior.rs rust-server/src/main.rs
git commit -m "feat(server): add interior map data structures"
```

---

### Task 2: Interior Registry (Server)

**Files:**
- Create: `rust-server/src/interior_registry.rs`
- Modify: `rust-server/src/main.rs`
- Modify: `rust-server/src/interior.rs`

**Step 1: Create the interior registry**

In `rust-server/src/interior_registry.rs`:

```rust
use std::collections::HashMap;
use std::path::Path;
use crate::interior::InteriorMapDef;

/// Registry of all interior map definitions
pub struct InteriorRegistry {
    interiors: HashMap<String, InteriorMapDef>,
}

impl InteriorRegistry {
    /// Create a new registry by scanning the interiors directory
    pub fn load_from_directory(dir: &str) -> Result<Self, String> {
        let mut interiors = HashMap::new();
        let path = Path::new(dir);

        if !path.exists() {
            log::warn!("Interiors directory {} does not exist, creating empty registry", dir);
            return Ok(Self { interiors });
        }

        let entries = std::fs::read_dir(path)
            .map_err(|e| format!("Failed to read interiors directory: {}", e))?;

        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().map_or(false, |ext| ext == "json") {
                match InteriorMapDef::load_from_file(file_path.to_str().unwrap()) {
                    Ok(interior) => {
                        log::info!("Loaded interior map: {} ({})", interior.id, interior.name);
                        interiors.insert(interior.id.clone(), interior);
                    }
                    Err(e) => {
                        log::error!("Failed to load interior {:?}: {}", file_path, e);
                    }
                }
            }
        }

        log::info!("Loaded {} interior maps", interiors.len());
        Ok(Self { interiors })
    }

    /// Get an interior definition by ID
    pub fn get(&self, id: &str) -> Option<&InteriorMapDef> {
        self.interiors.get(id)
    }

    /// List all interior IDs
    pub fn list_ids(&self) -> Vec<&String> {
        self.interiors.keys().collect()
    }
}
```

**Step 2: Add module and load registry in main.rs**

In `rust-server/src/main.rs`, add module (~line 6):

```rust
mod interior_registry;
```

Add to AppState struct (~line 75):

```rust
pub interior_registry: Arc<InteriorRegistry>,
```

In `AppState::new()` (~line 150, before the Ok):

```rust
let interior_registry = Arc::new(
    InteriorRegistry::load_from_directory("maps/interiors")
        .expect("Failed to load interior registry")
);
```

And include it in the struct initialization.

**Step 3: Re-export from interior.rs**

Add to bottom of `rust-server/src/interior.rs`:

```rust
pub use crate::interior_registry::InteriorRegistry;
```

**Step 4: Run to verify it compiles**

Run: `cd rust-server && cargo build`
Expected: Compiles (may warn about unused code)

**Step 5: Commit**

```bash
git add rust-server/src/interior_registry.rs rust-server/src/interior.rs rust-server/src/main.rs
git commit -m "feat(server): add interior registry to load interior maps"
```

---

### Task 3: Create Test Interior Map

**Files:**
- Create: `rust-server/maps/interiors/test_house.json`

**Step 1: Create interiors directory and test map**

```bash
mkdir -p rust-server/maps/interiors
```

Create `rust-server/maps/interiors/test_house.json`:

```json
{
  "id": "test_house",
  "name": "Test House",
  "instance_type": "public",
  "size": { "width": 10, "height": 8 },
  "spawn_points": {
    "entrance": { "x": 5.0, "y": 6.5 }
  },
  "portals": [
    {
      "id": "exit_door",
      "x": 4,
      "y": 7,
      "width": 2,
      "height": 1,
      "target_map": "world",
      "target_x": 16.5,
      "target_y": 17.5,
      "target_spawn": null
    }
  ],
  "layers": {
    "ground": [],
    "objects": [],
    "overhead": []
  },
  "collision": "",
  "entities": []
}
```

**Step 2: Verify server loads it**

Run: `cd rust-server && cargo run`
Expected: Log shows "Loaded interior map: test_house (Test House)"

**Step 3: Commit**

```bash
git add rust-server/maps/interiors/test_house.json
git commit -m "feat: add test interior map"
```

---

### Task 4: Portal Object in Overworld (Server)

**Files:**
- Modify: `rust-server/src/chunk.rs`

**Step 1: Add Portal struct to chunk.rs**

Add after the `Wall` struct (~line 50):

```rust
/// A portal that teleports players to another map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portal {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub target_map: String,           // Interior map ID
    pub target_spawn: String,         // Spawn point name in target
}
```

**Step 2: Add portals field to Chunk struct**

In the Chunk struct (~line 175), add:

```rust
pub portals: Vec<Portal>,
```

**Step 3: Initialize portals in Chunk parsing**

In `parse_simplified_json` function, add after walls parsing:

```rust
let portals: Vec<Portal> = root
    .get("portals")
    .and_then(|v| serde_json::from_value(v.clone()).ok())
    .unwrap_or_default();
```

And include `portals` in the Chunk construction.

**Step 4: Add to protocol for chunk data**

In `rust-server/src/protocol.rs`, add ChunkPortalData struct after ChunkWallData:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPortalData {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub target_map: String,
    pub target_spawn: String,
}
```

Add to ServerMessage::ChunkData variant:

```rust
portals: Vec<ChunkPortalData>,
```

**Step 5: Update chunk data handler in game.rs**

In `handle_chunk_request` (~line 3653), add portal conversion:

```rust
let portals: Vec<ChunkPortalData> = chunk.portals.iter().map(|p| ChunkPortalData {
    id: p.id.clone(),
    x: p.x,
    y: p.y,
    width: p.width,
    height: p.height,
    target_map: p.target_map.clone(),
    target_spawn: p.target_spawn.clone(),
}).collect();
```

Include `portals` in the ChunkData message.

**Step 6: Build and verify**

Run: `cd rust-server && cargo build`
Expected: Compiles successfully

**Step 7: Commit**

```bash
git add rust-server/src/chunk.rs rust-server/src/protocol.rs rust-server/src/game.rs
git commit -m "feat(server): add portal support to chunks"
```

---

### Task 5: Portal Message Protocol (Server)

**Files:**
- Modify: `rust-server/src/protocol.rs`

**Step 1: Add EnterPortal client message**

In ClientMessage enum (~line 50):

```rust
EnterPortal {
    portal_id: String,
},
```

**Step 2: Add MapTransition server message**

In ServerMessage enum (~line 200):

```rust
MapTransition {
    map_type: String,        // "interior" or "world"
    map_id: String,          // Interior ID or "world_0"
    spawn_x: f32,
    spawn_y: f32,
    instance_id: String,     // Unique instance identifier
},
```

**Step 3: Build and verify**

Run: `cd rust-server && cargo build`
Expected: Compiles (warnings about unused variants OK)

**Step 4: Commit**

```bash
git add rust-server/src/protocol.rs
git commit -m "feat(server): add portal transition protocol messages"
```

---

### Task 6: Instance Manager (Server)

**Files:**
- Create: `rust-server/src/instance.rs`
- Modify: `rust-server/src/main.rs`

**Step 1: Create instance manager**

In `rust-server/src/instance.rs`:

```rust
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::interior::{InstanceType, InteriorMapDef};
use crate::game::GameRoom;

/// Tracks an active instance
pub struct Instance {
    pub id: String,
    pub map_id: String,
    pub instance_type: InstanceType,
    pub players: RwLock<HashSet<String>>,
    pub room: Arc<GameRoom>,
}

impl Instance {
    pub async fn player_count(&self) -> usize {
        self.players.read().await.len()
    }

    pub async fn add_player(&self, player_id: &str) {
        self.players.write().await.insert(player_id.to_string());
    }

    pub async fn remove_player(&self, player_id: &str) -> usize {
        let mut players = self.players.write().await;
        players.remove(player_id);
        players.len()
    }
}

/// Manages all active instances
pub struct InstanceManager {
    /// Public instances: one per map_id
    public_instances: DashMap<String, Arc<Instance>>,
    /// Private instances: keyed by (owner_player_id, map_id)
    private_instances: DashMap<(String, String), Arc<Instance>>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            public_instances: DashMap::new(),
            private_instances: DashMap::new(),
        }
    }

    /// Get or create a public instance for a map
    pub async fn get_or_create_public(
        &self,
        map_def: &InteriorMapDef,
        room_factory: impl FnOnce(&str) -> Arc<GameRoom>,
    ) -> Arc<Instance> {
        // Check if exists
        if let Some(instance) = self.public_instances.get(&map_def.id) {
            return instance.clone();
        }

        // Create new
        let instance_id = format!("pub_{}", map_def.id);
        let room = room_factory(&instance_id);
        let instance = Arc::new(Instance {
            id: instance_id.clone(),
            map_id: map_def.id.clone(),
            instance_type: InstanceType::Public,
            players: RwLock::new(HashSet::new()),
            room,
        });

        self.public_instances.insert(map_def.id.clone(), instance.clone());
        log::info!("Created public instance: {}", instance_id);
        instance
    }

    /// Get or create a private instance for a player/party
    pub async fn get_or_create_private(
        &self,
        map_def: &InteriorMapDef,
        owner_id: &str,
        room_factory: impl FnOnce(&str) -> Arc<GameRoom>,
    ) -> Arc<Instance> {
        let key = (owner_id.to_string(), map_def.id.clone());

        // Check if exists
        if let Some(instance) = self.private_instances.get(&key) {
            return instance.clone();
        }

        // Create new
        let instance_id = format!("priv_{}_{}", map_def.id, Uuid::new_v4());
        let room = room_factory(&instance_id);
        let instance = Arc::new(Instance {
            id: instance_id.clone(),
            map_id: map_def.id.clone(),
            instance_type: InstanceType::Private,
            players: RwLock::new(HashSet::new()),
            room,
        });

        self.private_instances.insert(key, instance.clone());
        log::info!("Created private instance: {} for owner {}", instance_id, owner_id);
        instance
    }

    /// Remove a private instance (called when empty)
    pub fn remove_private(&self, owner_id: &str, map_id: &str) {
        let key = (owner_id.to_string(), map_id.to_string());
        if self.private_instances.remove(&key).is_some() {
            log::info!("Removed private instance for {} / {}", owner_id, map_id);
        }
    }

    /// Find which instance a player is in
    pub async fn find_player_instance(&self, player_id: &str) -> Option<Arc<Instance>> {
        // Check public instances
        for entry in self.public_instances.iter() {
            if entry.value().players.read().await.contains(player_id) {
                return Some(entry.value().clone());
            }
        }
        // Check private instances
        for entry in self.private_instances.iter() {
            if entry.value().players.read().await.contains(player_id) {
                return Some(entry.value().clone());
            }
        }
        None
    }
}
```

**Step 2: Add module to main.rs**

```rust
mod instance;
```

Add to AppState:

```rust
pub instance_manager: Arc<InstanceManager>,
```

Initialize in AppState::new():

```rust
let instance_manager = Arc::new(InstanceManager::new());
```

**Step 3: Build and verify**

Run: `cd rust-server && cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add rust-server/src/instance.rs rust-server/src/main.rs
git commit -m "feat(server): add instance manager for interior maps"
```

---

### Task 7: Portal Entry Handler (Server)

**Files:**
- Modify: `rust-server/src/game.rs`
- Modify: `rust-server/src/main.rs`

**Step 1: Add portal entry handler to GameRoom**

In `rust-server/src/game.rs`, add new method to GameRoom impl (~after handle_chunk_request):

```rust
/// Find a portal at the player's current position
pub async fn find_portal_at_player(&self, player_id: &str) -> Option<crate::chunk::Portal> {
    let players = self.players.read().await;
    let player = players.get(player_id)?;
    let coord = ChunkCoord::from_world(player.x, player.y);

    let chunk = self.world.get_or_load_chunk(coord)?;
    chunk.portals.iter().find(|p| {
        player.x >= p.x && player.x < p.x + p.width &&
        player.y >= p.y && player.y < p.y + p.height
    }).cloned()
}
```

**Step 2: Add message handler dispatch in main.rs**

In `handle_client_message` function, add case for EnterPortal:

```rust
ClientMessage::EnterPortal { portal_id } => {
    handle_enter_portal(state, room, player_id, &portal_id).await;
}
```

**Step 3: Implement handle_enter_portal in main.rs**

Add after handle_client_message function:

```rust
async fn handle_enter_portal(
    state: &AppState,
    room: &GameRoom,
    player_id: &str,
    portal_id: &str,
) {
    // Find portal at player position
    let portal = match room.find_portal_at_player(player_id).await {
        Some(p) if p.id == portal_id => p,
        _ => {
            log::warn!("Player {} tried to use portal {} but not standing on it", player_id, portal_id);
            return;
        }
    };

    // Get interior definition
    let interior = match state.interior_registry.get(&portal.target_map) {
        Some(i) => i,
        None => {
            log::error!("Portal {} references unknown interior {}", portal_id, portal.target_map);
            return;
        }
    };

    // Get spawn point
    let spawn = match interior.get_spawn_point(&portal.target_spawn) {
        Some(s) => s,
        None => {
            log::error!("Interior {} has no spawn point {}", interior.id, portal.target_spawn);
            return;
        }
    };

    // TODO: Create/get instance and transition player
    // For now, just log
    log::info!(
        "Player {} entering portal {} -> {} at ({}, {})",
        player_id, portal_id, interior.id, spawn.x, spawn.y
    );

    // Send transition message to client
    room.send_to_player(
        player_id,
        ServerMessage::MapTransition {
            map_type: "interior".to_string(),
            map_id: interior.id.clone(),
            spawn_x: spawn.x,
            spawn_y: spawn.y,
            instance_id: format!("pub_{}", interior.id), // Placeholder
        },
    ).await;
}
```

**Step 4: Build and verify**

Run: `cd rust-server && cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add rust-server/src/game.rs rust-server/src/main.rs
git commit -m "feat(server): add portal entry handler"
```

---

### Task 8: Client Portal Data Structures

**Files:**
- Modify: `client/src/game/chunk.rs`
- Modify: `client/src/network/client.rs`

**Step 1: Add Portal struct to client chunk.rs**

After the Wall struct:

```rust
/// A portal that teleports players to another map
#[derive(Debug, Clone)]
pub struct Portal {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub target_map: String,
    pub target_spawn: String,
}
```

**Step 2: Add portals field to Chunk struct**

```rust
pub portals: Vec<Portal>,
```

**Step 3: Parse portals in network client**

In `client/src/network/client.rs`, in the ChunkData handler, add portal parsing:

```rust
let portals: Vec<Portal> = value
    .get("portals")
    .and_then(|v| v.as_array())
    .map(|arr| {
        arr.iter().filter_map(|p| {
            Some(Portal {
                id: p.get("id")?.as_str()?.to_string(),
                x: p.get("x")?.as_i64()? as i32,
                y: p.get("y")?.as_i64()? as i32,
                width: p.get("width")?.as_i64()? as i32,
                height: p.get("height")?.as_i64()? as i32,
                target_map: p.get("target_map")?.as_str()?.to_string(),
                target_spawn: p.get("target_spawn")?.as_str()?.to_string(),
            })
        }).collect()
    })
    .unwrap_or_default();
```

Include in Chunk construction.

**Step 4: Build and verify**

Run: `cd client && cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add client/src/game/chunk.rs client/src/network/client.rs
git commit -m "feat(client): add portal data structures"
```

---

### Task 9: Client Map Transition State

**Files:**
- Modify: `client/src/game/state.rs`

**Step 1: Add transition state enum and struct**

Add near top of file after imports:

```rust
/// State of a map transition (fade effect)
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionState {
    None,
    FadingOut,
    Loading,
    FadingIn,
}

/// Tracks an in-progress map transition
#[derive(Debug, Clone)]
pub struct MapTransition {
    pub state: TransitionState,
    pub progress: f32,
    pub target_map_type: String,
    pub target_map_id: String,
    pub target_spawn_x: f32,
    pub target_spawn_y: f32,
    pub instance_id: String,
}

impl Default for MapTransition {
    fn default() -> Self {
        Self {
            state: TransitionState::None,
            progress: 0.0,
            target_map_type: String::new(),
            target_map_id: String::new(),
            target_spawn_x: 0.0,
            target_spawn_y: 0.0,
            instance_id: String::new(),
        }
    }
}
```

**Step 2: Add to GameState struct**

```rust
pub map_transition: MapTransition,
```

**Step 3: Initialize in GameState::new()**

```rust
map_transition: MapTransition::default(),
```

**Step 4: Build and verify**

Run: `cd client && cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add client/src/game/state.rs
git commit -m "feat(client): add map transition state"
```

---

### Task 10: Client Transition Rendering & Input

**Files:**
- Modify: `client/src/render/renderer.rs`
- Modify: `client/src/game/state.rs`
- Modify: `client/src/main.rs`

**Step 1: Add transition update logic to GameState**

In `client/src/game/state.rs`, add method to GameState impl:

```rust
/// Update map transition animation
pub fn update_transition(&mut self, delta: f32) {
    const FADE_DURATION: f32 = 0.25;

    match self.map_transition.state {
        TransitionState::FadingOut => {
            self.map_transition.progress += delta / FADE_DURATION;
            if self.map_transition.progress >= 1.0 {
                self.map_transition.progress = 1.0;
                self.map_transition.state = TransitionState::Loading;
            }
        }
        TransitionState::FadingIn => {
            self.map_transition.progress -= delta / FADE_DURATION;
            if self.map_transition.progress <= 0.0 {
                self.map_transition.progress = 0.0;
                self.map_transition.state = TransitionState::None;
            }
        }
        _ => {}
    }
}

/// Start a map transition
pub fn start_transition(&mut self, map_type: String, map_id: String, spawn_x: f32, spawn_y: f32, instance_id: String) {
    self.map_transition = MapTransition {
        state: TransitionState::FadingOut,
        progress: 0.0,
        target_map_type: map_type,
        target_map_id: map_id,
        target_spawn_x: spawn_x,
        target_spawn_y: spawn_y,
        instance_id,
    };
}

/// Check if input should be blocked due to transition
pub fn is_transitioning(&self) -> bool {
    self.map_transition.state != TransitionState::None
}
```

**Step 2: Add fade overlay rendering**

In `client/src/render/renderer.rs`, add method to Renderer impl:

```rust
/// Render transition fade overlay
pub fn render_transition_overlay(&self, state: &GameState) {
    use macroquad::prelude::*;

    if state.map_transition.state == crate::game::state::TransitionState::None {
        return;
    }

    let alpha = state.map_transition.progress;
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, alpha),
    );
}
```

**Step 3: Call transition update in main loop**

In `client/src/main.rs`, in the main game loop, add after other state updates:

```rust
game_state.update_transition(delta);
```

**Step 4: Call overlay render after UI**

After UI rendering:

```rust
renderer.render_transition_overlay(&game_state);
```

**Step 5: Build and verify**

Run: `cd client && cargo build`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add client/src/render/renderer.rs client/src/game/state.rs client/src/main.rs
git commit -m "feat(client): add map transition fade effect"
```

---

### Task 11: Client Portal Interaction

**Files:**
- Modify: `client/src/input/handler.rs`
- Modify: `client/src/network/client.rs`
- Modify: `client/src/network/messages.rs`

**Step 1: Add EnterPortal message**

In `client/src/network/messages.rs`, add to ClientMessage enum (or equivalent):

```rust
pub fn enter_portal(portal_id: &str) -> String {
    serde_json::json!({
        "type": "enterPortal",
        "portal_id": portal_id
    }).to_string()
}
```

**Step 2: Add portal check helper to ChunkManager**

In `client/src/game/chunk.rs`, add to ChunkManager impl:

```rust
/// Find a portal at the given world position
pub fn get_portal_at(&self, x: f32, y: f32) -> Option<&Portal> {
    let coord = ChunkCoord::from_world(x, y);
    let chunk = self.chunks.get(&coord)?;
    let tile_x = x.floor() as i32;
    let tile_y = y.floor() as i32;

    chunk.portals.iter().find(|p| {
        tile_x >= p.x && tile_x < p.x + p.width &&
        tile_y >= p.y && tile_y < p.y + p.height
    })
}
```

**Step 3: Add portal interaction on 'E' key or click**

In input handler, when player presses interact key at a portal location:

```rust
// Check for portal at player position
if let Some(portal) = state.chunk_manager.get_portal_at(player.x, player.y) {
    // Send enter portal message
    send_message(messages::enter_portal(&portal.id));
}
```

**Step 4: Handle MapTransition message**

In `client/src/network/client.rs`, add handler for mapTransition:

```rust
"mapTransition" => {
    if let Some(value) = data {
        let map_type = extract_string(value, "map_type").unwrap_or_default();
        let map_id = extract_string(value, "map_id").unwrap_or_default();
        let spawn_x = extract_f32(value, "spawn_x").unwrap_or(0.0);
        let spawn_y = extract_f32(value, "spawn_y").unwrap_or(0.0);
        let instance_id = extract_string(value, "instance_id").unwrap_or_default();

        state.start_transition(map_type, map_id, spawn_x, spawn_y, instance_id);
    }
}
```

**Step 5: Build and verify**

Run: `cd client && cargo build`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add client/src/input/handler.rs client/src/network/client.rs client/src/network/messages.rs client/src/game/chunk.rs
git commit -m "feat(client): add portal interaction and transition handling"
```

---

## Phase 2: Instance System

### Task 12: Full Instance Lifecycle (Server)

**Files:**
- Modify: `rust-server/src/instance.rs`
- Modify: `rust-server/src/main.rs`
- Modify: `rust-server/src/game.rs`

**Step 1: Add player tracking to instance transitions**

Update handle_enter_portal to properly:
1. Remove player from current room/instance
2. Add player to new instance
3. Track instance ownership for private instances
4. Clean up empty private instances

**Step 2: Add party support for private instances**

Check if player is in a party, use party leader's instance if exists.

**Step 3: Add instance cleanup on player leave**

When player disconnects or leaves instance, check if private instance is empty and clean up.

(Detailed implementation steps would follow the same pattern as above)

---

### Task 13: Interior Map Chunk Loading (Server)

**Files:**
- Modify: `rust-server/src/world.rs`
- Modify: `rust-server/src/game.rs`

**Step 1: Support loading interior chunks**

Interior maps use a simplified single-chunk format. Modify World to support loading from interior map definitions.

**Step 2: Create GameRoom for interior instances**

Factory function to create a GameRoom initialized with interior map data.

---

### Task 14: Client Interior Map Loading

**Files:**
- Modify: `client/src/game/chunk.rs`
- Modify: `client/src/game/state.rs`
- Modify: `client/src/network/client.rs`

**Step 1: Support receiving interior map data**

Handle a new message type for full interior map data (not chunked).

**Step 2: Switch chunk manager mode**

Interior maps use a single "chunk" - switch ChunkManager to interior mode.

**Step 3: Complete transition on map load**

When interior data is received during Loading state, spawn player and fade in.

---

## Testing Checklist

After implementation:

1. [ ] Place a portal in overworld using map editor
2. [ ] Walk player onto portal tile
3. [ ] Press interact key (E)
4. [ ] Verify fade out animation plays
5. [ ] Verify player appears in interior at spawn point
6. [ ] Verify fade in animation plays
7. [ ] Walk to exit portal in interior
8. [ ] Verify return to overworld at correct position
9. [ ] Test with multiple players for public instance (both see each other)
10. [ ] Test private instance (only party members can join)
11. [ ] Test private instance cleanup (leave and verify instance destroyed)

---

## File Reference

### Server Files
- `rust-server/src/interior.rs` - Interior map data structures
- `rust-server/src/interior_registry.rs` - Registry loading
- `rust-server/src/instance.rs` - Instance manager
- `rust-server/src/chunk.rs` - Portal struct
- `rust-server/src/protocol.rs` - Network messages
- `rust-server/src/game.rs` - Portal handler
- `rust-server/src/main.rs` - Integration

### Client Files
- `client/src/game/chunk.rs` - Portal struct, ChunkManager
- `client/src/game/state.rs` - MapTransition
- `client/src/network/client.rs` - Message handlers
- `client/src/network/messages.rs` - Outgoing messages
- `client/src/render/renderer.rs` - Fade overlay
- `client/src/input/handler.rs` - Portal interaction
