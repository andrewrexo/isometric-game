# Gathering System Design

A generic gathering system for fishing, woodcutting, mining, and future skills. Designed for semi-AFK gameplay with competitive bonus tile mechanics.

## Core Loop

1. Player walks to a gathering marker tile (e.g., next to a lake)
2. Player activates gathering (click/keypress)
3. If tile is unoccupied and player meets level requirement, player claims the tile and begins gathering
4. Gathering runs automatically - loot rolls happen at intervals based on zone's `base_gather_speed`
5. Successful gathers add items to inventory and grant XP (zone base + item rarity bonus)
6. Gathering continues until:
   - Player moves away or cancels
   - Inventory fills (gathering pauses, player keeps tile)

### Bonus Tile Events

1. Periodically (per zone's `bonus_spawn_frequency`), a random unclaimed marker tile in the zone begins telegraphing (5s visual warning)
2. First player to reach and claim it gets a 30-second 2x gathering speed buff
3. If no one claims it before telegraph ends, it fades

### Tile Claiming

- One player per marker tile
- Moving off the tile releases it immediately

## Progression

- **Skill levels only** - No equipment requirements
- **XP curve** - Uses existing RS-style exponential curve (shared with combat skills)
- **XP per gather** - Zone base XP + item rarity bonus XP

## Data Architecture

### Gathering Zones (`gathering_zones.toml`)

```toml
[zones.lumbridge_pond]
skill = "fishing"
level_required = 1
loot_table = "fish_beginner"
bonus_spawn_frequency = 60
base_gather_speed = 5.0

[zones.barbarian_village_trees]
skill = "woodcutting"
level_required = 15
loot_table = "logs_oak"
bonus_spawn_frequency = 45
base_gather_speed = 4.0
```

### Loot Tables (`loot_tables.toml`)

Tiered roll system with skill-scaled weights:

```toml
[fish_beginner]
skill = "fishing"

[fish_beginner.tiers.common]
base_weight = 70
level_scaling = -0.5  # loses 0.5 weight per player level
items = [
    { id = "raw_shrimp", level = 1, weight = 10, xp_bonus = 0 },
    { id = "raw_sardine", level = 5, weight = 10, xp_bonus = 5 },
]

[fish_beginner.tiers.uncommon]
base_weight = 25
level_scaling = 0.3
items = [
    { id = "raw_herring", level = 10, weight = 10, xp_bonus = 10 },
]

[fish_beginner.tiers.rare]
base_weight = 5
level_scaling = 0.2
items = [
    { id = "raw_trout", level = 20, weight = 10, xp_bonus = 25 },
]
```

### Loot Roll Logic

1. Calculate adjusted tier weights based on player skill level
2. Roll to select tier (common/uncommon/rare)
3. Filter items in tier by player level requirement
4. Roll weighted selection among eligible items

## Map Editor Integration

### Marker Tile Data

Each marker tile stores only:
- `zone_id: string` (e.g., "lumbridge_pond")

### Map File Structure

```toml
[[gathering_markers]]
x = 15
y = 22
zone_id = "lumbridge_pond"

[[gathering_markers]]
x = 16
y = 22
zone_id = "lumbridge_pond"
```

### Mapper Tool Changes

1. **New tool mode** - "Gathering Marker" brush for placing markers
2. **Zone ID selector** - Dropdown populated from `gathering_zones.toml`
3. **Visual indicator** - Colored overlay per skill type (blue = fishing, green = woodcutting)
4. **Validation** - Warn if marker references nonexistent zone ID

## Server/Client Responsibilities

### Server (authoritative)

- Loads zone configs and loot tables from TOML
- Tracks which player occupies each marker tile
- Runs gather tick timer per active gatherer
- Rolls loot (tier → item) and validates level requirements
- Grants items and XP
- Spawns bonus tile events, tracks telegraph countdown
- Awards 2x buff to first claimer
- Validates all player actions

### Client

- Sends: gather start, gather cancel, movement
- Receives: gather results, bonus tile events, buff events
- Renders: gathering animation, bonus tile telegraph, buff indicator
- UI: skill XP/level, buff timer

### Network Events

```
Client → Server:
- StartGathering { marker_position }
- StopGathering

Server → Client:
- GatheringStarted { marker_position, zone_id }
- GatheringResult { item_id, xp_gained }
- GatheringStopped { reason: "inventory_full" | "cancelled" | "moved" }
- BonusTileSpawned { position, telegraph_duration }
- BonusTileClaimed { position, player_id }
- BonusTileExpired { position }
- BuffApplied { buff_type, duration }
- BuffExpired { buff_type }
```

## Generic System Architecture

### Shared Core

All gathering skills use:
- Marker tile claiming/releasing
- Zone config loading
- Loot table rolling (tiered + level-gated)
- XP granting (zone base + item bonus)
- Bonus tile spawning, telegraphing, claiming
- 2x buff application
- Inventory checks

### Skill-Specific Hooks

```rust
trait GatheringSkill {
    fn skill_type(&self) -> SkillType;

    // Optional hooks with default implementations
    fn on_gather_tick(&self, ctx: &mut GatherContext) {}
    fn modify_loot_roll(&self, roll: &mut LootRoll) {}
    fn modify_gather_speed(&self, base: f32, ctx: &GatherContext) -> f32 { base }
    fn on_bonus_claimed(&self, ctx: &mut GatherContext) {}
}
```

### Potential Skill Twists (future)

| Skill | Possible Twist |
|-------|----------------|
| Fishing | Weather affects rare spawn rates |
| Woodcutting | Trees deplete after X gathers, respawn after timer |
| Mining | Ore veins have hidden "richness" that depletes |
| Farming | Time-based growth cycles instead of instant gather |

## UI Elements

### During Gathering

- Skill icon + progress indicator for next gather tick
- Floating text or inventory flash on item gathered

### Buff Active

- Buff icon with countdown timer (30s → 0)
- Visual effect on player (glow, sparkle)

### Bonus Tile Telegraph

- Ground effect on target tile (pulsing glow, expanding ring)
- 5 second countdown visible to nearby players
- Sound cue when it spawns

### Skills Panel

- Gathering skills listed alongside combat skills (existing panel)
- XP bar, current level, XP to next level

## Future Considerations

- **Anti-AFK measures** - Kick idle players from tiles after inactivity
- **Skill-specific twists** - Weather, depletion, growth cycles
- **Overcrowding mechanics** - If one-player-per-tile feels too limiting
- **Rare events** - Special spawns (treasure from fishing, bird nests from trees)
- **Tool/equipment tier** - If skill-only progression feels flat
- **Achievements/milestones** - First catch, level milestones, rare discoveries

## Summary

| Aspect | Decision |
|--------|----------|
| Core loop | AFK gathering + bonus tile races |
| Progression | Skill levels only, RS XP curve |
| Attention mechanic | Shared telegraphed bonus tiles (5s), 2x buff (30s) |
| Spatial model | Explicit marker tiles → zone ID reference |
| Config | `gathering_zones.toml` + `loot_tables.toml` |
| Loot | Tiered rolls, skill-scaled weights |
| Multiplayer | One player per tile |
| Architecture | Shared core + skill-specific hooks |
