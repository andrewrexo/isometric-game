# Area Banner Design

Display area names on screen when transitioning between locations (entering/leaving interiors).

## Overview

A classic RPG-style banner appears at the top center of the screen during map transitions, showing the current area name with decorative flourishes.

## Timing & Flow

```
Portal triggered → Fade out (0.25s) → Loading → Fade in (0.25s) → Banner visible (2.5s) → Banner fades (0.5s)
```

- Banner fades in synchronized with the map transition fade-in
- Stays fully visible for ~2.5 seconds after transition completes
- Gracefully fades out on its own
- Appears on every transition (not just first visit)

## Visual Design

```
         ═══════════════════════════
              Verdant Fields
         ───────────────────────────
```

**Elements:**
- Main text: Area name, centered, slightly larger than normal UI text
- Top flourish: Thin decorative line above
- Bottom flourish: Matching line below, slightly shorter
- Text shadow: Subtle dark shadow for readability
- Semi-transparent backing: Very subtle dark gradient behind

**Positioning:**
- Horizontally centered
- ~15-20% down from top of screen
- Scales with screen size (relative positioning)

**Colors:**
- Text: Off-white/cream (#F5F0E1) for parchment feel
- Flourishes: Same color, slightly more transparent
- Shadow: Dark brown/black at ~40% opacity

## Area Names

| Location | Display Name |
|----------|--------------|
| world_0 (overworld) | "Verdant Fields" |
| old_house | "Old House" |
| old_man_house | "Old Man's House" |
| (other interiors) | Uses `name` field from interior JSON |

## Implementation

### New File: `client/src/render/ui/area_banner.rs`

```rust
pub struct AreaBanner {
    pub text: String,
    pub phase: BannerPhase,
    pub timer: f32,
}

pub enum BannerPhase {
    Hidden,
    FadingIn,   // 0.5s
    Holding,    // 2.5s
    FadingOut,  // 0.5s
}
```

### Integration Points

1. **GameState** (`client/src/game/state.rs`)
   - Add `area_banner: AreaBanner` field

2. **Network handler** (`client/src/network/client.rs`)
   - On `interiorData`: trigger banner with interior name
   - On `locationChange` (back to overworld): trigger banner with "Verdant Fields"

3. **Server message update**
   - Add `name` field to `interiorData` message

4. **Main render loop**
   - Call `area_banner.render()` after other UI (draws on top)

### Edge Cases

- Rapid transitions: New banner replaces old, resets timer
- Missing name: Falls back to map ID

## Constants

```rust
const OVERWORLD_NAME: &str = "Verdant Fields";
const BANNER_FADE_IN: f32 = 0.5;
const BANNER_HOLD: f32 = 2.5;
const BANNER_FADE_OUT: f32 = 0.5;
```
