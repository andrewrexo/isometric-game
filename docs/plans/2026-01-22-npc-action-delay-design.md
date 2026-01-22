# NPC Action Delay System

## Overview

Add configurable min/max delays before NPCs take hostile actions (chasing movement and attacks), giving players reaction time to escape or counterattack.

## TOML Configuration

```toml
[pig.behaviors]
hostile = true
action_delay_min_ms = 200   # minimum delay before each hostile action
action_delay_max_ms = 600   # maximum delay before each hostile action
```

- Defaults to 0/0 (no delay) if not specified, preserving current behavior
- Only applies to hostile actions - wandering, returning home, and idle behavior are unaffected

## Implementation

### NPC Struct Changes (`npc.rs`)

```rust
// Add to PrototypeStats
pub action_delay_min_ms: u64,
pub action_delay_max_ms: u64,

// Add to Npc struct
pub action_delay_until: u64,  // timestamp when current delay expires
```

### State Machine Logic

1. **Entering Chasing state** (from Idle/Wandering):
   - Roll initial delay so the NPC doesn't instantly start moving

2. **Chasing state** - Before `try_move_toward()`:
   - If `current_time < action_delay_until`, skip the move (wait)
   - After a successful move, roll new delay: `action_delay_until = current_time + rand(min, max)`

3. **Attacking state** - Before dealing damage:
   - If `current_time < action_delay_until`, skip the attack (wait)
   - After attacking, roll new delay for next action

### Files to Modify

- `rust-server/src/entity/prototype.rs` - Add fields to raw/resolved structs
- `rust-server/src/npc.rs` - Add field to PrototypeStats, Npc, and update logic
- `rust-server/data/entities/monsters/*.toml` - Add config to monsters (optional)

## Behavior Summary

| State | Delay Applied? | Notes |
|-------|---------------|-------|
| Idle | No | Normal aggro detection |
| Wandering | No | Normal movement |
| Chasing | Yes | Delay before each move toward player |
| Attacking | Yes | Delay before each attack |
| Returning | No | Normal movement back to spawn |
| Dead | No | N/A |
