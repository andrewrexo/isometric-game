//! Utility functions shared across the codebase

use macroquad::prelude::*;
use macroquad::file::load_file;
use std::collections::HashMap;
use std::cell::Cell;
use crate::mobile_scale::VIRTUAL_WIDTH;

// Cache for virtual screen size to avoid recalculating hundreds of times per frame
thread_local! {
    static CACHED_VIRTUAL_SIZE: Cell<(f32, f32, f64)> = Cell::new((0.0, 0.0, -1.0));
}

/// Get virtual screen dimensions for layout (cached per frame)
/// On Android, returns the virtual resolution used for scaling
/// On desktop, returns the actual screen dimensions
pub fn virtual_screen_size() -> (f32, f32) {
    let current_time = get_time();

    // Check cache - invalidate if more than 1/120th of a second old (handles 120fps)
    let cached = CACHED_VIRTUAL_SIZE.with(|c| c.get());
    if current_time - cached.2 < 0.008 {
        return (cached.0, cached.1);
    }

    // Calculate fresh values
    #[cfg(target_os = "android")]
    let result = {
        let screen_w = screen_width();
        let screen_h = screen_height();
        let aspect = screen_h / screen_w;
        let virtual_height = (VIRTUAL_WIDTH * aspect).round();
        (VIRTUAL_WIDTH, virtual_height)
    };

    #[cfg(not(target_os = "android"))]
    let result = (screen_width(), screen_height());

    // Update cache
    CACHED_VIRTUAL_SIZE.with(|c| c.set((result.0, result.1, current_time)));

    result
}

/// Convert an asset path to the correct format for the current platform.
/// On desktop: paths are relative to the working directory (e.g., "assets/sprites/tiles.png")
/// On Android: paths are relative to the APK's assets folder (e.g., "sprites/tiles.png")
pub fn asset_path(path: &str) -> String {
    #[cfg(target_os = "android")]
    {
        // On Android, strip the "assets/" prefix since files are loaded from the APK's assets folder
        if let Some(stripped) = path.strip_prefix("assets/") {
            stripped.to_string()
        } else {
            path.to_string()
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        path.to_string()
    }
}

/// Sprite manifest for Android (where we can't scan directories)
#[derive(serde::Deserialize, Default)]
pub struct SpriteManifest {
    #[serde(default)]
    pub enemies: Vec<String>,
    #[serde(default)]
    pub equipment: Vec<String>,
    #[serde(default)]
    pub weapons: Vec<String>,
    #[serde(default)]
    pub inventory: Vec<String>,
    #[serde(default)]
    pub objects: Vec<String>,
    #[serde(default)]
    pub walls: Vec<String>,
}

impl SpriteManifest {
    /// Load the sprite manifest from the assets folder
    pub async fn load() -> Self {
        let path = asset_path("assets/sprite_manifest.json");
        match load_file(&path).await {
            Ok(data) => {
                match serde_json::from_slice(&data) {
                    Ok(manifest) => {
                        log::info!("Loaded sprite manifest");
                        manifest
                    }
                    Err(e) => {
                        log::warn!("Failed to parse sprite manifest: {}", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to load sprite manifest: {}", e);
                Self::default()
            }
        }
    }
}

/// Load sprites from a directory (desktop) or manifest (Android)
pub async fn load_sprites_from_dir_or_manifest(
    dir_path: &str,
    manifest_items: &[String],
    base_path_for_manifest: &str,
) -> HashMap<String, Texture2D> {
    let mut sprites = HashMap::new();

    #[cfg(target_os = "android")]
    {
        // On Android, use the manifest
        for item in manifest_items {
            // For equipment, the manifest includes subdirectory (e.g., "equipment/body/admin_robes")
            // Extract just the filename for the key
            let key = item.rsplit('/').next().unwrap_or(item).to_string();
            let path = asset_path(&format!("{}/{}.png", base_path_for_manifest, item));

            match load_texture(&path).await {
                Ok(tex) => {
                    tex.set_filter(FilterMode::Nearest);
                    log::debug!("Loaded sprite: {}", key);
                    sprites.insert(key, tex);
                }
                Err(e) => {
                    log::warn!("Failed to load sprite {}: {}", path, e);
                }
            }
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        // On desktop, scan the directory
        let _ = manifest_items; // Unused on desktop
        let _ = base_path_for_manifest;

        fn scan_dir(dir: &str, sprites: &mut Vec<(String, String)>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        scan_dir(&path.to_string_lossy(), sprites);
                    } else if path.extension().map_or(false, |ext| ext == "png") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            sprites.push((stem.to_string(), path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }

        let mut found = Vec::new();
        scan_dir(dir_path, &mut found);

        for (key, path) in found {
            match load_texture(&path).await {
                Ok(tex) => {
                    tex.set_filter(FilterMode::Nearest);
                    log::debug!("Loaded sprite: {}", key);
                    sprites.insert(key, tex);
                }
                Err(e) => {
                    log::warn!("Failed to load sprite {}: {}", path, e);
                }
            }
        }
    }

    sprites
}
