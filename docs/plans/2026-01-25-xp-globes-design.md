# XP Globes Design

## Overview

Add circular XP globe notifications that appear when the player gains XP. Globes display the skill icon with a progress ring showing progress toward the next level.

## Visual Design

Each XP globe is a **40×40 pixel** circular element with three layers:

1. **Outer ring** - Dark metallic border (matching existing panel frame styling)
2. **Progress arc** - Colored circular progress bar showing % to next level, drawn clockwise from top
3. **Center icon** - 24×24 skill icon from `ui_icons.png`, centered

### Colors

- Progress arc uses existing skill colors from `skills.rs`:
  - Hitpoints = red (`Color::new(0.8, 0.2, 0.2)`)
  - Combat = gold (`Color::new(0.85, 0.65, 0.15)`)
  - (Additional skills follow same pattern)
- Background inside ring is dark/semi-transparent
- Outer border uses dark bronze from `common.rs`

### Positioning

- Anchored to the left of player stats area (top-right of screen)
- Globes stack horizontally leftward with 4px spacing
- Vertically aligned with the player name tag
- Most recent XP gain is rightmost (closest to player stats)

## Behavior

### On XP Gain Event

1. Check if a globe for that skill already exists
2. If exists: Update XP values, reset the fade timer
3. If new: Create globe, add to the left of existing globes

### Fade Out

- Globe stays visible for **3 seconds** after last XP update
- Then fades out over ~0.5 seconds (opacity decrease)
- Fully faded globes are removed from the active list

### Stacking

- One globe per skill (no duplicates)
- Rapid XP gains to same skill accumulate on existing globe
- Multiple different skills can display simultaneously

## Data Structures

```rust
pub struct XpGlobe {
    pub skill_id: SkillId,
    pub current_xp: u32,
    pub xp_for_next_level: u32,
    pub last_updated: f64,  // Time of last XP gain
    pub opacity: f32,       // For fade animation (1.0 = full, 0.0 = invisible)
}

pub struct XpGlobesManager {
    pub globes: Vec<XpGlobe>,
}
```

## Implementation

### New File

`client/src/render/ui/xp_globes.rs`:
- `XpGlobe` struct
- `XpGlobesManager` with methods:
  - `on_xp_gain(skill_id, current_xp, xp_for_level)` - Handle incoming XP event
  - `update(delta_time)` - Update fade timers, remove expired globes
  - `render()` - Draw all active globes

### Integration Points

1. **Module export**: `client/src/render/ui/mod.rs` - Add `pub mod xp_globes;`

2. **Rendering**: `client/src/render/renderer.rs` (~line 3219)
   - Call `xp_globes_manager.update()` each frame
   - Call `render_xp_globes()` in `render_ui()` near player stats

3. **Network handler**: Where XP gain server messages are processed
   - Call `xp_globes_manager.on_xp_gain(skill_id, current_xp, xp_for_level)`

### Drawing Approach

- `draw_circle()` for filled background
- Custom arc drawing for progress ring (iterate segments with `draw_line()` or use `draw_poly()`)
- `draw_texture_ex()` for skill icon (same approach as skills panel)

## Constants

```rust
const GLOBE_SIZE: f32 = 40.0;
const ICON_SIZE: f32 = 24.0;
const GLOBE_SPACING: f32 = 4.0;
const FADE_DURATION: f32 = 3.0;      // Seconds before fade starts
const FADE_OUT_TIME: f32 = 0.5;      // Seconds to fully fade
const RING_THICKNESS: f32 = 3.0;     // Progress arc width
```
