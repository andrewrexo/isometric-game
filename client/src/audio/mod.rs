use macroquad::audio::{load_sound, play_sound, stop_sound, set_sound_volume, PlaySoundParams, Sound};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::util::asset_path;

#[derive(Serialize, Deserialize, Clone)]
pub struct AudioSettings {
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub muted: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            music_volume: 0.7,
            sfx_volume: 0.7,
            muted: false,
        }
    }
}

pub struct AudioManager {
    current_music: Option<Sound>,
    settings: AudioSettings,
    /// Preloaded sound effects by name
    sfx: HashMap<String, Sound>,
    /// Sword attack sounds for random selection
    sword_sounds: Vec<Sound>,
}

impl AudioManager {
    pub async fn new() -> Self {
        let settings = load_settings();
        let mut manager = Self {
            current_music: None,
            settings,
            sfx: HashMap::new(),
            sword_sounds: Vec::new(),
        };
        manager.preload_sfx().await;
        manager
    }

    /// Preload all sound effects at startup
    async fn preload_sfx(&mut self) {
        let sfx_files = [
            ("enter", "assets/audio/sfx/menu/enter.wav"),
            ("item_grab", "assets/audio/sfx/menu/item_grab.wav"),
            ("item_put", "assets/audio/sfx/menu/item_put.wav"),
            ("send_message", "assets/audio/sfx/menu/send_message.wav"),
            ("buy", "assets/audio/sfx/menu/buy.wav"),
            ("login_success", "assets/audio/sfx/menu/login_success.wav"),
            ("level_up", "assets/audio/sfx/menu/level_up.wav"),
            ("unarmed", "assets/audio/sfx/attack/unarmed.wav"),
        ];

        for (name, path) in sfx_files {
            match load_sound(&asset_path(path)).await {
                Ok(sound) => {
                    self.sfx.insert(name.to_string(), sound);
                    log::debug!("Loaded SFX: {}", name);
                }
                Err(e) => {
                    log::warn!("Failed to load SFX '{}': {:?}", path, e);
                }
            }
        }

        // Load sword attack sounds for random selection
        for i in 1..=4 {
            let path = asset_path(&format!("assets/audio/sfx/attack/sword_{}.wav", i));
            match load_sound(&path).await {
                Ok(sound) => {
                    self.sword_sounds.push(sound);
                    log::debug!("Loaded sword sound {}", i);
                }
                Err(e) => {
                    log::warn!("Failed to load sword sound '{}': {:?}", path, e);
                }
            }
        }

        log::info!("Loaded {} SFX, {} sword sounds", self.sfx.len(), self.sword_sounds.len());
    }

    /// Play a sound effect by name
    pub fn play_sfx(&self, name: &str) {
        if let Some(sound) = self.sfx.get(name) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: self.effective_sfx_volume(),
                },
            );
        } else {
            log::warn!("Unknown SFX: {}", name);
        }
    }

    /// Play attack sound - random sword sound if armed, unarmed sound if not
    pub fn play_attack_sound(&self, has_weapon: bool) {
        if has_weapon && !self.sword_sounds.is_empty() {
            // Pick a random sword sound
            let idx = macroquad::rand::gen_range(0, self.sword_sounds.len());
            play_sound(
                &self.sword_sounds[idx],
                PlaySoundParams {
                    looped: false,
                    volume: self.effective_sfx_volume(),
                },
            );
        } else {
            self.play_sfx("unarmed");
        }
    }

    pub async fn play_music(&mut self, path: &str) {
        // Stop any currently playing music
        self.stop_music();

        let actual_path = asset_path(path);
        log::info!("Loading music from: {}", actual_path);
        match load_sound(&actual_path).await {
            Ok(sound) => {
                let volume = self.effective_music_volume();
                log::info!("Playing music with volume: {} (muted: {})", volume, self.settings.muted);
                play_sound(
                    &sound,
                    PlaySoundParams {
                        looped: true,
                        volume,
                    },
                );
                self.current_music = Some(sound);
                log::info!("Music started successfully");
            }
            Err(e) => {
                log::error!("Failed to load music '{}': {:?}", path, e);
            }
        }
    }

    pub fn stop_music(&mut self) {
        if let Some(sound) = self.current_music.take() {
            stop_sound(&sound);
        }
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        self.settings.music_volume = volume.clamp(0.0, 1.0);
        self.apply_music_volume();
        save_settings(&self.settings);
    }

    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.settings.sfx_volume = volume.clamp(0.0, 1.0);
        save_settings(&self.settings);
    }

    pub fn toggle_mute(&mut self) {
        self.settings.muted = !self.settings.muted;
        self.apply_music_volume();
        save_settings(&self.settings);
    }

    pub fn effective_music_volume(&self) -> f32 {
        if self.settings.muted {
            0.0
        } else {
            self.settings.music_volume
        }
    }

    pub fn effective_sfx_volume(&self) -> f32 {
        if self.settings.muted {
            0.0
        } else {
            self.settings.sfx_volume
        }
    }

    pub fn music_volume(&self) -> f32 {
        self.settings.music_volume
    }

    pub fn sfx_volume(&self) -> f32 {
        self.settings.sfx_volume
    }

    pub fn is_muted(&self) -> bool {
        self.settings.muted
    }

    fn apply_music_volume(&self) {
        if let Some(ref sound) = self.current_music {
            set_sound_volume(sound, self.effective_music_volume());
        }
    }
}

// Platform-specific settings persistence

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn settings_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("new-aeven").join("audio.toml"))
}

#[cfg(target_os = "android")]
fn settings_path() -> Option<std::path::PathBuf> {
    // On Android, we use a simple path in the app's internal storage
    // The actual path will be relative to where the app runs
    Some(std::path::PathBuf::from("audio_settings.toml"))
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn load_settings() -> AudioSettings {
    let Some(path) = settings_path() else {
        return AudioSettings::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => AudioSettings::default(),
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn save_settings(settings: &AudioSettings) {
    let Some(path) = settings_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(contents) = toml::to_string_pretty(settings) {
        let _ = std::fs::write(&path, contents);
    }
}

#[cfg(target_os = "android")]
fn load_settings() -> AudioSettings {
    // On Android, settings file is in the app's current directory
    let Some(path) = settings_path() else {
        return AudioSettings::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => AudioSettings::default(),
    }
}

#[cfg(target_os = "android")]
fn save_settings(settings: &AudioSettings) {
    let Some(path) = settings_path() else {
        return;
    };

    if let Ok(contents) = toml::to_string_pretty(settings) {
        let _ = std::fs::write(&path, contents);
    }
}

#[cfg(target_arch = "wasm32")]
fn load_settings() -> AudioSettings {
    quad_storage::STORAGE
        .lock()
        .expect("storage lock")
        .get("audio_settings")
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn save_settings(settings: &AudioSettings) {
    if let Ok(json) = serde_json::to_string(settings) {
        quad_storage::STORAGE
            .lock()
            .expect("storage lock")
            .set("audio_settings", &json);
    }
}
