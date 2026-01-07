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
- [x] Grid-based movement (250ms per tile = 4 tiles/sec)
- [x] Smooth visual interpolation at 60fps
- [x] Cardinal-only movement (no diagonals for isometric grid)
- [x] WASM build working (offline demo mode)

### Phase 3: Combat - COMPLETE
- [x] Click-to-target system (select enemy/player)
- [x] Space-to-attack mechanic (changed from CTRL for macOS compatibility)
- [x] Range validation (1.5 tile attack range)
- [x] Attack cooldown (1 second)
- [x] Health bars (players and NPCs)
- [x] Floating damage numbers (rise + fade animation)
- [x] NPC spawning and state sync (5 Slimes)
- [x] NPC AI (idle, aggro, chase, attack, return states)
- [x] NPC grid-based movement with smooth interpolation
- [x] Death and respawn (5 second player respawn, 10 second NPC respawn)
- [x] "You Died" overlay with respawn countdown

### Phase 4: Progression - COMPLETE
- [x] EXP/Level system (kill NPCs for EXP)
- [x] Level-up rewards (+10 max HP per level, full heal)
- [x] EXP bar UI with current/needed display
- [x] Floating "LEVEL UP!" text animation
- [x] SQLite database structure (ready for persistence integration)

### Phase 5: Items - COMPLETE
- [x] Item drops on NPC death (loot tables with drop rates)
- [x] Ground item rendering with bobbing animation
- [x] F key to pickup nearest item (2 tile range)
- [x] Item ownership period (only killer can loot for 30 seconds)
- [x] Item despawn (60 second lifetime)
- [x] Inventory UI (5x4 grid, I key to toggle)
- [x] Item stacking (potions stack to 99)
- [x] Gold counter
- [x] Quick slots (1-5 keys)
- [x] Use items (Health Potion heals 30 HP)
- [ ] Equipment slots (stretch goal)

### Phase 6: Social - COMPLETE
- [x] Chat input (Enter to open, Enter to send, Escape to cancel)
- [x] Chat message display with sender names
- [x] Floating player names above characters
- [ ] Emotes (stretch goal)

### Phase 7: Polish - IN PROGRESS
- [x] WASM build compiles (offline demo)
- [x] Grid-based movement for all entities
- [x] Smooth visual interpolation (pixel-perfect movement)
- [ ] WASM networking (blocked by ewebsock/miniquad conflict)
- [ ] Sound effects and music
- [ ] Sprite art (currently using placeholder shapes)
- [ ] Performance optimization
- [ ] Bug fixes and testing

---

## Controls

| Key | Action |
|-----|--------|
| WASD / Arrows | Move (cardinal directions) |
| Space | Attack target |
| Left Click | Target entity |
| Escape | Clear target / Cancel chat |
| Enter | Open/send chat |
| I | Toggle inventory |
| 1-5 | Use quick slot item |
| F | Pickup nearest item |
| F3 | Toggle debug info |

---

## Project Structure

```
isometric-game/
├── client/                     # Rust/macroquad client
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs            # Entry point, game loop
│   │   ├── game/              # State, entities, items, NPCs, tilemap
│   │   ├── render/            # Isometric rendering, UI
│   │   ├── network/           # WebSocket client, protocol
│   │   └── input/             # Keyboard/mouse handling
│   └── web/                   # WASM build output
│
├── rust-server/               # Rust/axum server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # Server entry, HTTP + WebSocket
│       ├── game.rs           # Game room, player, tick loop
│       ├── npc.rs            # NPC entity and AI
│       ├── item.rs           # Items and inventory
│       ├── db.rs             # SQLite database
│       ├── protocol.rs       # MessagePack encoding
│       └── tilemap.rs        # Map collision
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
| Movement | 4 tiles/second | 250ms per tile, grid-based |
| NPC Movement | 2 tiles/second | 500ms per tile, grid-based |

---

## Architecture

### Client Game Loop (60 fps)
```
1. Poll network messages (state sync from server)
2. Handle input (WASD, Space, mouse, keys)
3. Send commands to server (move, attack, chat, pickup, use item)
4. Update state:
   - All entities: smooth interpolation toward server grid positions
5. Render (isometric depth sort, UI overlays)
6. Debug overlay (FPS, player count, connection status)
```

### Server Tick Loop (20 Hz)
```
1. Process incoming client commands
2. Validate and apply grid movement (250ms cooldown)
3. Run NPC AI (aggro, chase, attack, return)
4. Process combat (damage, death, respawn)
5. Update items (drops, pickups, despawns)
6. Broadcast state sync to all clients
```

### Movement System
- **Grid-based**: Players and NPCs move in whole tiles only
- **Server authoritative**: Server validates all movement
- **Visual interpolation**: Client smoothly animates between grid positions
- **Player speed**: 4 tiles/sec (250ms per tile)
- **NPC speed**: 2 tiles/sec (500ms per tile)

---

## NPC AI State Machine

```
┌─────────┐  player in range   ┌─────────┐
│  IDLE   │ ─────────────────► │ CHASING │
└─────────┘                    └─────────┘
     ▲                              │
     │                              │ in attack range
     │                              ▼
     │                         ┌──────────┐
     │   too far from spawn    │ATTACKING │
     └─────────────────────────┴──────────┘
                                    │
                                    │ target lost/died
                                    ▼
                              ┌───────────┐
                              │ RETURNING │
                              └───────────┘
                                    │
                                    │ reached spawn
                                    ▼
                                 (IDLE)
```

---

## Known Issues / Technical Debt

1. **WASM Networking**: ewebsock uses wasm-bindgen which conflicts with miniquad's WASM loader. Current WASM build is offline demo only.

2. **Placeholder Graphics**: All entities rendered as colored shapes. Need sprite art.

3. **Database Integration**: SQLite structure exists but persistence not fully wired up (player save/load on connect/disconnect).

4. **Direction Validation**: Attack doesn't currently require facing the target (range check only).

---

## Remaining Work

### High Priority
1. **Sprite Art**: Replace placeholder shapes with pixel art
2. **Database Integration**: Wire up player save/load
3. **Direction Check**: Require facing target to attack

### Medium Priority
4. **Sound Effects**: Attack, damage, death, pickup, level up sounds
5. **More NPC Types**: Add variety beyond Slimes
6. **Equipment System**: Weapon/armor slots

### Low Priority (Stretch Goals)
7. **WASM Networking**: Find alternative to ewebsock
8. **Multiple Zones**: Zone transitions
9. **Emotes**: /wave, /dance, etc.
10. **Parties/Trading**: Social features

---

## Running the Game

```bash
# Terminal 1 - Start server
cd rust-server && cargo run --release

# Terminal 2 - Start client
cd client && cargo run --release
```

Server runs on `http://localhost:2567` with WebSocket upgrade for game connections.
