# Area Banner Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Display area names with decorative flourishes when transitioning between map locations.

**Architecture:** Client-side banner state tracks display phase (FadingIn → Holding → FadingOut → Hidden). Server sends interior name in `interiorData` message. Client shows "Verdant Fields" for overworld.

**Tech Stack:** Rust (client: macroquad, server: tokio)

---

### Task 1: Create AreaBanner Module

**Files:**
- Create: `client/src/render/ui/area_banner.rs`
- Modify: `client/src/render/ui/mod.rs`

**Step 1: Create the area_banner.rs file with state struct**

```rust
//! Area banner UI - displays location name during map transitions

/// Banner display phase
#[derive(Debug, Clone, PartialEq)]
pub enum BannerPhase {
    Hidden,
    FadingIn,
    Holding,
    FadingOut,
}

/// Timing constants
const FADE_IN_DURATION: f32 = 0.5;
const HOLD_DURATION: f32 = 2.5;
const FADE_OUT_DURATION: f32 = 0.5;

/// Overworld display name
pub const OVERWORLD_NAME: &str = "Verdant Fields";

/// Area banner state
#[derive(Debug, Clone)]
pub struct AreaBanner {
    pub text: String,
    pub phase: BannerPhase,
    pub timer: f32,
}

impl Default for AreaBanner {
    fn default() -> Self {
        Self {
            text: String::new(),
            phase: BannerPhase::Hidden,
            timer: 0.0,
        }
    }
}

impl AreaBanner {
    /// Trigger the banner with a new area name
    pub fn show(&mut self, name: &str) {
        self.text = name.to_string();
        self.phase = BannerPhase::FadingIn;
        self.timer = FADE_IN_DURATION;
    }

    /// Update the banner timer, transitioning phases as needed
    pub fn update(&mut self, delta: f32) {
        if self.phase == BannerPhase::Hidden {
            return;
        }

        self.timer -= delta;

        if self.timer <= 0.0 {
            match self.phase {
                BannerPhase::FadingIn => {
                    self.phase = BannerPhase::Holding;
                    self.timer = HOLD_DURATION;
                }
                BannerPhase::Holding => {
                    self.phase = BannerPhase::FadingOut;
                    self.timer = FADE_OUT_DURATION;
                }
                BannerPhase::FadingOut => {
                    self.phase = BannerPhase::Hidden;
                    self.timer = 0.0;
                }
                BannerPhase::Hidden => {}
            }
        }
    }

    /// Get current opacity (0.0 to 1.0)
    pub fn opacity(&self) -> f32 {
        match self.phase {
            BannerPhase::Hidden => 0.0,
            BannerPhase::FadingIn => 1.0 - (self.timer / FADE_IN_DURATION),
            BannerPhase::Holding => 1.0,
            BannerPhase::FadingOut => self.timer / FADE_OUT_DURATION,
        }
    }

    /// Check if banner should be rendered
    pub fn is_visible(&self) -> bool {
        self.phase != BannerPhase::Hidden
    }
}
```

**Step 2: Add module to mod.rs**

In `client/src/render/ui/mod.rs`, add after the existing modules:

```rust
pub mod area_banner;
```

**Step 3: Commit**

```bash
git add client/src/render/ui/area_banner.rs client/src/render/ui/mod.rs
git commit -m "feat(client): add area banner state module"
```

---

### Task 2: Add Banner to GameState

**Files:**
- Modify: `client/src/game/state.rs`

**Step 1: Add import at top of file**

After the existing imports (around line 11), add:

```rust
use crate::render::ui::area_banner::AreaBanner;
```

**Step 2: Add field to GameState struct**

In the `GameState` struct (around line 514), add after `last_portal_check_pos`:

```rust
    /// Area banner for displaying location names during transitions
    pub area_banner: AreaBanner,
```

**Step 3: Initialize in GameState::new()**

In `GameState::new()` (around line 628), add before the closing brace:

```rust
            area_banner: AreaBanner::default(),
```

**Step 4: Update banner in GameState::update()**

In `GameState::update()` (around line 722, after cleaning up announcements), add:

```rust
        // Update area banner timer
        self.area_banner.update(delta);
```

**Step 5: Commit**

```bash
git add client/src/game/state.rs
git commit -m "feat(client): add area banner to game state"
```

---

### Task 3: Add Interior Name to Server Protocol

**Files:**
- Modify: `rust-server/src/protocol.rs`

**Step 1: Add name field to InteriorData**

In `ServerMessage::InteriorData` (around line 329), add after `map_id`:

```rust
        name: String,
```

**Step 2: Update encoding**

In the encoding section for `InteriorData` (around line 1509), add after the `mapId` line:

```rust
            map.push((Value::String("name".into()), Value::String(name.clone().into())));
```

**Step 3: Commit**

```bash
git add rust-server/src/protocol.rs
git commit -m "feat(server): add name field to InteriorData message"
```

---

### Task 4: Send Interior Name from Server

**Files:**
- Modify: `rust-server/src/instance.rs` (or wherever InteriorData is constructed)

**Step 1: Find where InteriorData is sent**

Search for `InteriorData` construction and add the name field from the interior definition.

```bash
rg "InteriorData" rust-server/src --type rust
```

**Step 2: Add name to InteriorData construction**

When constructing `ServerMessage::InteriorData`, include the interior's name:

```rust
name: interior_def.name.clone(),
```

**Step 3: Commit**

```bash
git add rust-server/src/instance.rs
git commit -m "feat(server): include interior name in InteriorData message"
```

---

### Task 5: Trigger Banner on Map Transitions (Client)

**Files:**
- Modify: `client/src/network/client.rs`

**Step 1: Add import for OVERWORLD_NAME**

At the top with other imports, add:

```rust
use crate::render::ui::area_banner::OVERWORLD_NAME;
```

**Step 2: Trigger banner on interiorData**

In the `interiorData` handler (around line 1676), after parsing the map data, extract and use the name:

```rust
                    // Extract interior name (fallback to map_id if missing)
                    let name = extract_string(value, "name").unwrap_or(map_id.clone());

                    // Trigger area banner
                    state.area_banner.show(&name);
```

**Step 3: Trigger banner on mapTransition to overworld**

In the `mapTransition` handler (around line 1643), inside the `if map_type == "overworld"` block, add before or after clearing the interior state:

```rust
                        // Trigger area banner for overworld
                        state.area_banner.show(OVERWORLD_NAME);
```

**Step 4: Commit**

```bash
git add client/src/network/client.rs
git commit -m "feat(client): trigger area banner on map transitions"
```

---

### Task 6: Render the Area Banner

**Files:**
- Modify: `client/src/render/ui/area_banner.rs`
- Modify: `client/src/render/renderer.rs`

**Step 1: Add render method to AreaBanner**

In `area_banner.rs`, add the rendering implementation. First add imports at the top:

```rust
use macroquad::prelude::*;
use crate::render::Renderer;
```

Then add an impl block for Renderer:

```rust
impl Renderer {
    /// Render the area banner (called from main render loop)
    pub fn render_area_banner(&self, text: &str, opacity: f32) {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // Colors
        let text_color = Color::new(0.96, 0.94, 0.88, opacity); // Off-white/cream
        let flourish_color = Color::new(0.96, 0.94, 0.88, opacity * 0.7);
        let shadow_color = Color::new(0.1, 0.08, 0.05, opacity * 0.5);
        let bg_color = Color::new(0.0, 0.0, 0.0, opacity * 0.3);

        // Position: 18% down from top
        let banner_y = screen_h * 0.18;

        // Measure text
        let font_size = 28.0;
        let text_dims = self.measure_text_sharp(text, font_size);

        // Banner dimensions
        let padding_x = 40.0;
        let padding_y = 16.0;
        let banner_width = text_dims.width + padding_x * 2.0;
        let banner_height = text_dims.height + padding_y * 2.0;
        let banner_x = (screen_w - banner_width) / 2.0;

        // Draw semi-transparent background
        draw_rectangle(
            banner_x,
            banner_y - padding_y,
            banner_width,
            banner_height,
            bg_color,
        );

        // Draw flourishes (decorative lines)
        let flourish_width = text_dims.width * 0.8;
        let flourish_x = (screen_w - flourish_width) / 2.0;

        // Top flourish (thicker)
        let top_y = banner_y - 4.0;
        draw_line(flourish_x, top_y, flourish_x + flourish_width, top_y, 2.0, flourish_color);

        // Bottom flourish (thinner, slightly shorter)
        let bottom_flourish_width = flourish_width * 0.9;
        let bottom_flourish_x = (screen_w - bottom_flourish_width) / 2.0;
        let bottom_y = banner_y + text_dims.height + 8.0;
        draw_line(bottom_flourish_x, bottom_y, bottom_flourish_x + bottom_flourish_width, bottom_y, 1.0, flourish_color);

        // Draw text shadow
        let text_x = (screen_w - text_dims.width) / 2.0;
        let text_y = banner_y + text_dims.height * 0.8;
        self.draw_text_sharp(text, text_x + 2.0, text_y + 2.0, font_size, shadow_color);

        // Draw text
        self.draw_text_sharp(text, text_x, text_y, font_size, text_color);
    }
}
```

**Step 2: Call render from render_ui**

In `client/src/render/renderer.rs`, in the `render_ui` method (around line 3121), add near the end (before the chat input section):

```rust
        // Area banner (location name during transitions)
        if state.area_banner.is_visible() {
            self.render_area_banner(&state.area_banner.text, state.area_banner.opacity());
        }
```

**Step 3: Commit**

```bash
git add client/src/render/ui/area_banner.rs client/src/render/renderer.rs
git commit -m "feat(client): render area banner with flourishes"
```

---

### Task 7: Test and Polish

**Step 1: Build and run**

```bash
cd rust-server && cargo build
cd ../client && cargo build --target wasm32-unknown-unknown
```

**Step 2: Test transitions**

- Enter an interior (should show interior name like "Old House")
- Exit back to overworld (should show "Verdant Fields")
- Verify timing feels right (fade in, hold, fade out)

**Step 3: Adjust if needed**

If the banner feels too slow/fast, adjust the constants in `area_banner.rs`:
- `FADE_IN_DURATION`
- `HOLD_DURATION`
- `FADE_OUT_DURATION`

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: area banner displays location names on map transitions"
```
