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

/// Rectangle within a sprite atlas
#[derive(serde::Deserialize, Clone, Debug)]
pub struct SpriteRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Atlas info from the manifest: file path + sprite rectangles
#[derive(serde::Deserialize, Clone, Debug)]
pub struct SpriteAtlasInfo {
    pub file: String,
    pub sprites: HashMap<String, SpriteRect>,
}

/// Sprite manifest for Android/WASM (where we can't scan directories)
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
    #[serde(default)]
    pub objects_atlas: Option<SpriteAtlasInfo>,
    #[serde(default)]
    pub walls_atlas: Option<SpriteAtlasInfo>,
    #[serde(default)]
    pub inventory_atlas: Option<SpriteAtlasInfo>,
}

impl SpriteManifest {
    /// Log to browser console (WASM only)
    #[cfg(target_arch = "wasm32")]
    fn console_log(msg: &str) {
        use sapp_jsutils::JsObject;
        extern "C" { fn console_log(msg: JsObject); }
        let js_msg = JsObject::string(msg);
        unsafe { console_log(js_msg); }
    }

    /// Load the sprite manifest from the assets folder
    pub async fn load() -> Self {
        let path = asset_path("assets/sprite_manifest.json");
        match load_file(&path).await {
            Ok(data) => {
                #[cfg(target_arch = "wasm32")]
                {
                    let preview = String::from_utf8_lossy(&data[..200.min(data.len())]);
                    Self::console_log(&format!("MANIFEST: loaded {} bytes, starts with: {}", data.len(), preview));

                    // Check if raw JSON contains atlas key
                    let raw = String::from_utf8_lossy(&data);
                    Self::console_log(&format!("MANIFEST: contains objects_atlas: {}", raw.contains("objects_atlas")));
                }
                match serde_json::from_slice(&data) {
                    Ok(manifest) => {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let m: &SpriteManifest = &manifest;
                            Self::console_log(&format!("MANIFEST: parsed OK, objects_atlas={}, objects={}", m.objects_atlas.is_some(), m.objects.len()));
                        }
                        manifest
                    }
                    Err(e) => {
                        #[cfg(target_arch = "wasm32")]
                        Self::console_log(&format!("MANIFEST: parse FAILED: {}", e));
                        log::warn!("Failed to parse sprite manifest: {}", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                #[cfg(target_arch = "wasm32")]
                Self::console_log(&format!("MANIFEST: load FAILED: {}", e));
                log::warn!("Failed to load sprite manifest: {}", e);
                Self::default()
            }
        }
    }
}

/// Progress callback for sprite loading: (loaded_count, total_count)
pub type LoadProgress = dyn FnMut(usize, usize);

/// Load sprites from a directory (desktop) or manifest (Android/WASM).
/// Calls `on_progress(loaded, total)` after each sprite is loaded.
pub async fn load_sprites_with_progress(
    dir_path: &str,
    manifest_items: &[String],
    base_path_for_manifest: &str,
    on_progress: &mut LoadProgress,
) -> HashMap<String, Texture2D> {
    let mut sprites = HashMap::new();

    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    {
        let total = manifest_items.len();
        // On Android/WASM, use the manifest
        for (i, item) in manifest_items.iter().enumerate() {
            let key = item.rsplit('/').next().unwrap_or(item).to_string();
            let path = asset_path(&format!("{}/{}.png", base_path_for_manifest, item));

            match load_texture(&path).await {
                Ok(tex) => {
                    tex.set_filter(FilterMode::Nearest);
                    sprites.insert(key, tex);
                }
                Err(e) => {
                    log::warn!("Failed to load sprite {}: {}", path, e);
                }
            }
            on_progress(i + 1, total);
        }
    }

    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    {
        let _ = manifest_items;
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

        let total = found.len();
        for (i, (key, path)) in found.into_iter().enumerate() {
            match load_texture(&path).await {
                Ok(tex) => {
                    tex.set_filter(FilterMode::Nearest);
                    sprites.insert(key, tex);
                }
                Err(e) => {
                    log::warn!("Failed to load sprite {}: {}", path, e);
                }
            }
            on_progress(i + 1, total);
        }
    }

    sprites
}

/// Load sprites without progress (convenience wrapper)
pub async fn load_sprites_from_dir_or_manifest(
    dir_path: &str,
    manifest_items: &[String],
    base_path_for_manifest: &str,
) -> HashMap<String, Texture2D> {
    load_sprites_with_progress(dir_path, manifest_items, base_path_for_manifest, &mut |_, _| {}).await
}
