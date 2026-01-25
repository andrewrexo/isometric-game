# XP Globes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add circular XP globe notifications that appear when gaining XP, showing skill icon + progress ring.

**Architecture:** New `xp_globes.rs` UI module with `XpGlobe` struct and `XpGlobesManager`. Hook into existing `skillXp` network event. Render to the left of player stats in top-right corner.

**Tech Stack:** Rust, Macroquad (draw_circle, draw_line, draw_texture_ex)

---

### Task 1: Create XP Globes Module Structure

**Files:**
- Create: `client/src/render/ui/xp_globes.rs`
- Modify: `client/src/render/ui/mod.rs`

**Step 1: Create the xp_globes.rs file with structs and constants**

```rust
//! XP globe notifications - circular progress indicators for skill XP gains

use macroquad::prelude::*;
use crate::game::SkillType;

// ============================================================================
// Constants
// ============================================================================

pub const GLOBE_SIZE: f32 = 40.0;
pub const GLOBE_SPACING: f32 = 4.0;
const ICON_SIZE: f32 = 24.0;
const RING_THICKNESS: f32 = 3.0;
const VISIBLE_DURATION: f64 = 3.0;  // Seconds before fade starts
const FADE_OUT_DURATION: f64 = 0.5; // Seconds to fully fade

// UI icons sprite sheet: 24x24 icons in 10 columns
const UI_ICON_SIZE: f32 = 24.0;

// ============================================================================
// XP Globe
// ============================================================================

/// A single XP globe notification
pub struct XpGlobe {
    pub skill_type: SkillType,
    pub current_xp: i64,
    pub xp_for_next_level: i64,
    pub level: i32,
    pub last_updated: f64,
}

impl XpGlobe {
    pub fn new(skill_type: SkillType, current_xp: i64, xp_for_next_level: i64, level: i32) -> Self {
        Self {
            skill_type,
            current_xp,
            xp_for_next_level,
            level,
            last_updated: macroquad::time::get_time(),
        }
    }

    /// Calculate progress toward next level (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.xp_for_next_level <= 0 {
            return 1.0;
        }
        let current_level_xp = crate::game::skills::total_xp_for_level(self.level);
        let xp_in_level = self.current_xp - current_level_xp;
        let xp_needed = self.xp_for_next_level - current_level_xp;
        if xp_needed <= 0 {
            return 1.0;
        }
        (xp_in_level as f32 / xp_needed as f32).clamp(0.0, 1.0)
    }

    /// Get opacity based on time since last update
    pub fn opacity(&self, current_time: f64) -> f32 {
        let age = current_time - self.last_updated;
        if age < VISIBLE_DURATION {
            1.0
        } else {
            let fade_progress = (age - VISIBLE_DURATION) / FADE_OUT_DURATION;
            (1.0 - fade_progress as f32).clamp(0.0, 1.0)
        }
    }

    /// Check if globe should be removed
    pub fn is_expired(&self, current_time: f64) -> bool {
        current_time - self.last_updated > VISIBLE_DURATION + FADE_OUT_DURATION
    }
}

// ============================================================================
// XP Globes Manager
// ============================================================================

/// Manages active XP globe notifications
#[derive(Default)]
pub struct XpGlobesManager {
    pub globes: Vec<XpGlobe>,
}

impl XpGlobesManager {
    pub fn new() -> Self {
        Self { globes: Vec::new() }
    }

    /// Handle an XP gain event
    pub fn on_xp_gain(&mut self, skill_type: SkillType, current_xp: i64, xp_for_next_level: i64, level: i32) {
        // Check if globe for this skill already exists
        if let Some(globe) = self.globes.iter_mut().find(|g| g.skill_type == skill_type) {
            // Update existing globe
            globe.current_xp = current_xp;
            globe.xp_for_next_level = xp_for_next_level;
            globe.level = level;
            globe.last_updated = macroquad::time::get_time();
        } else {
            // Create new globe (insert at beginning so it appears on the left)
            self.globes.insert(0, XpGlobe::new(skill_type, current_xp, xp_for_next_level, level));
        }
    }

    /// Update globes, removing expired ones
    pub fn update(&mut self) {
        let current_time = macroquad::time::get_time();
        self.globes.retain(|globe| !globe.is_expired(current_time));
    }
}
```

**Step 2: Add module export to mod.rs**

Add to `client/src/render/ui/mod.rs`:
```rust
pub mod xp_globes;
```

**Step 3: Verify it compiles**

Run: `cd /Users/samson/projects/isometric-game/client && cargo check 2>&1 | head -30`
Expected: No errors related to xp_globes module

**Step 4: Commit**

```bash
git add client/src/render/ui/xp_globes.rs client/src/render/ui/mod.rs
git commit -m "feat(client): add XP globes module structure"
```

---

### Task 2: Add XpGlobesManager to GameState

**Files:**
- Modify: `client/src/game/state.rs`
- Modify: `client/src/game/mod.rs`

**Step 1: Add XpGlobesManager to GameState struct**

In `client/src/game/state.rs`, add import at top:
```rust
use crate::render::ui::xp_globes::XpGlobesManager;
```

In the `GameState` struct (around line 537 near `skill_xp_events`), add:
```rust
    pub xp_globes: XpGlobesManager,
```

In `GameState::new()` (around line 613), add initialization:
```rust
            xp_globes: XpGlobesManager::new(),
```

**Step 2: Re-export in game/mod.rs if needed**

No change needed - XpGlobesManager is accessed through render::ui::xp_globes.

**Step 3: Verify it compiles**

Run: `cd /Users/samson/projects/isometric-game/client && cargo check 2>&1 | head -30`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add client/src/game/state.rs
git commit -m "feat(client): add XpGlobesManager to GameState"
```

---

### Task 3: Hook XP Gain Events to XpGlobesManager

**Files:**
- Modify: `client/src/network/client.rs`

**Step 1: Call xp_globes.on_xp_gain in skillXp handler**

In `client/src/network/client.rs`, find the `skillXp` handler (around line 936). After the existing `skill_xp_events.push(...)` call, add:

```rust
                            // Update XP globes
                            if let Some(skill_type) = SkillType::from_str(&skill_name) {
                                let xp_for_next = crate::game::skills::total_xp_for_level(level + 1);
                                state.xp_globes.on_xp_gain(skill_type, total_xp, xp_for_next, level);
                            }
```

**Step 2: Verify it compiles**

Run: `cd /Users/samson/projects/isometric-game/client && cargo check 2>&1 | head -30`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add client/src/network/client.rs
git commit -m "feat(client): hook XP gain events to XpGlobesManager"
```

---

### Task 4: Implement Globe Rendering

**Files:**
- Modify: `client/src/render/ui/xp_globes.rs`
- Modify: `client/src/render/renderer.rs`

**Step 1: Add rendering function to xp_globes.rs**

Add to the end of `client/src/render/ui/xp_globes.rs`:

```rust
use super::common::*;
use crate::render::Renderer;

impl Renderer {
    /// Render XP globes to the left of player stats
    pub fn render_xp_globes(&self, xp_globes: &XpGlobesManager, stats_left_x: f32, stats_center_y: f32) {
        let current_time = macroquad::time::get_time();

        // Globes render right-to-left from stats area
        // Most recent XP (first in vec) appears leftmost, but we want it rightmost
        // So iterate in reverse order
        let mut x = stats_left_x - GLOBE_SPACING - GLOBE_SIZE;

        for globe in xp_globes.globes.iter().rev() {
            let opacity = globe.opacity(current_time);
            if opacity <= 0.0 {
                continue;
            }

            let center_x = x + GLOBE_SIZE / 2.0;
            let center_y = stats_center_y;

            self.draw_xp_globe(globe, center_x, center_y, opacity);

            x -= GLOBE_SIZE + GLOBE_SPACING;
        }
    }

    fn draw_xp_globe(&self, globe: &XpGlobe, center_x: f32, center_y: f32, opacity: f32) {
        let radius = GLOBE_SIZE / 2.0;
        let inner_radius = radius - RING_THICKNESS;

        // Get skill color
        let skill_color = self.get_xp_globe_skill_color(globe.skill_type);

        // Background circle (dark)
        draw_circle(
            center_x,
            center_y,
            radius,
            Color::new(0.05, 0.05, 0.07, 0.95 * opacity)
        );

        // Dark ring border
        draw_circle_lines(
            center_x,
            center_y,
            radius - 1.0,
            2.0,
            Color::new(0.2, 0.18, 0.15, opacity)
        );

        // Progress arc
        let progress = globe.progress();
        if progress > 0.0 {
            self.draw_progress_arc(center_x, center_y, radius - 2.0, inner_radius + 1.0, progress, skill_color, opacity);
        }

        // Inner dark circle (behind icon)
        draw_circle(
            center_x,
            center_y,
            inner_radius,
            Color::new(0.08, 0.08, 0.10, opacity)
        );

        // Skill icon
        self.draw_xp_globe_icon(globe.skill_type, center_x, center_y, opacity);
    }

    fn draw_progress_arc(&self, cx: f32, cy: f32, outer_r: f32, inner_r: f32, progress: f32, color: Color, opacity: f32) {
        // Draw arc as a series of small segments
        let segments = 32;
        let end_angle = progress * std::f32::consts::TAU;
        let start_angle = -std::f32::consts::FRAC_PI_2; // Start from top

        let mid_r = (outer_r + inner_r) / 2.0;
        let thickness = outer_r - inner_r;

        for i in 0..segments {
            let t0 = i as f32 / segments as f32;
            let t1 = (i + 1) as f32 / segments as f32;

            if t0 >= progress {
                break;
            }

            let t1_clamped = t1.min(progress);

            let angle0 = start_angle + t0 * std::f32::consts::TAU;
            let angle1 = start_angle + t1_clamped * std::f32::consts::TAU;

            let x0 = cx + angle0.cos() * mid_r;
            let y0 = cy + angle0.sin() * mid_r;
            let x1 = cx + angle1.cos() * mid_r;
            let y1 = cy + angle1.sin() * mid_r;

            draw_line(x0, y0, x1, y1, thickness, Color::new(color.r, color.g, color.b, opacity));
        }
    }

    fn draw_xp_globe_icon(&self, skill_type: SkillType, center_x: f32, center_y: f32, opacity: f32) {
        // Icon positions in ui_icons.png (same as skills panel)
        let (icon_col, icon_row) = match skill_type {
            SkillType::Hitpoints => (0, 6),
            SkillType::Combat => (2, 6),
        };

        if let Some(ref texture) = self.ui_icons {
            let src_x = icon_col as f32 * UI_ICON_SIZE;
            let src_y = icon_row as f32 * UI_ICON_SIZE;
            let src_rect = Rect::new(src_x, src_y, UI_ICON_SIZE, UI_ICON_SIZE);

            let icon_x = center_x - ICON_SIZE / 2.0;
            let icon_y = center_y - ICON_SIZE / 2.0;

            draw_texture_ex(
                texture,
                icon_x,
                icon_y,
                Color::new(1.0, 1.0, 1.0, opacity),
                DrawTextureParams {
                    source: Some(src_rect),
                    dest_size: Some(Vec2::new(ICON_SIZE, ICON_SIZE)),
                    ..Default::default()
                },
            );
        } else {
            // Fallback to letter
            let letter = match skill_type {
                SkillType::Hitpoints => "H",
                SkillType::Combat => "C",
            };
            let color = self.get_xp_globe_skill_color(skill_type);
            let dims = self.measure_text_sharp(letter, 18.0);
            self.draw_text_sharp(
                letter,
                center_x - dims.width / 2.0,
                center_y + 6.0,
                18.0,
                Color::new(color.r, color.g, color.b, opacity)
            );
        }
    }

    fn get_xp_globe_skill_color(&self, skill_type: SkillType) -> Color {
        match skill_type {
            SkillType::Hitpoints => Color::new(0.8, 0.2, 0.2, 1.0),
            SkillType::Combat => Color::new(0.85, 0.65, 0.15, 1.0),
        }
    }
}
```

**Step 2: Verify it compiles**

Run: `cd /Users/samson/projects/isometric-game/client && cargo check 2>&1 | head -30`
Expected: Compiles (may have warnings about unused code until integration)

**Step 3: Commit**

```bash
git add client/src/render/ui/xp_globes.rs
git commit -m "feat(client): implement XP globe rendering"
```

---

### Task 5: Integrate Rendering into UI Loop

**Files:**
- Modify: `client/src/render/renderer.rs`

**Step 1: Add update call in main loop and render call near player stats**

In `client/src/render/renderer.rs`, find the `render_ui` function. Near the player stats rendering (around line 3219), add the XP globes rendering.

First, add the xp_globes update call. Find where the function starts and add early:
```rust
        // Update XP globes fade timers
        // Note: We need mutable access, so this should be done in the game loop
        // For now, we'll just render - update is called elsewhere
```

Then, after the HP bar rendering (around line 3281), add:
```rust
            // XP Globes (to the left of player stats)
            let globe_stats_y = tag_y + tag_height / 2.0; // Vertically centered with name tag
            self.render_xp_globes(&state.xp_globes, bar_x, globe_stats_y);
```

**Step 2: Add xp_globes.update() call in main game loop**

Find where `state.update()` is called (likely in main.rs or game loop). The xp_globes update needs to happen each frame.

In `client/src/game/state.rs`, in the `update` method (around line 700), add:
```rust
        // Update XP globes (fade timers)
        self.xp_globes.update();
```

**Step 3: Verify it compiles**

Run: `cd /Users/samson/projects/isometric-game/client && cargo check 2>&1 | head -30`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add client/src/render/renderer.rs client/src/game/state.rs
git commit -m "feat(client): integrate XP globes into render loop"
```

---

### Task 6: Test and Polish

**Step 1: Build and run**

Run: `cd /Users/samson/projects/isometric-game/client && cargo build 2>&1 | tail -20`
Expected: Build succeeds

**Step 2: Manual testing**

- Start the game
- Attack an enemy to gain Combat XP
- Verify globe appears to the left of player stats
- Verify progress ring shows correct fill
- Verify globe fades after 3 seconds
- Gain XP again quickly - verify it updates existing globe
- Gain XP in different skill - verify multiple globes stack

**Step 3: Final commit if any adjustments needed**

```bash
git add -A
git commit -m "feat(client): polish XP globes implementation"
```
