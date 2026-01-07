# Isometric MMORPG - Implementation Plan

## Overview
Build an isometric 2.5D pixel-art MMORPG with:
- **Client**: Rust + macroquad (native + WASM)
- **Server**: Rust + axum (WebSocket + HTTP)
- **MVP Scope**: Single zone, 10-50 players, basic combat/items/chat

---

## Current Progress

### Phase 1: Foundation - COMPLETE
- [x] Rust client with macroquad
- [x] Rust server with axum (replaced Colyseus due to connection issues)
- [x] WebSocket connection working
- [x] MessagePack protocol for efficient serialization
- [x] Player join/leave with state sync
- [x] WASD movement with server validation

### Phase 2: World Rendering - COMPLETE
- [x] Tilemap system with Tiled JSON support
- [x] Isometric projection (world-to-screen transformation)
- [x] Depth sorting (painter's algorithm)
- [x] Camera system (follows player, centered on screen)
- [x] Server-side collision detection
- [x] Pixel-perfect smooth movement at 60fps
- [x] Client-side prediction with server reconciliation
- [x] Cardinal-only movement (no diagonals for isometric grid)
- [x] WASM build working (offline demo mode)

### Phase 3: Combat - NOT STARTED
- [ ] Click-to-target system (select enemy)
- [ ] CTRL-to-attack mechanic
- [ ] Direction validation (must face enemy)
- [ ] Range validation
- [ ] Attack animation + damage calculation
- [ ] Health bars and floating damage numbers
- [ ] NPC spawning and state sync
- [ ] NPC AI (aggro, chase, attack)
- [ ] Death and respawn

### Phase 4: Progression - NOT STARTED
- [ ] EXP/Level system
- [ ] Stats (modified by level)
- [ ] SQLite persistence (save/load characters)

### Phase 5: Items - NOT STARTED
- [ ] Item drops on NPC death
- [ ] Click-to-pickup
- [ ] Inventory UI (grid)
- [ ] Use items (potions)
- [ ] Equipment slots (stretch goal)

### Phase 6: Social - NOT STARTED
- [ ] Chat input and display
- [ ] Floating player names
- [ ] Emotes (stretch goal)

### Phase 7: Polish - PARTIAL
- [x] WASM build compiles (offline demo)
- [ ] WASM networking (blocked by ewebsock/miniquad conflict)
- [ ] Sound effects and music
- [ ] Performance optimization
- [ ] Bug fixes and testing

---

## Project Structure

```
isometric-game/
├── client/                     # Rust/macroquad client
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs            # Entry point, game loop
│   │   ├── game/              # State management, entities, tilemap
│   │   ├── render/            # Isometric rendering, sprites
│   │   ├── network/           # WebSocket client (ewebsock)
│   │   └── input/             # Keyboard/mouse handling
│   └── web/                   # WASM build output
│
├── rust-server/               # Rust/axum server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # Server entry, HTTP + WebSocket
│       ├── game.rs           # Game room logic, tick loop
│       └── protocol.rs       # MessagePack encoding
│
└── IMPLEMENTATION_PLAN.md    # This file
```

---

## Key Technical Details

| Component | Technology | Notes |
|-----------|------------|-------|
| Client Engine | macroquad 0.4 | Cross-platform (native + WASM) |
| Client WebSocket | ewebsock 0.6 | Native only (WASM has conflict) |
| Server Framework | axum | Async Rust web framework |
| Protocol | MessagePack | Efficient binary serialization |
| Tick Rate | 20 Hz server, 60 fps client | Smooth interpolation |
| Tile Size | 64x32 px | Standard 2:1 isometric ratio |
| Movement | 4 tiles/second | 250ms per tile, cardinal only |

---

## Architecture

### Client Game Loop (60 fps)
```
1. Poll network messages (state sync from server)
2. Handle input (WASD, mouse, CTRL)
3. Update state:
   - Local player: client-side prediction + server reconciliation
   - Other players: interpolation toward server position
4. Render (isometric depth sort)
5. Debug overlay (FPS, player count, connection status)
```

### Server Tick Loop (20 Hz)
```
1. Process incoming client commands
2. Validate and apply movement
3. Run game systems (combat, AI, respawn)
4. Broadcast state sync to all clients
```

### Client-Server Sync Strategy
- **On Join**: Server sends full player list
- **Every Tick**: Server sends positions of all players
- **Client Prediction**: Local player moves immediately on input
- **Reconciliation**: Smooth correction if client/server desync > 0.5 tiles

---

## Combat System Design (Phase 3)

### CTRL-to-Attack
- Not click-to-attack or auto-attack
- Player must press CTRL to initiate attack
- Adds tactical depth - positioning matters

### Validation Rules
1. Player must be **within attack range** of target
2. Player must be **facing the target** (4-direction check)
3. Server validates both before applying damage

### NPC AI
- Idle: Stand in spawn area
- Aggro: Player enters detection radius
- Chase: Move toward player at NPC speed
- Attack: When in range and facing player
- Reset: Return to spawn if player escapes

---

## Known Issues / Technical Debt

1. **WASM Networking**: ewebsock uses wasm-bindgen which conflicts with miniquad's WASM loader. Current WASM build is offline demo only.

2. **Unused Code Warnings**: Several structs/functions for Tiled JSON parsing are defined but not yet used. Will be needed when loading real map files.

3. **No Persistence**: Player data is not saved. Phase 4 will add SQLite.

---

## Next Steps

1. **Phase 3 Priority**: Implement basic combat
   - Start with click-to-target selection
   - Add CTRL attack with range check
   - Add simple NPC that can be attacked

2. **WASM Networking**: Investigate alternatives
   - Option A: Use quad-net or custom WebSocket via JS interop
   - Option B: Use wasm-bindgen build instead of miniquad loader
