# Background Music System Design

## Overview

Add background music that plays on loop after login, with volume/mute controls in the ESC menu. Settings persist across sessions.

## AudioManager Structure

```rust
// client/src/audio/mod.rs

pub struct AudioManager {
    current_music: Option<Sound>,
    volume: f32,           // 0.0 to 1.0
    muted: bool,
}

impl AudioManager {
    pub async fn new() -> Self;                      // Load settings from storage
    pub async fn play_music(&mut self, path: &str);  // Load and loop track
    pub fn stop_music(&mut self);
    pub fn set_volume(&mut self, volume: f32);       // Also saves to storage
    pub fn toggle_mute(&mut self);                   // Also saves to storage
    pub fn effective_volume(&self) -> f32;           // Returns 0.0 if muted
}
```

Uses `macroquad::audio::Sound` for playback with `PlaySoundParams { looped: true, volume }`.

## Settings Persistence

```rust
// client/src/audio/settings.rs

#[derive(Serialize, Deserialize)]
pub struct AudioSettings {
    pub volume: f32,
    pub muted: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { volume: 0.7, muted: false }
    }
}
```

- **Native:** `~/.config/new-aeven/audio.toml`
- **WASM:** localStorage key `"audio_settings"` (JSON)

Settings load on `AudioManager::new()`, save immediately on volume/mute changes.

## Integration Points

- **AudioManager location:** Field on `Renderer`
- **Music starts:** When entering `AppState::Playing` (main.rs ~line 152-157)
- **Music stops:** On disconnect
- **Volume changes:** ESC menu calls `AudioManager` methods directly

## ESC Menu UI

Add "Music" section between Camera Zoom and Controls:

```
┌─────────────────────┐
│       MENU          │
├─────────────────────┤
│ Camera Zoom         │
│  [1x]  [2x]         │
│                     │
│ Music               │
│  [━━━━━━━●━━] 70%   │  <- Volume slider
│  [Mute]             │  <- Mute toggle button
│                     │
│ Controls            │
│  WASD - Move        │
│  ...                │
└─────────────────────┘
```

New `UiElementId` variants: `EscapeMenuVolumeSlider`, `EscapeMenuMuteToggle`

## Future Extension

Per-area music: Call `play_music("assets/audio/<area>.ogg")` when player enters a new area. The current design supports this with no changes needed.
