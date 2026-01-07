# Isometric Game Architecture

This repo is split into a Rust game server (`rust-server/`) and a Macroquad client (`client/`). Both sides share a lightweight MessagePack protocol (Colyseus-compatible “ROOM_DATA” frames) and stay in lockstep on tick rate (20 Hz).

## Server (rust-server/)
- **Entrypoint:** `rust-server/src/main.rs` with `#[tokio::main]` to boot the async runtime. Builds an Axum router with:
  - `POST /api/register | /api/login | /api/logout` for auth (SQLite + Argon2 hashes via `db.rs`).
  - `POST /matchmake/joinOrCreate/:room` to create/fetch rooms and pre‑reserve a player session.
  - `GET /:room_id` upgrades to WebSocket and hands off to `handle_socket`.
- **State:** `AppState` holds `Arc<DashMap<...>>` for rooms, sessions, and auth tokens plus an `Arc<Database>`. `Arc` gives shared ownership across tasks; `DashMap` is a concurrent hashmap. Per-room data lives inside `GameRoom` behind `tokio::RwLock` to allow many readers / single writer.
- **Game loop:** Two background tasks:
  - Tick loop every 50 ms calls `GameRoom::tick` for movement, NPC AI, respawns, item expiry, and broadcasts a `StateSync`.
  - Auto-save every 30 s iterates active sessions, pulls save snapshots, and persists to SQLite.
- **WebSocket flow (`handle_socket`):**
  - Validates the session, sends `Welcome`, then replays currently active players to the new client.
  - Subscribes to a `broadcast::Sender<ServerMessage>` for room-wide events and spawns:
    - A send task (listens to room broadcasts + direct mpsc channel).
    - A recv task (decodes client MessagePack bytes with `protocol::decode_client_message` and dispatches to handlers).
  - On disconnect, saves player data, removes them from the room, and broadcasts `PlayerLeft`.
- **Gameplay systems (`game.rs`, `npc.rs`, `item.rs`, `tilemap.rs`):**
  - `GameRoom` tracks `players`, `npcs`, and `ground_items` (all `RwLock<HashMap<...>>`).
  - Movement is grid-based with a 250 ms cooldown (`MOVE_COOLDOWN_MS`). Direction is stored as an `enum Direction` with `#[repr(u8)]` for compact network encoding.
  - Combat enforces a 1 s attack cooldown, finds a target on the tile in front of the attacker, applies damage, and triggers drops/EXP/level‑up messages.
  - NPCs use simple state (Idle/Chasing/Attacking/Returning/Dead) and tile-by-tile AI with per-type stats (`NpcType::stats`). Respawn timers are handled in `tick`.
  - Items: `GroundItem` includes an owner-only pickup window and 60 s despawn timer; `Inventory` is 20 slots with stack limits. Drops are deterministic-ish off a time-based seed.
  - Tilemap collision: `Tilemap::new_test_map` mirrors the client generation—edges are blocked and some procedural rocks. `is_tile_walkable` is used for move validation.
- **Protocol (`protocol.rs`):**
  - Client messages are `#[serde(tag = "type")]` enums like `move`, `attack`, `pickup`, `useItem`, decoded from MessagePack arrays `[13, "type", {data}]`.
  - Server messages (`ServerMessage`) cover joins/leaves, state sync, chat, damage, deaths/respawns, EXP/level up, item lifecycle, inventory updates, and errors. `encode_server_message` builds the `[13, "type", map]` frame manually with `rmpv`.
- **Persistence (`db.rs`):** `sqlx` with a SQLite pool. `Database::new` runs migrations (creates `players` table and backfills columns). Passwords are hashed with Argon2 (`argon2` crate). Player saves serialize inventory slots as `(slot_idx, item_type_u8, quantity)` JSON.

### Rust-specific notes (server)
- `#[tokio::main]` macro generates an async main that spins up the runtime.
- `Arc` (atomically ref-counted) + `DashMap`/`RwLock` allow multi-task shared state without `unsafe`.
- `tokio::sync::broadcast` publishes events to all subscribers; `mpsc` provides per-connection channels.
- `#[serde(untagged)]` on `ServerMessage` lets variant shape drive serialization, while `#[repr(u8)]` on enums guarantees a stable numeric layout for the wire.
- `async fn` handlers use Axum extractors (`State`, `Path`, `Query`, `Json`) to parse requests, returning types that implement `IntoResponse`.

## Client (client/)
- **Entrypoint:** `client/src/main.rs` under `#[macroquad::main]`. Uses an app-state enum:
  - `Login` → `CharacterSelect` (native only) → `Playing`, or
  - `GuestMode` (dev shortcut). WASM builds run an offline demo loop.
- **Game state (`game/state.rs`):** Holds map, players/NPCs/items, UI state, camera, damage/level-up events, and inventory. `update` interpolates positions, follows the local player, and prunes transient effects.
- **Networking (`network/`):**
  - `NetworkClient` does REST matchmaking via `ureq` (`/matchmake/joinOrCreate/game_room`), then connects a WebSocket with `ewebsock` to `/{room}?sessionId=...`.
  - Incoming MessagePack is decoded with `network::protocol::decode_message`, then dispatched by message type to mutate `GameState` (positions, HP, inventory, drops, chat, etc.).
  - Outgoing input commands become `ClientMessage` variants and are encoded to `[13, "type", {data}]` with `protocol::encode_message`.
- **Input (`input/handler.rs`):** Polls keyboard/mouse at ~20 Hz send interval to mirror server tick. Produces commands for movement (cardinal only), attack, targeting, chat, pickup, and quick-use items.
- **Rendering (`render/`):**
  - `isometric.rs` handles world↔screen transforms and depth sorting helpers; tiles are 64×32 diamonds.
  - `renderer.rs` paints ground, depth-sorts players/NPCs/items/object tiles, overlays damage numbers and level-up text, and draws simple UI (connection status, inventory, chat feed).
- **UI/Auth (native):** `ui/screens.rs` draws login/character screens in Macroquad; `auth/client.rs` wraps the server auth endpoints (`/api/login`, `/api/register`, `/api/logout`, plus stub character APIs).
- **Assets:** Procedural tiles/colors for now (`game/tilemap.rs`); `assets/` reserved for future sprites and audio stubs live in `audio/`.

### Rust-specific notes (client)
- Conditional compilation (`#[cfg(not(target_arch = "wasm32"))]`) strips network/auth for WASM demo builds.
- `#[serde(rename_all = "camelCase")]` mirrors server JSON casing during matchmaking/auth.
- Macroquad runs an async main loop; `next_frame().await` yields to the engine each frame.
- Interpolation: server sends grid-aligned `i32` positions; client stores `f32` targets and linearly interpolates (`interpolate_visual`) for smooth movement.

## Message Flow Cheatsheet
1) **Matchmaking:** Client POSTs to `/matchmake/joinOrCreate/game_room`, receives `{roomId, sessionId}`.
2) **Connect:** WebSocket to `ws://host:2567/{roomId}?sessionId=...`.
3) **On open:** Server sends `Welcome {player_id}` + existing players (`PlayerJoined`).
4) **Gameplay loop:** Client sends movement/attack/target/chat/pickup/useItem; server ticks at 20 Hz, resolves NPC AI/combat/collision, and broadcasts `StateSync` plus event messages.
5) **Persistence:** Auto-saves every 30 s and on disconnect using `GameRoom::get_player_save_data` → `db.save_player`.

## Quick mental model
- The server is authoritative on a grid map; players/NPCs move tile-by-tile with cooldowns. Every 50 ms it broadcasts the authoritative grid state. The client keeps its own smooth visuals by interpolating toward those grid coordinates and only ever sends intents (no physics).
- Inventory and drops are fully server-driven; the client just renders the latest `InventoryUpdate` and ground item list.
- Protocol stability relies on the numeric enum representations (`as u8`) and the consistent MessagePack array format `[13, "msgType", map]` on both sides.
