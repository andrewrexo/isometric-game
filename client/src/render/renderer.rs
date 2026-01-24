use macroquad::prelude::*;
use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::material::{load_material, gl_use_material, gl_use_default_material, Material, MaterialParams};
use macroquad::miniquad::UniformDesc;
use macroquad::miniquad::ShaderSource;
use std::collections::HashMap;
use crate::game::{GameState, Player, Camera, ConnectionStatus, LayerType, GroundItem, ChunkLayerType, CHUNK_SIZE, MapObject, ChatChannel, Direction};
use crate::game::npc::{Npc, NpcState};
use crate::game::tilemap::get_tile_color;
use crate::ui::UiLayout;
use super::ui::common::{SlotState, CORNER_ACCENT_SIZE};
use super::isometric::{world_to_screen, TILE_WIDTH, TILE_HEIGHT, calculate_depth};
use super::animation::{SPRITE_WIDTH, SPRITE_HEIGHT, WEAPON_SPRITE_WIDTH, WEAPON_SPRITE_HEIGHT, BOOT_SPRITE_WIDTH, BOOT_SPRITE_HEIGHT, BODY_ARMOR_SPRITE_WIDTH, BODY_ARMOR_SPRITE_HEIGHT, HEAD_SPRITE_WIDTH, HEAD_SPRITE_HEIGHT, NpcAnimation, get_weapon_frame, get_weapon_offset, get_boot_frame, get_boot_offset, get_body_armor_frame, get_body_armor_offset, get_head_frame, get_head_offset, AnimationState};
use super::font::BitmapFont;
use super::shaders;

/// Timing data from a render pass
#[derive(Default, Clone)]
pub struct RenderTimings {
    pub ground_ms: f64,
    pub entities_ms: f64,
    pub overhead_ms: f64,
    pub effects_ms: f64,
    pub ui_ms: f64,
    pub total_ms: f64,
}

/// Tileset configuration
const TILESET_TILE_WIDTH: f32 = 64.0;
const TILESET_TILE_HEIGHT: f32 = 32.0;
const TILESET_COLUMNS: u32 = 32;

/// Available player appearance options
pub const GENDERS: &[&str] = &["male", "female"];
pub const SKINS: &[&str] = &["tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"];

/// Objects tileset firstgid from objects.tsx (used to map gids to sprite filenames)
const OBJECTS_FIRSTGID: u32 = 1249;
/// Offset to convert local tile id to sprite filename number
const OBJECTS_ID_OFFSET: u32 = 87;

// ============================================================================
// Inventory UI Color Palette - Medieval Fantasy Theme
// ============================================================================

// Panel backgrounds (darker to lighter for depth)
const PANEL_BG_DARK: Color = Color::new(0.071, 0.071, 0.094, 0.961);    // rgba(18, 18, 24, 245)
const PANEL_BG_MID: Color = Color::new(0.110, 0.110, 0.149, 1.0);       // rgba(28, 28, 38, 255)

// Frame/Border colors (bronze/gold medieval theme)
const FRAME_OUTER: Color = Color::new(0.322, 0.243, 0.165, 1.0);        // rgba(82, 62, 42, 255)
const FRAME_MID: Color = Color::new(0.557, 0.424, 0.267, 1.0);          // rgba(142, 108, 68, 255)
const FRAME_INNER: Color = Color::new(0.729, 0.580, 0.361, 1.0);        // rgba(186, 148, 92, 255)
const FRAME_ACCENT: Color = Color::new(0.855, 0.698, 0.424, 1.0);       // rgba(218, 178, 108, 255)

// Slot colors
const SLOT_BG_EMPTY: Color = Color::new(0.086, 0.086, 0.118, 1.0);      // rgba(22, 22, 30, 255)
const SLOT_BG_FILLED: Color = Color::new(0.125, 0.125, 0.173, 1.0);     // rgba(32, 32, 44, 255)
const SLOT_INNER_SHADOW: Color = Color::new(0.047, 0.047, 0.063, 1.0);  // rgba(12, 12, 16, 255)
const SLOT_HIGHLIGHT: Color = Color::new(0.188, 0.188, 0.251, 1.0);     // rgba(48, 48, 64, 255)
const SLOT_BORDER: Color = Color::new(0.227, 0.212, 0.188, 1.0);        // rgba(58, 54, 48, 255)

// Hover/Selection states
const SLOT_HOVER_BG: Color = Color::new(0.188, 0.188, 0.282, 1.0);      // rgba(48, 48, 72, 255)
const SLOT_HOVER_BORDER: Color = Color::new(0.659, 0.580, 0.424, 1.0);  // rgba(168, 148, 108, 255)
const SLOT_SELECTED_BORDER: Color = Color::new(0.855, 0.737, 0.502, 1.0); // rgba(218, 188, 128, 255)
const SLOT_DRAG_SOURCE: Color = Color::new(0.314, 0.392, 0.627, 0.706); // rgba(80, 100, 160, 180)

// Text colors (used by stats panel)
const TEXT_TITLE: Color = Color::new(0.855, 0.737, 0.502, 1.0);         // rgba(218, 188, 128, 255)
const TEXT_NORMAL: Color = Color::new(0.824, 0.824, 0.855, 1.0);        // rgba(210, 210, 218, 255)
const TEXT_DIM: Color = Color::new(0.502, 0.502, 0.541, 1.0);           // rgba(128, 128, 138, 255)

// Layout constant for draw_panel_frame helper
const FRAME_THICKNESS: f32 = 4.0;

// ============================================================================
// Health Bar Colors - Ornate Medieval Style
// ============================================================================

// Health bar frame (bronze-tinted dark metal)
const HEALTHBAR_FRAME_DARK: Color = Color::new(0.18, 0.14, 0.10, 1.0);   // Dark bronze outline
const HEALTHBAR_FRAME_MID: Color = Color::new(0.35, 0.27, 0.18, 1.0);    // Mid bronze
const HEALTHBAR_FRAME_LIGHT: Color = Color::new(0.55, 0.43, 0.28, 1.0);  // Light bronze
const HEALTHBAR_FRAME_ACCENT: Color = Color::new(0.72, 0.58, 0.38, 1.0); // Gold highlight

// Health bar background (recessed dark)
const HEALTHBAR_BG_OUTER: Color = Color::new(0.04, 0.04, 0.05, 1.0);     // Outer shadow
const HEALTHBAR_BG_INNER: Color = Color::new(0.08, 0.07, 0.09, 1.0);     // Inner dark

// Health colors - rich jewel tones (dark/mid/light for gradient effect)
const HEALTH_GREEN_DARK: Color = Color::new(0.12, 0.45, 0.22, 1.0);      // Emerald base
const HEALTH_GREEN_MID: Color = Color::new(0.20, 0.62, 0.32, 1.0);       // Emerald bright
const HEALTH_GREEN_LIGHT: Color = Color::new(0.35, 0.78, 0.48, 1.0);     // Emerald highlight

const HEALTH_YELLOW_DARK: Color = Color::new(0.65, 0.45, 0.08, 1.0);     // Amber base
const HEALTH_YELLOW_MID: Color = Color::new(0.85, 0.62, 0.12, 1.0);      // Amber bright
const HEALTH_YELLOW_LIGHT: Color = Color::new(0.95, 0.78, 0.25, 1.0);    // Amber highlight

const HEALTH_RED_DARK: Color = Color::new(0.55, 0.12, 0.12, 1.0);        // Ruby base
const HEALTH_RED_MID: Color = Color::new(0.75, 0.18, 0.18, 1.0);         // Ruby bright
const HEALTH_RED_LIGHT: Color = Color::new(0.90, 0.35, 0.35, 1.0);       // Ruby highlight

pub struct Renderer {
    player_color: Color,
    local_player_color: Color,
    /// Loaded tileset texture
    tileset: Option<Texture2D>,
    /// Player sprite sheets by appearance key (e.g., "male_tan")
    player_sprites: HashMap<String, Texture2D>,
    /// Hair sprite sheets by style index (0, 1, 2)
    hair_sprites: HashMap<i32, Texture2D>,
    /// Equipment sprite sheets by item ID (e.g., "peasant_suit")
    equipment_sprites: HashMap<String, Texture2D>,
    /// Weapon sprite sheets by item ID (e.g., "goblin_axe")
    weapon_sprites: HashMap<String, Texture2D>,
    /// Item inventory sprites by item ID (sprite sheets with icon on left half)
    pub(crate) item_sprites: HashMap<String, Texture2D>,
    /// Map object sprites by filename number (e.g., "101" -> Texture2D)
    object_sprites: HashMap<String, Texture2D>,
    /// NPC sprites by entity type (e.g., "pig" -> Texture2D)
    npc_sprites: HashMap<String, Texture2D>,
    /// Multi-size pixel font for sharp text rendering at various sizes
    font: BitmapFont,
    /// Quest complete banner texture
    pub(crate) quest_complete_texture: Option<Texture2D>,
    /// Gold nugget icon for inventory
    pub(crate) gold_nugget_texture: Option<Texture2D>,
    /// Circular stone backdrop for shop item icons
    pub(crate) circular_stone_texture: Option<Texture2D>,
    /// Menu button icons (inventory, character, settings, skills, social)
    pub(crate) menu_button_icons: Option<Texture2D>,
    /// UI icons sprite sheet (24x24 icons in 10x10 grid)
    pub(crate) ui_icons: Option<Texture2D>,
    /// Small chat icon for quest giver name tags
    pub(crate) chat_small_icon: Option<Texture2D>,
    /// Small coin icon for merchant name tags
    pub(crate) coin_small_icon: Option<Texture2D>,
    /// Material for head+hair composite rendering (shader-based)
    head_hair_material: Option<Material>,
}

impl Renderer {
    pub async fn new() -> Self {
        // Try to load the tileset texture
        let tileset = match load_texture("assets/sprites/tiles.png").await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                log::info!("Loaded tileset: {}x{}", tex.width(), tex.height());
                Some(tex)
            }
            Err(e) => {
                log::warn!("Failed to load tileset: {}. Using fallback colors.", e);
                None
            }
        };

        // Load all player sprite sheets for each appearance combination
        let mut player_sprites = HashMap::new();
        for gender in GENDERS {
            for skin in SKINS {
                let key = format!("{}_{}", gender, skin);
                let path = format!("assets/sprites/players/player_{}_{}.png", gender, skin);
                match load_texture(&path).await {
                    Ok(tex) => {
                        tex.set_filter(FilterMode::Nearest);
                        log::debug!("Loaded player sprite: {}", key);
                        player_sprites.insert(key, tex);
                    }
                    Err(e) => {
                        log::warn!("Failed to load player sprite {}: {}", path, e);
                    }
                }
            }
        }
        log::info!("Loaded {} player sprite variants", player_sprites.len());

        // Load hair sprites from assets/sprites/hair/
        let mut hair_sprites = HashMap::new();
        for style in 0..3 {
            let path = format!("assets/sprites/hair/hair_{}.png", style);
            match load_texture(&path).await {
                Ok(tex) => {
                    tex.set_filter(FilterMode::Nearest);
                    log::info!("Loaded hair sprite: style {} ({}x{})", style, tex.width(), tex.height());
                    hair_sprites.insert(style, tex);
                }
                Err(e) => {
                    log::warn!("Failed to load hair sprite {}: {}", path, e);
                }
            }
        }
        log::info!("Loaded {} hair sprite variants", hair_sprites.len());

        // Load equipment sprites from assets/sprites/equipment/ (scan directory and subfolders)
        let mut equipment_sprites = HashMap::new();
        let mut equipment_dirs = vec![std::path::PathBuf::from("assets/sprites/equipment")];
        while let Some(dir) = equipment_dirs.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Add subdirectory to scan
                        equipment_dirs.push(path);
                    } else if path.extension().map_or(false, |ext| ext == "png") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let item_id = stem.to_string();
                            let path_str = path.to_string_lossy().to_string();

                            match load_texture(&path_str).await {
                                Ok(tex) => {
                                    tex.set_filter(FilterMode::Nearest);
                                    log::info!("Loaded equipment sprite: {} ({}x{})", item_id, tex.width(), tex.height());
                                    equipment_sprites.insert(item_id, tex);
                                }
                                Err(e) => {
                                    log::warn!("Failed to load equipment sprite {}: {}", path_str, e);
                                }
                            }
                        }
                    }
                }
            }
        }
        log::info!("Loaded {} equipment sprite variants", equipment_sprites.len());

        // Load weapon sprites from assets/sprites/weapons/ (scan directory)
        let mut weapon_sprites = HashMap::new();
        if let Ok(entries) = std::fs::read_dir("assets/sprites/weapons") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "png") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let item_id = stem.to_string();
                        let path_str = path.to_string_lossy().to_string();
                        match load_texture(&path_str).await {
                            Ok(tex) => {
                                tex.set_filter(FilterMode::Nearest);
                                log::info!("Loaded weapon sprite: {} ({}x{})", item_id, tex.width(), tex.height());
                                weapon_sprites.insert(item_id, tex);
                            }
                            Err(e) => {
                                log::warn!("Failed to load weapon sprite {}: {}", path_str, e);
                            }
                        }
                    }
                }
            }
        }
        log::info!("Loaded {} weapon sprite variants", weapon_sprites.len());

        // Load item inventory sprites from assets/sprites/inventory/ (scan directory)
        let mut item_sprites = HashMap::new();
        if let Ok(entries) = std::fs::read_dir("assets/sprites/inventory") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "png") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let item_id = stem.to_string();
                        let path_str = path.to_string_lossy().to_string();
                        match load_texture(&path_str).await {
                            Ok(tex) => {
                                tex.set_filter(FilterMode::Nearest);
                                log::info!("Loaded item sprite: {} ({}x{})", item_id, tex.width(), tex.height());
                                item_sprites.insert(item_id, tex);
                            }
                            Err(e) => {
                                log::warn!("Failed to load item sprite {}: {}", path_str, e);
                            }
                        }
                    }
                }
            }
        }
        log::info!("Loaded {} item sprite variants", item_sprites.len());

        // Load map object sprites from assets/sprites/objects/ (scan directory)
        let mut object_sprites = HashMap::new();
        if let Ok(entries) = std::fs::read_dir("assets/sprites/objects") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "png") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let sprite_key = stem.to_string();
                        let path_str = path.to_string_lossy().to_string();
                        match load_texture(&path_str).await {
                            Ok(tex) => {
                                tex.set_filter(FilterMode::Nearest);
                                log::debug!("Loaded object sprite: {} ({}x{})", sprite_key, tex.width(), tex.height());
                                object_sprites.insert(sprite_key, tex);
                            }
                            Err(e) => {
                                log::warn!("Failed to load object sprite {}: {}", path_str, e);
                            }
                        }
                    }
                }
            }
        }
        log::info!("Loaded {} object sprite variants", object_sprites.len());

        // Load NPC sprites from assets/sprites/enemies/ (scan directory)
        let mut npc_sprites = HashMap::new();
        if let Ok(entries) = std::fs::read_dir("assets/sprites/enemies") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "png") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let entity_type = stem.to_string();
                        let path_str = path.to_string_lossy().to_string();
                        match load_texture(&path_str).await {
                            Ok(tex) => {
                                tex.set_filter(FilterMode::Nearest);
                                log::info!("Loaded NPC sprite: {} ({}x{}, frame: {}x{})",
                                    entity_type, tex.width(), tex.height(),
                                    tex.width() / 16.0, tex.height());
                                npc_sprites.insert(entity_type, tex);
                            }
                            Err(e) => {
                                log::warn!("Failed to load NPC sprite {}: {}", path_str, e);
                            }
                        }
                    }
                }
            }
        }
        log::info!("Loaded {} NPC sprite variants", npc_sprites.len());

        // Load monogram pixel font at multiple sizes for crisp rendering
        let font = BitmapFont::load_or_default("assets/fonts/monogram/ttf/monogram-extended.ttf").await;
        if font.is_loaded() {
            log::info!("Loaded monogram bitmap font at multiple sizes");
        } else {
            log::warn!("Failed to load monogram font, using default");
        }

        // Load quest complete banner texture
        let quest_complete_texture = match load_texture("assets/ui/quest_complete.png").await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                log::info!("Loaded quest complete texture: {}x{}", tex.width(), tex.height());
                Some(tex)
            }
            Err(e) => {
                log::warn!("Failed to load quest complete texture: {}", e);
                None
            }
        };

        // Load gold nugget icon for inventory
        let gold_nugget_texture = match load_texture("assets/ui/gold_nugget.png").await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                log::info!("Loaded gold nugget texture: {}x{}", tex.width(), tex.height());
                Some(tex)
            }
            Err(e) => {
                log::warn!("Failed to load gold nugget texture: {}", e);
                None
            }
        };

        // Load circular stone backdrop for shop item icons
        let circular_stone_texture = match load_texture("assets/ui/circular_stone.png").await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                log::info!("Loaded circular stone texture: {}x{}", tex.width(), tex.height());
                Some(tex)
            }
            Err(e) => {
                log::warn!("Failed to load circular stone texture: {}", e);
                None
            }
        };

        // Load menu button icons sprite sheet
        let menu_button_icons = match load_texture("assets/ui/background_icons.png").await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                log::info!("Loaded menu button icons: {}x{}", tex.width(), tex.height());
                Some(tex)
            }
            Err(e) => {
                log::warn!("Failed to load menu button icons: {}", e);
                None
            }
        };

        // Load UI icons sprite sheet (24x24 icons in 10x10 grid)
        let ui_icons = match load_texture("assets/ui/ui_icons.png").await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                log::info!("Loaded UI icons: {}x{}", tex.width(), tex.height());
                Some(tex)
            }
            Err(e) => {
                log::warn!("Failed to load UI icons: {}", e);
                None
            }
        };

        // Load small icons for NPC name tags
        let chat_small_icon = match load_texture("assets/ui/chat_small.png").await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                Some(tex)
            }
            Err(e) => {
                log::warn!("Failed to load chat_small icon: {}", e);
                None
            }
        };

        let coin_small_icon = match load_texture("assets/ui/coin_small.png").await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                Some(tex)
            }
            Err(e) => {
                log::warn!("Failed to load coin_small icon: {}", e);
                None
            }
        };

        // Load head+hair composite shader material
        let head_hair_material = match load_material(
            ShaderSource::Glsl {
                vertex: shaders::HEAD_HAIR_VERTEX,
                fragment: shaders::HEAD_HAIR_FRAGMENT,
            },
            MaterialParams {
                textures: vec!["HairTexture".to_string()],
                uniforms: vec![
                    UniformDesc::new("HairUvTransform", UniformType::Float4),
                    UniformDesc::new("Tint", UniformType::Float4),
                ],
                ..Default::default()
            },
        ) {
            Ok(mat) => {
                log::info!("Loaded head+hair composite shader");
                Some(mat)
            }
            Err(e) => {
                log::warn!("Failed to load head+hair shader: {}. Head equipment will render without hair masking.", e);
                None
            }
        };

        Self {
            player_color: Color::from_rgba(100, 150, 255, 255),
            local_player_color: Color::from_rgba(100, 255, 150, 255),
            tileset,
            player_sprites,
            hair_sprites,
            equipment_sprites,
            weapon_sprites,
            item_sprites,
            object_sprites,
            npc_sprites,
            font,
            quest_complete_texture,
            gold_nugget_texture,
            circular_stone_texture,
            menu_button_icons,
            ui_icons,
            chat_small_icon,
            coin_small_icon,
            head_hair_material,
        }
    }

    /// Get the sprite texture for a given player appearance
    fn get_player_sprite(&self, gender: &str, skin: &str) -> Option<&Texture2D> {
        let key = format!("{}_{}", gender, skin);
        self.player_sprites.get(&key)
            // Fallback to male_tan if sprite not found
            .or_else(|| self.player_sprites.get("male_tan"))
    }

    /// Get the sprite texture for a map object by its gid
    fn get_object_sprite(&self, gid: u32) -> Option<&Texture2D> {
        if gid < OBJECTS_FIRSTGID {
            return None;
        }
        let local_id = gid - OBJECTS_FIRSTGID;
        let sprite_number = local_id + OBJECTS_ID_OFFSET;
        let key = sprite_number.to_string();
        self.object_sprites.get(&key)
    }

    /// Draw text with pixel font for sharp rendering
    /// Uses multi-size bitmap font for crisp text at any size
    pub fn draw_text_sharp(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        // Round to integer pixels for crisp rendering
        self.font.draw_text(text, x.floor(), y.floor(), font_size, color);
    }

    /// Measure text with pixel font
    pub(crate) fn measure_text_sharp(&self, text: &str, font_size: f32) -> TextDimensions {
        self.font.measure_text(text, font_size)
    }

    /// Draw text with word wrapping to fit within max_width
    /// Returns the total height used
    pub(crate) fn draw_text_wrapped(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color, max_width: f32, line_height: f32) -> f32 {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut current_line = String::new();
        let mut current_y = y;
        let space_width = self.measure_text_sharp(" ", font_size).width;

        for word in words {
            let word_width = self.measure_text_sharp(word, font_size).width;
            let line_width = if current_line.is_empty() {
                word_width
            } else {
                self.measure_text_sharp(&current_line, font_size).width + space_width + word_width
            };

            if line_width > max_width && !current_line.is_empty() {
                // Draw current line and start new one
                self.draw_text_sharp(&current_line, x, current_y, font_size, color);
                current_y += line_height;
                current_line = word.to_string();
            } else {
                // Add word to current line
                if current_line.is_empty() {
                    current_line = word.to_string();
                } else {
                    current_line.push(' ');
                    current_line.push_str(word);
                }
            }
        }

        // Draw remaining line
        if !current_line.is_empty() {
            self.draw_text_sharp(&current_line, x, current_y, font_size, color);
            current_y += line_height;
        }

        current_y - y
    }

    /// Get the UV rect for a tile ID in the tileset
    /// Tiled uses 1-indexed tile IDs (0 = empty)
    fn get_tile_uv(&self, tile_id: u32) -> Option<Rect> {
        if tile_id == 0 {
            return None;
        }

        let tileset = self.tileset.as_ref()?;
        let id = tile_id - 1; // Convert to 0-indexed

        let col = id % TILESET_COLUMNS;
        let row = id / TILESET_COLUMNS;

        let x = col as f32 * TILESET_TILE_WIDTH;
        let y = row as f32 * TILESET_TILE_HEIGHT;

        Some(Rect::new(
            x / tileset.width(),
            y / tileset.height(),
            TILESET_TILE_WIDTH / tileset.width(),
            TILESET_TILE_HEIGHT / tileset.height(),
        ))
    }

    /// Draw a tile sprite from the tileset
    fn draw_tile_sprite(&self, screen_x: f32, screen_y: f32, tile_id: u32, zoom: f32) {
        let scaled_width = TILE_WIDTH * zoom;
        let scaled_height = TILE_HEIGHT * zoom;

        if let (Some(tileset), Some(uv)) = (&self.tileset, self.get_tile_uv(tile_id)) {
            // Center the tile on screen position
            let draw_x = screen_x - scaled_width / 2.0;
            let draw_y = screen_y - scaled_height / 2.0;

            draw_texture_ex(
                tileset,
                draw_x,
                draw_y,
                WHITE,
                DrawTextureParams {
                    source: Some(Rect::new(
                        uv.x * tileset.width(),
                        uv.y * tileset.height(),
                        TILESET_TILE_WIDTH,
                        TILESET_TILE_HEIGHT,
                    )),
                    dest_size: Some(Vec2::new(scaled_width, scaled_height)),
                    ..Default::default()
                },
            );
        } else {
            // Fallback to colored tile
            let color = get_tile_color(tile_id);
            self.draw_isometric_tile(screen_x, screen_y, color, zoom);
        }
    }

    pub fn render(&self, state: &GameState) -> (UiLayout, RenderTimings) {
        let render_start = get_time();
        let mut timings = RenderTimings::default();

        // 1. Render ground layer tiles
        let t0 = get_time();
        self.render_tilemap_layer(state, LayerType::Ground);

        // 1.5. Render hovered tile border if hovering over a tile
        if let Some((tile_x, tile_y)) = state.hovered_tile {
            self.render_tile_hover(tile_x, tile_y, &state.camera);
        }
        timings.ground_ms = (get_time() - t0) * 1000.0;

        // 2. Collect renderable items (players + NPCs + items + object tiles + map objects) for depth sorting
        let t1 = get_time();
        #[derive(Clone)]
        enum Renderable<'a> {
            Player(&'a Player, bool),
            Npc(&'a Npc),
            Item(&'a GroundItem),
            Tile { x: u32, y: u32, tile_id: u32 },
            ChunkObject(&'a MapObject),
        }

        // Pre-allocate with estimated capacity to reduce allocations
        let estimated_capacity = state.players.len() + state.npcs.len() + state.ground_items.len() + 100;
        let mut renderables: Vec<(f32, Renderable)> = Vec::with_capacity(estimated_capacity);

        // Add ground items (render below entities)
        for item in state.ground_items.values() {
            let depth = calculate_depth(item.x, item.y, 0); // Lower layer than entities
            renderables.push((depth, Renderable::Item(item)));
        }

        // Add players
        for player in state.players.values() {
            let is_local = state.local_player_id.as_ref() == Some(&player.id);
            let depth = calculate_depth(player.x, player.y, 1);
            renderables.push((depth, Renderable::Player(player, is_local)));
        }

        // Add NPCs
        for npc in state.npcs.values() {
            let depth = calculate_depth(npc.x, npc.y, 1);
            renderables.push((depth, Renderable::Npc(npc)));
        }

        // Add object layer tiles (trees, rocks, buildings) from static tilemap
        for layer in &state.tilemap.layers {
            if layer.layer_type == LayerType::Objects {
                for y in 0..state.tilemap.height {
                    for x in 0..state.tilemap.width {
                        let idx = (y * state.tilemap.width + x) as usize;
                        let tile_id = layer.tiles.get(idx).copied().unwrap_or(0);
                        if tile_id > 0 {
                            let depth = calculate_depth(x as f32, y as f32, 1);
                            renderables.push((depth, Renderable::Tile { x, y, tile_id }));
                        }
                    }
                }
            }
        }

        // Add map objects from loaded chunks (trees, rocks, decorations placed in Tiled)
        for chunk in state.chunk_manager.chunks().values() {
            for obj in &chunk.objects {
                // Depth is based on tile_y (bottom edge of object for proper sorting)
                let depth = calculate_depth(obj.tile_x as f32, obj.tile_y as f32, 1);
                renderables.push((depth, Renderable::ChunkObject(obj)));
            }
        }

        // Sort by depth (painter's algorithm)
        renderables.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // 3. Render sorted entities
        for (_, renderable) in renderables {
            match renderable {
                Renderable::Item(item) => {
                    self.render_ground_item(item, &state.camera, state);
                }
                Renderable::Player(player, is_local) => {
                    let is_selected = state.selected_entity_id.as_ref() == Some(&player.id);
                    let is_hovered = state.hovered_entity_id.as_ref() == Some(&player.id);
                    self.render_player(player, is_local, is_selected, is_hovered, &state.camera);
                }
                Renderable::Npc(npc) => {
                    let is_selected = state.selected_entity_id.as_ref() == Some(&npc.id);
                    let is_hovered = state.hovered_entity_id.as_ref() == Some(&npc.id);
                    self.render_npc(npc, is_selected, is_hovered, &state.camera);
                }
                Renderable::Tile { x, y, tile_id } => {
                    let (screen_x, screen_y) = world_to_screen(x as f32, y as f32, &state.camera);
                    self.draw_isometric_object(screen_x, screen_y, tile_id, state.camera.zoom);
                }
                Renderable::ChunkObject(obj) => {
                    self.render_map_object(obj, &state.camera);
                }
            }
        }

        // Render subtle local player silhouette (high z-index, visible through trees)
        if let Some(ref local_id) = state.local_player_id {
            if let Some(local_player) = state.players.get(local_id) {
                self.render_player_silhouette(local_player, &state.camera);
            }
        }

        timings.entities_ms = (get_time() - t1) * 1000.0;

        // 4. Render overhead layer (always on top)
        let t2 = get_time();
        self.render_tilemap_layer(state, LayerType::Overhead);
        timings.overhead_ms = (get_time() - t2) * 1000.0;

        // 5. Render floating damage numbers
        let t3 = get_time();
        self.render_damage_numbers(state);

        // 6. Render floating level up text
        self.render_level_up_events(state);

        // 7. Render chat bubbles above players
        self.render_chat_bubbles(state);

        // 7.5. Render projectiles
        self.render_projectiles(state);
        timings.effects_ms = (get_time() - t3) * 1000.0;

        // 8. Render UI (non-interactive elements)
        let t4 = get_time();
        self.render_ui(state);

        // 9. Render interactive UI elements and return layout for hit detection
        let layout = self.render_interactive_ui(state);
        timings.ui_ms = (get_time() - t4) * 1000.0;

        timings.total_ms = (get_time() - render_start) * 1000.0;
        (layout, timings)
    }

    fn render_level_up_events(&self, state: &GameState) {
        let current_time = macroquad::time::get_time();

        for event in &state.level_up_events {
            let age = (current_time - event.time) as f32;
            if age > 2.0 {
                continue;
            }

            // Calculate position with upward float
            let float_offset = age * 20.0; // Float up over time
            let (screen_x, screen_y) = world_to_screen(event.x, event.y, &state.camera);
            let final_y = screen_y - 40.0 - float_offset;

            // Fade out over time (slower fade)
            let alpha = ((2.0 - age) / 2.0 * 255.0) as u8;

            // Draw "LEVEL UP!" text with outline
            let text = format!("LEVEL UP! ({})", event.new_level);
            let font_size = 24.0;
            let text_width = measure_text(&text, None, font_size as u16, 1.0).width;

            // Outline
            let outline_color = Color::from_rgba(0, 0, 0, alpha);
            for ox in [-2.0, 2.0] {
                for oy in [-2.0, 2.0] {
                    draw_text(
                        &text,
                        screen_x - text_width / 2.0 + ox,
                        final_y + oy,
                        font_size,
                        outline_color,
                    );
                }
            }

            // Main text (gold color)
            draw_text(
                &text,
                screen_x - text_width / 2.0,
                final_y,
                font_size,
                Color::from_rgba(255, 215, 0, alpha),
            );
        }
    }

    /// Create a mesh for a rounded rectangle with optional tail (no overlapping geometry)
    fn create_rounded_rect_mesh(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) -> Mesh {
        Self::create_bubble_mesh(x, y, w, h, r, color, None)
    }

    /// Create a mesh for a chat bubble with tail (no overlapping geometry)
    fn create_bubble_mesh(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color, tail: Option<(f32, f32, f32)>) -> Mesh {
        let color_arr = [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ];

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Helper to add a vertex
        let mut add_vertex = |px: f32, py: f32| -> u16 {
            let idx = vertices.len() as u16;
            vertices.push(Vertex {
                position: Vec3::new(px, py, 0.0),
                uv: Vec2::ZERO,
                color: color_arr,
                normal: Vec4::ZERO,
            });
            idx
        };

        // Corner circle segment count
        let segments = 8;

        // Center rectangle vertices (4 corners where the rounded corners meet)
        let c_tl = add_vertex(x + r, y + r);     // top-left inner corner
        let c_tr = add_vertex(x + w - r, y + r); // top-right inner corner
        let c_bl = add_vertex(x + r, y + h - r); // bottom-left inner corner
        let c_br = add_vertex(x + w - r, y + h - r); // bottom-right inner corner

        // Center rectangle (2 triangles)
        indices.extend_from_slice(&[c_tl, c_tr, c_br, c_tl, c_br, c_bl]);

        // Top edge strip
        let t_tl = add_vertex(x + r, y);
        let t_tr = add_vertex(x + w - r, y);
        indices.extend_from_slice(&[t_tl, t_tr, c_tr, t_tl, c_tr, c_tl]);

        // Bottom edge strip
        let b_bl = add_vertex(x + r, y + h);
        let b_br = add_vertex(x + w - r, y + h);
        indices.extend_from_slice(&[c_bl, c_br, b_br, c_bl, b_br, b_bl]);

        // Left edge strip
        let l_tl = add_vertex(x, y + r);
        let l_bl = add_vertex(x, y + h - r);
        indices.extend_from_slice(&[l_tl, c_tl, c_bl, l_tl, c_bl, l_bl]);

        // Right edge strip
        let r_tr = add_vertex(x + w, y + r);
        let r_br = add_vertex(x + w, y + h - r);
        indices.extend_from_slice(&[c_tr, r_tr, r_br, c_tr, r_br, c_br]);

        // Corner fans (quarter circles)
        use std::f32::consts::PI;

        // Top-left corner (180° to 270°)
        let mut prev = l_tl; // starts at left edge
        for i in 1..=segments {
            let angle = PI + (PI / 2.0) * (i as f32 / segments as f32);
            let px = x + r + r * angle.cos();
            let py = y + r + r * angle.sin();
            let curr = add_vertex(px, py);
            indices.extend_from_slice(&[c_tl, prev, curr]);
            prev = curr;
        }

        // Top-right corner (270° to 360°)
        prev = t_tr; // starts at top edge
        for i in 1..=segments {
            let angle = PI * 1.5 + (PI / 2.0) * (i as f32 / segments as f32);
            let px = x + w - r + r * angle.cos();
            let py = y + r + r * angle.sin();
            let curr = add_vertex(px, py);
            indices.extend_from_slice(&[c_tr, prev, curr]);
            prev = curr;
        }

        // Bottom-right corner (0° to 90°)
        prev = r_br; // starts at right edge
        for i in 1..=segments {
            let angle = (PI / 2.0) * (i as f32 / segments as f32);
            let px = x + w - r + r * angle.cos();
            let py = y + h - r + r * angle.sin();
            let curr = add_vertex(px, py);
            indices.extend_from_slice(&[c_br, prev, curr]);
            prev = curr;
        }

        // Bottom-left corner (90° to 180°)
        prev = b_bl; // starts at bottom edge
        for i in 1..=segments {
            let angle = PI / 2.0 + (PI / 2.0) * (i as f32 / segments as f32);
            let px = x + r + r * angle.cos();
            let py = y + h - r + r * angle.sin();
            let curr = add_vertex(px, py);
            indices.extend_from_slice(&[c_bl, prev, curr]);
            prev = curr;
        }

        Mesh {
            vertices,
            indices,
            texture: None,
        }
    }

    /// Render chat bubbles above players' heads
    fn render_chat_bubbles(&self, state: &GameState) {
        let current_time = macroquad::time::get_time();

        for bubble in &state.chat_bubbles {
            let age = (current_time - bubble.time) as f32;
            if age > 5.0 {
                continue;
            }

            // Find the player this bubble belongs to
            let Some(player) = state.players.get(&bubble.player_id) else {
                continue;
            };

            // Get player screen position
            let (screen_x, screen_y) = world_to_screen(player.x, player.y, &state.camera);

            // Fade out in the last 1 second (age 4-5)
            let alpha = if age > 4.0 {
                ((5.0 - age) * 255.0) as u8
            } else {
                255
            };

            // Word wrap the text
            let max_bubble_width = 220.0;
            let font_size = 16.0;
            let line_height = 18.0;
            let padding_h = 4.0;
            let padding_v = 1.0;
            let tail_height = 6.0;
            let corner_radius = 5.0;

            let lines = self.wrap_text(&bubble.text, max_bubble_width - padding_h * 2.0, font_size);
            let num_lines = lines.len().max(1);

            // Calculate bubble dimensions
            let mut max_line_width = 0.0f32;
            for line in &lines {
                let width = self.measure_text_sharp(line, font_size).width;
                max_line_width = max_line_width.max(width);
            }

            let bubble_width = (max_line_width + padding_h * 2.0).max(18.0);
            let bubble_height = num_lines as f32 * line_height + padding_v * 2.0;

            // Position bubble above player's head
            // Base offset: sprite height (78) minus feet offset (8) = 70, scaled by zoom
            let zoom = state.camera.zoom;
            let base_offset = (SPRITE_HEIGHT - 8.0) * zoom;

            // Check if name tag is showing (hovered or selected) - need extra space
            let is_hovered = state.hovered_entity_id.as_ref() == Some(&bubble.player_id);
            let is_selected = state.selected_entity_id.as_ref() == Some(&bubble.player_id);
            let name_offset = if is_hovered || is_selected { 16.0 } else { 0.0 };

            let bubble_x = screen_x - bubble_width / 2.0;
            let bubble_y = screen_y - base_offset - name_offset - bubble_height - tail_height;

            // Colors with alpha - off-white paper/comic book style
            let bg_alpha = (alpha as f32 * 0.8) as u8; // 80% opacity for background
            let bg_color = Color::from_rgba(255, 250, 240, bg_alpha); // Warm off-white/cream
            let border_color = Color::from_rgba(60, 50, 40, alpha); // Dark brown border
            let text_color = Color::from_rgba(30, 25, 20, alpha); // Dark brown text

            // Draw rounded rectangle bubble body using mesh (no overlapping geometry)
            let r = corner_radius;
            let bx = bubble_x.floor();
            let by = bubble_y.floor();
            let bw = bubble_width.floor();
            let bh = bubble_height.floor();

            // Draw border first (slightly larger rounded rect)
            let border_mesh = Self::create_rounded_rect_mesh(bx - 1.0, by - 1.0, bw + 2.0, bh + 2.0, r + 1.0, border_color);
            draw_mesh(&border_mesh);

            // Draw fill on top using mesh (no overlapping = no alpha stacking)
            let fill_mesh = Self::create_rounded_rect_mesh(bx, by, bw, bh, r, bg_color);
            draw_mesh(&fill_mesh);

            // Draw tail
            let tail_x = screen_x.floor();
            let tail_top_y = by + bh;
            let tail_bottom_y = tail_top_y + tail_height;
            let tail_half_width = 4.0;

            // Tail border
            draw_triangle(
                Vec2::new(tail_x - tail_half_width - 1.0, tail_top_y),
                Vec2::new(tail_x + tail_half_width + 1.0, tail_top_y),
                Vec2::new(tail_x, tail_bottom_y + 1.0),
                border_color,
            );
            // Tail fill - use a mesh vertex approach to match the bubble's alpha exactly
            // Create a small mesh for just the tail triangle
            let tail_color_arr = [
                (bg_color.r * 255.0) as u8,
                (bg_color.g * 255.0) as u8,
                (bg_color.b * 255.0) as u8,
                (bg_color.a * 255.0) as u8,
            ];
            let tail_mesh = Mesh {
                vertices: vec![
                    Vertex { position: Vec3::new(tail_x - tail_half_width, tail_top_y, 0.0), uv: Vec2::ZERO, color: tail_color_arr, normal: Vec4::ZERO },
                    Vertex { position: Vec3::new(tail_x + tail_half_width, tail_top_y, 0.0), uv: Vec2::ZERO, color: tail_color_arr, normal: Vec4::ZERO },
                    Vertex { position: Vec3::new(tail_x, tail_bottom_y, 0.0), uv: Vec2::ZERO, color: tail_color_arr, normal: Vec4::ZERO },
                ],
                indices: vec![0, 1, 2],
                texture: None,
            };
            draw_mesh(&tail_mesh);

            // Tail border lines
            draw_line(tail_x - tail_half_width, tail_top_y, tail_x, tail_bottom_y, 1.0, border_color);
            draw_line(tail_x + tail_half_width, tail_top_y, tail_x, tail_bottom_y, 1.0, border_color);

            // Draw text lines (centered)
            let bubble_center_x = bx + bw / 2.0;
            let mut text_y = by + padding_v + font_size * 0.85;

            for line in &lines {
                let line_width = self.measure_text_sharp(line, font_size).width;
                let text_x = bubble_center_x - line_width / 2.0;
                self.draw_text_sharp(line, text_x, text_y, font_size, text_color);
                text_y += line_height;
            }
        }
    }

    fn render_projectiles(&self, state: &GameState) {
        let current_time = macroquad::time::get_time();

        for projectile in &state.projectiles {
            let (world_x, world_y) = projectile.current_pos(current_time);
            let (screen_x, screen_y_raw) = world_to_screen(world_x, world_y, &state.camera);

            // Offset arrow vertically to match player center (not feet)
            let arrow_y_offset = -40.0 * state.camera.zoom;
            let screen_y = screen_y_raw + arrow_y_offset;

            // Calculate direction in SCREEN space (accounts for isometric transform)
            let (start_screen_x, start_screen_y) = world_to_screen(projectile.start_x, projectile.start_y, &state.camera);
            let (end_screen_x, end_screen_y) = world_to_screen(projectile.end_x, projectile.end_y, &state.camera);
            let dx = end_screen_x - start_screen_x;
            let dy = end_screen_y - start_screen_y;
            let angle = dy.atan2(dx);

            // Snap to isometric angles (2:1 ratio = atan2(1,2) ≈ 26.57°)
            // 8 isometric directions: 0°, 26.57°, 90°, 153.43°, 180°, -153.43°, -90°, -26.57°
            let iso_angle = (0.5_f32).atan(); // atan(1/2) ≈ 26.57° ≈ 0.4636 rad
            let iso_angles: [f32; 8] = [
                0.0,                                    // UpRight (east)
                iso_angle,                              // Right (26.57°)
                std::f32::consts::FRAC_PI_2,           // DownRight (90°)
                std::f32::consts::PI - iso_angle,      // Down (153.43°)
                std::f32::consts::PI,                  // DownLeft (180°)
                -std::f32::consts::PI + iso_angle,     // Left (-153.43°)
                -std::f32::consts::FRAC_PI_2,          // UpLeft (-90°)
                -iso_angle,                            // Up (-26.57°)
            ];

            // Find nearest isometric angle
            let mut snapped_angle = iso_angles[0];
            let mut min_diff = f32::MAX;
            for &iso_ang in &iso_angles {
                let mut diff = (angle - iso_ang).abs();
                // Handle wrap-around at ±180°
                if diff > std::f32::consts::PI {
                    diff = 2.0 * std::f32::consts::PI - diff;
                }
                if diff < min_diff {
                    min_diff = diff;
                    snapped_angle = iso_ang;
                }
            }

            // Direction vector from snapped angle
            let dir_x = snapped_angle.cos();
            let dir_y = snapped_angle.sin();

            // Perpendicular vector for arrow width
            let perp_x = -dir_y;
            let perp_y = dir_x;

            // Arrow dimensions
            let shaft_length = 18.0;
            let shaft_width = 2.0;
            let head_length = 6.0;
            let head_width = 5.0;
            let fletch_length = 4.0;
            let fletch_width = 3.0;

            // Colors
            let shaft_color = Color::new(0.55, 0.35, 0.15, 1.0); // Wood brown
            let head_color = Color::new(0.45, 0.45, 0.5, 1.0);   // Metal gray
            let fletch_color = Color::new(0.85, 0.85, 0.8, 1.0); // Light feathers

            // Arrow positions
            let tip_x = screen_x + dir_x * (shaft_length / 2.0 + head_length);
            let tip_y = screen_y + dir_y * (shaft_length / 2.0 + head_length);
            let back_x = screen_x - dir_x * shaft_length / 2.0;
            let back_y = screen_y - dir_y * shaft_length / 2.0;

            // Draw shaft (thick line)
            draw_line(
                back_x,
                back_y,
                screen_x + dir_x * shaft_length / 2.0,
                screen_y + dir_y * shaft_length / 2.0,
                shaft_width,
                shaft_color,
            );

            // Draw arrowhead (triangle pointing forward)
            let head_base_x = screen_x + dir_x * shaft_length / 2.0;
            let head_base_y = screen_y + dir_y * shaft_length / 2.0;
            draw_triangle(
                Vec2::new(tip_x, tip_y),
                Vec2::new(head_base_x + perp_x * head_width / 2.0, head_base_y + perp_y * head_width / 2.0),
                Vec2::new(head_base_x - perp_x * head_width / 2.0, head_base_y - perp_y * head_width / 2.0),
                head_color,
            );

            // Draw fletching (two small triangles at the back)
            let fletch_base_x = back_x + dir_x * fletch_length;
            let fletch_base_y = back_y + dir_y * fletch_length;

            // Left fletch
            draw_triangle(
                Vec2::new(back_x + perp_x * shaft_width / 2.0, back_y + perp_y * shaft_width / 2.0),
                Vec2::new(fletch_base_x + perp_x * shaft_width / 2.0, fletch_base_y + perp_y * shaft_width / 2.0),
                Vec2::new(back_x + perp_x * fletch_width, back_y + perp_y * fletch_width),
                fletch_color,
            );

            // Right fletch
            draw_triangle(
                Vec2::new(back_x - perp_x * shaft_width / 2.0, back_y - perp_y * shaft_width / 2.0),
                Vec2::new(fletch_base_x - perp_x * shaft_width / 2.0, fletch_base_y - perp_y * shaft_width / 2.0),
                Vec2::new(back_x - perp_x * fletch_width, back_y - perp_y * fletch_width),
                fletch_color,
            );
        }
    }

    fn render_damage_numbers(&self, state: &GameState) {
        let current_time = macroquad::time::get_time();
        const DURATION: f32 = 1.2;
        const FONT_SIZE: f32 = 16.0;

        for event in &state.damage_events {
            let age = (current_time - event.time) as f32;
            if age > DURATION {
                continue;
            }

            let t = age / DURATION;

            // Steady float upward - round to whole pixels for crisp movement
            let float_offset = (age * 40.0).round();

            // Compute height offset based on entity type and actual sprite size
            let height_offset = if state.players.contains_key(&event.target_id) {
                (SPRITE_HEIGHT - 8.0) / 2.0 // Center of player sprite
            } else if let Some(npc) = state.npcs.get(&event.target_id) {
                // Use actual sprite height if available, otherwise fallback to ellipse size
                if let Some(sprite) = self.npc_sprites.get(&npc.entity_type) {
                    sprite.height() / 2.0 // Center of NPC sprite
                } else {
                    12.0 // Center of fallback ellipse
                }
            } else {
                25.0 // Fallback for unknown entities
            };

            let (screen_x, screen_y) = world_to_screen(event.x, event.y, &state.camera);
            // Round all positions to whole pixels
            let final_y = (screen_y - height_offset - float_offset).round();

            // Fade: visible for first half, then fade out
            let alpha = if t < 0.5 {
                1.0
            } else {
                1.0 - (t - 0.5) * 2.0
            };

            // Text and color
            let (text, base_color) = if event.damage > 0 {
                (format!("-{}", event.damage), Color::new(1.0, 0.3, 0.2, alpha))
            } else if event.damage < 0 {
                (format!("+{}", -event.damage), Color::new(0.3, 1.0, 0.4, alpha))
            } else {
                ("MISS".to_string(), Color::new(0.6, 0.6, 0.6, alpha))
            };

            let text_dims = self.measure_text_sharp(&text, FONT_SIZE);
            // Round center position to whole pixels
            let draw_x = (screen_x - text_dims.width / 2.0).round();

            // Simple outline
            let outline_color = Color::new(0.0, 0.0, 0.0, alpha * 0.9);
            for &(ox, oy) in &[(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                self.draw_text_sharp(&text, draw_x + ox, final_y + oy, FONT_SIZE, outline_color);
            }

            self.draw_text_sharp(&text, draw_x, final_y, FONT_SIZE, base_color);
        }
    }

    fn render_tilemap_layer(&self, state: &GameState, layer_type: LayerType) {
        // Convert LayerType to ChunkLayerType for chunk rendering
        let chunk_layer_type = match layer_type {
            LayerType::Ground => ChunkLayerType::Ground,
            LayerType::Objects => ChunkLayerType::Objects,
            LayerType::Overhead => ChunkLayerType::Overhead,
        };

        // Try to render from chunks if any are loaded
        let chunks = state.chunk_manager.chunks();
        if !chunks.is_empty() {
            // Screen bounds for culling
            let screen_w = screen_width();
            let screen_h = screen_height();
            let margin = TILE_WIDTH * 4.0; // Extra margin for chunk edges

            // Render from chunk manager
            for (coord, chunk) in chunks.iter() {
                let chunk_offset_x = coord.x * CHUNK_SIZE as i32;
                let chunk_offset_y = coord.y * CHUNK_SIZE as i32;

                // CHUNK-LEVEL CULLING: Check if chunk is visible before iterating tiles
                // In isometric projection, a chunk forms a diamond. Check all 4 corners.
                let corners = [
                    (chunk_offset_x as f32, chunk_offset_y as f32),                           // top
                    (chunk_offset_x as f32 + CHUNK_SIZE as f32, chunk_offset_y as f32),       // right
                    (chunk_offset_x as f32, chunk_offset_y as f32 + CHUNK_SIZE as f32),       // left
                    (chunk_offset_x as f32 + CHUNK_SIZE as f32, chunk_offset_y as f32 + CHUNK_SIZE as f32), // bottom
                ];

                // Get screen bounds of the chunk
                let mut min_sx = f32::MAX;
                let mut max_sx = f32::MIN;
                let mut min_sy = f32::MAX;
                let mut max_sy = f32::MIN;

                for (wx, wy) in corners {
                    let (sx, sy) = world_to_screen(wx, wy, &state.camera);
                    min_sx = min_sx.min(sx);
                    max_sx = max_sx.max(sx);
                    min_sy = min_sy.min(sy);
                    max_sy = max_sy.max(sy);
                }

                // Skip entire chunk if completely off-screen
                if max_sx < -margin || min_sx > screen_w + margin ||
                   max_sy < -margin || min_sy > screen_h + margin {
                    continue;
                }

                // Find the layer
                for layer in &chunk.layers {
                    if layer.layer_type != chunk_layer_type {
                        continue;
                    }

                    // Render tiles in isometric order
                    for local_y in 0..CHUNK_SIZE {
                        for local_x in 0..CHUNK_SIZE {
                            let world_x = chunk_offset_x + local_x as i32;
                            let world_y = chunk_offset_y + local_y as i32;

                            let idx = (local_y * CHUNK_SIZE + local_x) as usize;
                            let tile_id = layer.tiles.get(idx).copied().unwrap_or(0);

                            if tile_id == 0 {
                                continue;
                            }

                            let (screen_x, screen_y) = world_to_screen(world_x as f32, world_y as f32, &state.camera);

                            // Tile-level culling (still needed for partially visible chunks)
                            let tile_margin = TILE_WIDTH * 2.0;
                            if screen_x < -tile_margin || screen_x > screen_w + tile_margin {
                                continue;
                            }
                            if screen_y < -tile_margin || screen_y > screen_h + tile_margin {
                                continue;
                            }

                            // Draw tile sprite (or fallback to colored tile)
                            self.draw_tile_sprite(screen_x, screen_y, tile_id, state.camera.zoom);

                            // Draw collision indicator in debug mode
                            if state.debug_mode && chunk.collision.get(idx).copied().unwrap_or(false) {
                                self.draw_collision_indicator(screen_x, screen_y, state.camera.zoom);
                            }
                        }
                    }
                }
            }
            return;
        }

        // Fallback: render from old tilemap if no chunks loaded
        let tilemap = &state.tilemap;

        for layer in &tilemap.layers {
            if layer.layer_type != layer_type {
                continue;
            }

            // Render in isometric order (back to front)
            for y in 0..tilemap.height {
                for x in 0..tilemap.width {
                    let idx = (y * tilemap.width + x) as usize;
                    let tile_id = layer.tiles.get(idx).copied().unwrap_or(0);

                    if tile_id == 0 {
                        continue; // Skip empty tiles
                    }

                    let (screen_x, screen_y) = world_to_screen(x as f32, y as f32, &state.camera);

                    // Culling: skip tiles outside viewport
                    let margin = TILE_WIDTH * 2.0;
                    if screen_x < -margin || screen_x > screen_width() + margin {
                        continue;
                    }
                    if screen_y < -margin || screen_y > screen_height() + margin {
                        continue;
                    }

                    // Draw tile sprite (or fallback to colored tile)
                    self.draw_tile_sprite(screen_x, screen_y, tile_id, state.camera.zoom);

                    // Draw collision indicator in debug mode
                    if state.debug_mode && tilemap.collision.get(idx).copied().unwrap_or(false) {
                        self.draw_collision_indicator(screen_x, screen_y, state.camera.zoom);
                    }
                }
            }
        }
    }

    fn draw_collision_indicator(&self, screen_x: f32, screen_y: f32, zoom: f32) {
        let half_w = TILE_WIDTH * zoom / 4.0;
        let half_h = TILE_HEIGHT * zoom / 4.0;
        draw_rectangle_lines(
            screen_x - half_w,
            screen_y - half_h,
            half_w * 2.0,
            half_h * 2.0,
            2.0 * zoom,
            Color::from_rgba(255, 0, 0, 150),
        );
    }

    fn draw_isometric_object(&self, screen_x: f32, screen_y: f32, tile_id: u32, zoom: f32) {
        // Draw shadow ellipse for objects
        draw_ellipse(screen_x, screen_y + 4.0 * zoom, 24.0 * zoom, 16.0 * zoom, 0.0, Color::from_rgba(0, 0, 0, 50));

        // Draw object tile sprite (slightly elevated)
        let elevated_y = screen_y - TILE_HEIGHT * zoom * 0.25;
        self.draw_tile_sprite(screen_x, elevated_y, tile_id, zoom);
    }

    fn draw_isometric_tile(&self, screen_x: f32, screen_y: f32, color: Color, zoom: f32) {
        // Draw a diamond-shaped tile
        let half_w = TILE_WIDTH * zoom / 2.0;
        let half_h = TILE_HEIGHT * zoom / 2.0;

        // Diamond vertices (clockwise from top)
        let top = (screen_x, screen_y - half_h);
        let right = (screen_x + half_w, screen_y);
        let bottom = (screen_x, screen_y + half_h);
        let left = (screen_x - half_w, screen_y);

        // Draw as two triangles
        draw_triangle(
            Vec2::new(top.0, top.1),
            Vec2::new(right.0, right.1),
            Vec2::new(bottom.0, bottom.1),
            color,
        );
        draw_triangle(
            Vec2::new(top.0, top.1),
            Vec2::new(bottom.0, bottom.1),
            Vec2::new(left.0, left.1),
            color,
        );

        // Draw outline
        let outline_color = Color::from_rgba(80, 80, 90, 255);
        draw_line(top.0, top.1, right.0, right.1, 1.0, outline_color);
        draw_line(right.0, right.1, bottom.0, bottom.1, 1.0, outline_color);
        draw_line(bottom.0, bottom.1, left.0, left.1, 1.0, outline_color);
        draw_line(left.0, left.1, top.0, top.1, 1.0, outline_color);
    }

    /// Draw a selection highlight around the tile at the given world position
    fn render_tile_selection(&self, world_x: f32, world_y: f32, camera: &Camera) {
        // Get the tile the entity is standing on (floor to get tile coords)
        let tile_x = world_x.floor();
        let tile_y = world_y.floor();

        // Get the center of that tile in screen space
        // Offset by half_h to align with where entities visually stand on the tile
        let (center_x, center_y) = world_to_screen(tile_x + 0.5, tile_y + 0.5, camera);
        let center_y = center_y - TILE_HEIGHT * camera.zoom / 2.0;

        // Tile dimensions (half-sizes for diamond corners), scaled by zoom
        let half_w = TILE_WIDTH * camera.zoom / 2.0;
        let half_h = TILE_HEIGHT * camera.zoom / 2.0;

        // Diamond corners (isometric tile shape)
        let top = (center_x, center_y - half_h);
        let right = (center_x + half_w, center_y);
        let bottom = (center_x, center_y + half_h);
        let left = (center_x - half_w, center_y);

        // Pulsing effect
        let pulse = (macroquad::time::get_time() * 3.0).sin() as f32 * 0.3 + 0.7;
        let alpha = (pulse * 255.0) as u8;
        let color = Color::from_rgba(255, 255, 0, alpha);

        // Draw yellow diamond outline
        let line_width = 2.0 * camera.zoom;
        draw_line(top.0, top.1, right.0, right.1, line_width, color);
        draw_line(right.0, right.1, bottom.0, bottom.1, line_width, color);
        draw_line(bottom.0, bottom.1, left.0, left.1, line_width, color);
        draw_line(left.0, left.1, top.0, top.1, line_width, color);
    }

    /// Draw corner indicators for the hovered tile
    pub(crate) fn render_tile_hover(&self, tile_x: i32, tile_y: i32, camera: &Camera) {
        // Get the center of the tile in screen space
        let (center_x, center_y) = world_to_screen(tile_x as f32 + 0.5, tile_y as f32 + 0.5, camera);
        let center_y = center_y - TILE_HEIGHT * camera.zoom / 2.0;

        // Tile dimensions (half-sizes for diamond corners), scaled by zoom
        let half_w = TILE_WIDTH * camera.zoom / 2.0;
        let half_h = TILE_HEIGHT * camera.zoom / 2.0;

        // Diamond corners (isometric tile shape)
        let top = (center_x, center_y - half_h);
        let right = (center_x + half_w, center_y);
        let bottom = (center_x, center_y + half_h);
        let left = (center_x - half_w, center_y);

        // Corner size as fraction of edge length
        let t = 0.2;

        // Thin white lines with some transparency
        let color = Color::from_rgba(255, 255, 255, 180);
        let line_width = 1.0 * camera.zoom;

        // Top corner
        draw_line(top.0, top.1, top.0 + (left.0 - top.0) * t, top.1 + (left.1 - top.1) * t, line_width, color);
        draw_line(top.0, top.1, top.0 + (right.0 - top.0) * t, top.1 + (right.1 - top.1) * t, line_width, color);

        // Right corner
        draw_line(right.0, right.1, right.0 + (top.0 - right.0) * t, right.1 + (top.1 - right.1) * t, line_width, color);
        draw_line(right.0, right.1, right.0 + (bottom.0 - right.0) * t, right.1 + (bottom.1 - right.1) * t, line_width, color);

        // Bottom corner
        draw_line(bottom.0, bottom.1, bottom.0 + (right.0 - bottom.0) * t, bottom.1 + (right.1 - bottom.1) * t, line_width, color);
        draw_line(bottom.0, bottom.1, bottom.0 + (left.0 - bottom.0) * t, bottom.1 + (left.1 - bottom.1) * t, line_width, color);

        // Left corner
        draw_line(left.0, left.1, left.0 + (bottom.0 - left.0) * t, left.1 + (bottom.1 - left.1) * t, line_width, color);
        draw_line(left.0, left.1, left.0 + (top.0 - left.0) * t, left.1 + (top.1 - left.1) * t, line_width, color);
    }

    fn render_player(&self, player: &Player, is_local: bool, is_selected: bool, is_hovered: bool, camera: &Camera) {
        let (screen_x, screen_y) = world_to_screen(player.x, player.y, camera);
        let zoom = camera.zoom;

        // Scaled sprite dimensions
        let scaled_sprite_width = SPRITE_WIDTH * zoom;
        let scaled_sprite_height = SPRITE_HEIGHT * zoom;

        // Dead players are faded
        let alpha = if player.is_dead { 100 } else { 255 };

        // Selection highlight (draw first, behind player)
        if is_selected && !player.is_dead {
            self.render_tile_selection(player.x, player.y, camera);
        }

        // Draw shadow under player
        draw_ellipse(screen_x, screen_y, 16.0 * zoom, 7.0 * zoom, 0.0, Color::from_rgba(0, 0, 0, 60));

        // Try to render sprite based on player's appearance, fall back to colored circle
        if let Some(sprite) = self.get_player_sprite(&player.gender, &player.skin) {
            let coords = player.animation.get_sprite_coords();
            let (src_x, src_y, src_w, src_h) = coords.to_source_rect();

            // Tint for local player distinction (slight green tint)
            let tint = if is_local {
                Color::from_rgba(220, 255, 220, alpha)
            } else {
                Color::from_rgba(255, 255, 255, alpha)
            };

            // Position sprite so feet are at screen_y
            let draw_x = screen_x - scaled_sprite_width / 2.0;
            let draw_y = screen_y - scaled_sprite_height + 8.0 * zoom; // Offset to align feet with tile

            // Calculate weapon frame info if weapon is equipped
            let weapon_info = player.equipped_weapon.as_ref().and_then(|weapon_id| {
                self.weapon_sprites.get(weapon_id).map(|weapon_sprite| {
                    let anim_frame = player.animation.frame as u32;
                    let weapon_frame = get_weapon_frame(player.animation.state, player.animation.direction, anim_frame);
                    let (offset_x, offset_y) = get_weapon_offset(player.animation.state, player.animation.direction, anim_frame);
                    (weapon_sprite, weapon_frame, offset_x, offset_y)
                })
            });

            // Scaled weapon dimensions
            let scaled_weapon_width = WEAPON_SPRITE_WIDTH * zoom;
            let scaled_weapon_height = WEAPON_SPRITE_HEIGHT * zoom;

            // Draw weapon under-layer (before player sprite)
            if let Some((weapon_sprite, ref weapon_frame, offset_x, offset_y)) = weapon_info {
                let weapon_src_x = weapon_frame.frame_under as f32 * WEAPON_SPRITE_WIDTH;
                let weapon_draw_x = draw_x + offset_x * zoom;
                let weapon_draw_y = draw_y + offset_y * zoom;

                draw_texture_ex(
                    weapon_sprite,
                    weapon_draw_x,
                    weapon_draw_y,
                    tint,
                    DrawTextureParams {
                        source: Some(Rect::new(weapon_src_x, 0.0, WEAPON_SPRITE_WIDTH, WEAPON_SPRITE_HEIGHT)),
                        dest_size: Some(Vec2::new(scaled_weapon_width, scaled_weapon_height)),
                        flip_x: weapon_frame.flip_h,
                        ..Default::default()
                    },
                );
            }

            // Draw player sprite
            draw_texture_ex(
                sprite,
                draw_x,
                draw_y,
                tint,
                DrawTextureParams {
                    source: Some(Rect::new(src_x, src_y, src_w, src_h)),
                    dest_size: Some(Vec2::new(scaled_sprite_width, scaled_sprite_height)),
                    flip_x: coords.flip_h,
                    ..Default::default()
                },
            );

            // Draw hair and head equipment (after base sprite, before body armor)
            // Hair sprites are 28x54, smaller than player sprites (34x78)
            const HAIR_SPRITE_WIDTH: f32 = 28.0;
            const HAIR_SPRITE_HEIGHT: f32 = 54.0;

            // Check if player has head equipment that we can render with shader
            let head_sprite = player.equipped_head.as_ref()
                .and_then(|head_item_id| self.equipment_sprites.get(head_item_id));

            let has_shader = self.head_hair_material.is_some();

            if let Some(head_sprite) = head_sprite {
                // Player has head equipment - use shader compositing if available
                if has_shader {
                    if let (Some(style), Some(color)) = (player.hair_style, player.hair_color) {
                        if let Some(hair_tex) = self.hair_sprites.get(&style) {
                            // Calculate hair frame info
                            let is_back = matches!(player.animation.direction, Direction::Up | Direction::Left);
                            let frame_index = color * 2 + if is_back { 1 } else { 0 };
                            let hair_src_x = frame_index as f32 * HAIR_SPRITE_WIDTH;

                            // Calculate hair offsets
                            let is_attack_frame_2 = player.animation.state == AnimationState::Attacking && (player.animation.frame as u32 % 2) == 1;
                            let is_shooting_bow = player.animation.state == AnimationState::ShootingBow;
                            let (hair_offset_x, hair_offset_y) = if is_attack_frame_2 {
                                let y_offset = if is_back { -2.0 } else { 2.0 };
                                let x_offset = if is_back {
                                    if coords.flip_h { 5.0 } else { -5.0 }
                                } else {
                                    if coords.flip_h { 6.0 } else { -6.0 }
                                };
                                (x_offset, y_offset)
                            } else if is_shooting_bow {
                                let x_offset = if is_back {
                                    if coords.flip_h { 1.0 } else { -1.0 }
                                } else {
                                    if coords.flip_h { 2.0 } else { -2.0 }
                                };
                                (x_offset, -3.0)
                            } else {
                                let x_offset = if is_back {
                                    if coords.flip_h { 2.0 } else { -2.0 }
                                } else {
                                    if coords.flip_h { 1.0 } else { -1.0 }
                                };
                                (x_offset, -3.0)
                            };

                            // Calculate head frame info
                            let anim_frame = player.animation.frame as u32;
                            let head_frame = get_head_frame(player.animation.direction);
                            let (head_offset_x, head_offset_y) = get_head_offset(player.animation.state, player.animation.direction, anim_frame);
                            let head_src_x = head_frame.frame as f32 * HEAD_SPRITE_WIDTH;

                            // Calculate pixel offset from head origin to hair origin (in unscaled pixels)
                            // Hair is centered: hair_x = (SPRITE_WIDTH - HAIR_SPRITE_WIDTH) / 2 + hair_offset_x = 3 + hair_offset_x
                            // Head uses head_offset_x directly
                            let hair_base_x = (SPRITE_WIDTH - HAIR_SPRITE_WIDTH) / 2.0 + hair_offset_x;
                            let hair_base_y = hair_offset_y;
                            let delta_x = hair_base_x - head_offset_x;
                            let delta_y = hair_base_y - head_offset_y;

                            // Compute UV transform for shader
                            // The shader needs to transform head UV to hair UV
                            // head UV is in full-texture coords, so we need to account for source rects
                            let head_tex_w = head_sprite.width();
                            let head_tex_h = head_sprite.height();
                            let hair_tex_w = hair_tex.width();
                            let hair_tex_h = hair_tex.height();

                            // Head source rect in normalized UV
                            let head_uv_x = head_src_x / head_tex_w;
                            let head_uv_w = HEAD_SPRITE_WIDTH / head_tex_w;
                            let head_uv_h = HEAD_SPRITE_HEIGHT / head_tex_h;

                            // Hair source rect in normalized UV
                            let hair_uv_x = hair_src_x / hair_tex_w;
                            let hair_uv_w = HAIR_SPRITE_WIDTH / hair_tex_w;
                            let hair_uv_h = HAIR_SPRITE_HEIGHT / hair_tex_h;

                            // The transform: given head UV (u, v) in full texture coords
                            // 1. Normalize to head frame: local = (u - head_uv_x) / head_uv_w, (v - 0) / head_uv_h
                            // 2. To pixels: pixel = local * (HEAD_SPRITE_WIDTH, HEAD_SPRITE_HEIGHT)
                            // 3. Offset: hair_pixel = pixel - (delta_x, delta_y)
                            // 4. To hair local: hair_local = hair_pixel / (HAIR_SPRITE_WIDTH, HAIR_SPRITE_HEIGHT)
                            // 5. To hair UV: hair_uv = hair_uv_x + hair_local.x * hair_uv_w, hair_local.y * hair_uv_h

                            // Combining: hair_uv.x = hair_uv_x + ((u - head_uv_x) / head_uv_w * HEAD_SPRITE_WIDTH - delta_x) / HAIR_SPRITE_WIDTH * hair_uv_w
                            // Simplify: hair_uv.x = hair_uv_x + (u - head_uv_x) * (HEAD_SPRITE_WIDTH / head_uv_w / HAIR_SPRITE_WIDTH) * hair_uv_w - delta_x / HAIR_SPRITE_WIDTH * hair_uv_w
                            // Since head_uv_w = HEAD_SPRITE_WIDTH / head_tex_w, so HEAD_SPRITE_WIDTH / head_uv_w = head_tex_w
                            // hair_uv.x = hair_uv_x + (u - head_uv_x) * head_tex_w / HAIR_SPRITE_WIDTH * hair_uv_w - delta_x * hair_uv_w / HAIR_SPRITE_WIDTH
                            // Let scale_x = head_tex_w * hair_uv_w / HAIR_SPRITE_WIDTH
                            // Let offset_x = hair_uv_x - head_uv_x * scale_x - delta_x * hair_uv_w / HAIR_SPRITE_WIDTH
                            // Then: hair_uv.x = offset_x + u * scale_x

                            let scale_x = head_tex_w * hair_uv_w / HAIR_SPRITE_WIDTH;
                            let scale_y = head_tex_h * hair_uv_h / HAIR_SPRITE_HEIGHT;
                            let offset_x = hair_uv_x - head_uv_x * scale_x - delta_x * hair_uv_w / HAIR_SPRITE_WIDTH;
                            let offset_y = -delta_y * hair_uv_h / HAIR_SPRITE_HEIGHT;

                            // Set up shader
                            let material = self.head_hair_material.as_ref().unwrap();
                            material.set_texture("HairTexture", hair_tex.clone());
                            material.set_uniform("HairUvTransform", [offset_x, offset_y, scale_x, scale_y]);
                            material.set_uniform("Tint", [1.0f32, 1.0f32, 1.0f32, 1.0f32]);
                            gl_use_material(material);

                            // Draw head with shader active
                            let scaled_head_width = HEAD_SPRITE_WIDTH * zoom;
                            let scaled_head_height = HEAD_SPRITE_HEIGHT * zoom;
                            let head_draw_x = draw_x + head_offset_x * zoom;
                            let head_draw_y = draw_y + head_offset_y * zoom;

                            draw_texture_ex(
                                &head_sprite,
                                head_draw_x,
                                head_draw_y,
                                tint,
                                DrawTextureParams {
                                    source: Some(Rect::new(head_src_x, 0.0, HEAD_SPRITE_WIDTH, HEAD_SPRITE_HEIGHT)),
                                    dest_size: Some(Vec2::new(scaled_head_width, scaled_head_height)),
                                    flip_x: head_frame.flip_h,
                                    ..Default::default()
                                },
                            );

                            gl_use_default_material();
                        }
                    } else {
                        // No hair, just draw head normally
                        let anim_frame = player.animation.frame as u32;
                        let head_frame = get_head_frame(player.animation.direction);
                        let (head_offset_x, head_offset_y) = get_head_offset(player.animation.state, player.animation.direction, anim_frame);
                        let head_src_x = head_frame.frame as f32 * HEAD_SPRITE_WIDTH;
                        let scaled_head_width = HEAD_SPRITE_WIDTH * zoom;
                        let scaled_head_height = HEAD_SPRITE_HEIGHT * zoom;
                        let head_draw_x = draw_x + head_offset_x * zoom;
                        let head_draw_y = draw_y + head_offset_y * zoom;

                        draw_texture_ex(
                            &head_sprite,
                            head_draw_x,
                            head_draw_y,
                            tint,
                            DrawTextureParams {
                                source: Some(Rect::new(head_src_x, 0.0, HEAD_SPRITE_WIDTH, HEAD_SPRITE_HEIGHT)),
                                dest_size: Some(Vec2::new(scaled_head_width, scaled_head_height)),
                                flip_x: head_frame.flip_h,
                                ..Default::default()
                            },
                        );
                    }
                } else {
                    // No shader available, draw hair then head (hair will show through transparent areas)
                    // Draw hair first
                    if let (Some(style), Some(color)) = (player.hair_style, player.hair_color) {
                        if let Some(hair_tex) = self.hair_sprites.get(&style) {
                            let is_back = matches!(player.animation.direction, Direction::Up | Direction::Left);
                            let frame_index = color * 2 + if is_back { 1 } else { 0 };
                            let hair_src_x = frame_index as f32 * HAIR_SPRITE_WIDTH;
                            let scaled_hair_width = HAIR_SPRITE_WIDTH * zoom;
                            let scaled_hair_height = HAIR_SPRITE_HEIGHT * zoom;

                            let is_attack_frame_2 = player.animation.state == AnimationState::Attacking && (player.animation.frame as u32 % 2) == 1;
                            let is_shooting_bow = player.animation.state == AnimationState::ShootingBow;
                            let (hair_offset_x, hair_offset_y) = if is_attack_frame_2 {
                                let y_offset = if is_back { -2.0 } else { 2.0 };
                                let x_offset = if is_back {
                                    if coords.flip_h { 5.0 } else { -5.0 }
                                } else {
                                    if coords.flip_h { 6.0 } else { -6.0 }
                                };
                                (x_offset, y_offset)
                            } else if is_shooting_bow {
                                let x_offset = if is_back {
                                    if coords.flip_h { 1.0 } else { -1.0 }
                                } else {
                                    if coords.flip_h { 2.0 } else { -2.0 }
                                };
                                (x_offset, -3.0)
                            } else {
                                let x_offset = if is_back {
                                    if coords.flip_h { 2.0 } else { -2.0 }
                                } else {
                                    if coords.flip_h { 1.0 } else { -1.0 }
                                };
                                (x_offset, -3.0)
                            };

                            let hair_draw_x = draw_x + (scaled_sprite_width - scaled_hair_width) / 2.0 + hair_offset_x * zoom;
                            let hair_draw_y = draw_y + hair_offset_y * zoom;

                            draw_texture_ex(
                                hair_tex,
                                hair_draw_x,
                                hair_draw_y,
                                tint,
                                DrawTextureParams {
                                    source: Some(Rect::new(hair_src_x, 0.0, HAIR_SPRITE_WIDTH, HAIR_SPRITE_HEIGHT)),
                                    dest_size: Some(Vec2::new(scaled_hair_width, scaled_hair_height)),
                                    flip_x: coords.flip_h,
                                    ..Default::default()
                                },
                            );
                        }
                    }

                    // Then draw head on top
                    let anim_frame = player.animation.frame as u32;
                    let head_frame = get_head_frame(player.animation.direction);
                    let (head_offset_x, head_offset_y) = get_head_offset(player.animation.state, player.animation.direction, anim_frame);
                    let head_src_x = head_frame.frame as f32 * HEAD_SPRITE_WIDTH;
                    let scaled_head_width = HEAD_SPRITE_WIDTH * zoom;
                    let scaled_head_height = HEAD_SPRITE_HEIGHT * zoom;
                    let head_draw_x = draw_x + head_offset_x * zoom;
                    let head_draw_y = draw_y + head_offset_y * zoom;

                    draw_texture_ex(
                        &head_sprite,
                        head_draw_x,
                        head_draw_y,
                        tint,
                        DrawTextureParams {
                            source: Some(Rect::new(head_src_x, 0.0, HEAD_SPRITE_WIDTH, HEAD_SPRITE_HEIGHT)),
                            dest_size: Some(Vec2::new(scaled_head_width, scaled_head_height)),
                            flip_x: head_frame.flip_h,
                            ..Default::default()
                        },
                    );
                }
            } else {
                // No head equipment - draw hair normally
                if let (Some(style), Some(color)) = (player.hair_style, player.hair_color) {
                    if let Some(hair_tex) = self.hair_sprites.get(&style) {
                        let is_back = matches!(player.animation.direction, Direction::Up | Direction::Left);
                        let frame_index = color * 2 + if is_back { 1 } else { 0 };
                        let hair_src_x = frame_index as f32 * HAIR_SPRITE_WIDTH;
                        let scaled_hair_width = HAIR_SPRITE_WIDTH * zoom;
                        let scaled_hair_height = HAIR_SPRITE_HEIGHT * zoom;

                        let is_attack_frame_2 = player.animation.state == AnimationState::Attacking && (player.animation.frame as u32 % 2) == 1;
                        let is_shooting_bow = player.animation.state == AnimationState::ShootingBow;
                        let (hair_offset_x, hair_offset_y) = if is_attack_frame_2 {
                            let y_offset = if is_back { -2.0 } else { 2.0 };
                            let x_offset = if is_back {
                                if coords.flip_h { 5.0 } else { -5.0 }
                            } else {
                                if coords.flip_h { 6.0 } else { -6.0 }
                            };
                            (x_offset, y_offset)
                        } else if is_shooting_bow {
                            let x_offset = if is_back {
                                if coords.flip_h { 1.0 } else { -1.0 }
                            } else {
                                if coords.flip_h { 2.0 } else { -2.0 }
                            };
                            (x_offset, -3.0)
                        } else {
                            let x_offset = if is_back {
                                if coords.flip_h { 2.0 } else { -2.0 }
                            } else {
                                if coords.flip_h { 1.0 } else { -1.0 }
                            };
                            (x_offset, -3.0)
                        };

                        let hair_draw_x = draw_x + (scaled_sprite_width - scaled_hair_width) / 2.0 + hair_offset_x * zoom;
                        let hair_draw_y = draw_y + hair_offset_y * zoom;

                        draw_texture_ex(
                            hair_tex,
                            hair_draw_x,
                            hair_draw_y,
                            tint,
                            DrawTextureParams {
                                source: Some(Rect::new(hair_src_x, 0.0, HAIR_SPRITE_WIDTH, HAIR_SPRITE_HEIGHT)),
                                dest_size: Some(Vec2::new(scaled_hair_width, scaled_hair_height)),
                                flip_x: coords.flip_h,
                                ..Default::default()
                            },
                        );
                    }
                }
            }

            // Draw equipment overlay (body armor)
            if let Some(ref body_item_id) = player.equipped_body {
                if let Some(equip_sprite) = self.equipment_sprites.get(body_item_id) {
                    // Check if this is a new-style single-row body armor sprite (width > height * 2)
                    // Body armor sprites are wider (16 frames) so use a more aggressive ratio check
                    let is_single_row = equip_sprite.width() > equip_sprite.height() * 2.0;

                    if is_single_row {
                        // New single-row body armor format
                        let anim_frame = player.animation.frame as u32;
                        let armor_frame = get_body_armor_frame(player.animation.state, player.animation.direction, anim_frame);
                        let (armor_offset_x, armor_offset_y) = get_body_armor_offset(player.animation.state, player.animation.direction, anim_frame);

                        let armor_src_x = armor_frame.frame as f32 * BODY_ARMOR_SPRITE_WIDTH;
                        let scaled_armor_width = BODY_ARMOR_SPRITE_WIDTH * zoom;
                        let scaled_armor_height = BODY_ARMOR_SPRITE_HEIGHT * zoom;

                        let armor_draw_x = draw_x + armor_offset_x * zoom;
                        let armor_draw_y = draw_y + armor_offset_y * zoom;

                        draw_texture_ex(
                            equip_sprite,
                            armor_draw_x,
                            armor_draw_y,
                            tint,
                            DrawTextureParams {
                                source: Some(Rect::new(armor_src_x, 0.0, BODY_ARMOR_SPRITE_WIDTH, BODY_ARMOR_SPRITE_HEIGHT)),
                                dest_size: Some(Vec2::new(scaled_armor_width, scaled_armor_height)),
                                flip_x: armor_frame.flip_h,
                                ..Default::default()
                            },
                        );
                    } else {
                        // Old grid-style body armor format (matches player sprite sheet layout)
                        draw_texture_ex(
                            equip_sprite,
                            draw_x,
                            draw_y,
                            tint, // Same tint as player
                            DrawTextureParams {
                                source: Some(Rect::new(src_x, src_y, src_w, src_h)),
                                dest_size: Some(Vec2::new(scaled_sprite_width, scaled_sprite_height)),
                                flip_x: coords.flip_h,
                                ..Default::default()
                            },
                        );
                    }
                }
            }

            // Draw equipment overlay (boots)
            if let Some(ref feet_item_id) = player.equipped_feet {
                if let Some(equip_sprite) = self.equipment_sprites.get(feet_item_id) {
                    // Check if this is a new-style single-row boot sprite (width > height)
                    let is_single_row = equip_sprite.width() > equip_sprite.height();

                    if is_single_row {
                        // New single-row boot format
                        let anim_frame = player.animation.frame as u32;
                        let boot_frame = get_boot_frame(player.animation.state, player.animation.direction, anim_frame);
                        let (boot_offset_x, boot_offset_y) = get_boot_offset(player.animation.state, player.animation.direction, anim_frame);

                        let boot_src_x = boot_frame.frame as f32 * BOOT_SPRITE_WIDTH;
                        let scaled_boot_width = BOOT_SPRITE_WIDTH * zoom;
                        let scaled_boot_height = BOOT_SPRITE_HEIGHT * zoom;

                        let boot_draw_x = draw_x + boot_offset_x * zoom;
                        let boot_draw_y = draw_y + boot_offset_y * zoom;

                        draw_texture_ex(
                            equip_sprite,
                            boot_draw_x,
                            boot_draw_y,
                            tint,
                            DrawTextureParams {
                                source: Some(Rect::new(boot_src_x, 0.0, BOOT_SPRITE_WIDTH, BOOT_SPRITE_HEIGHT)),
                                dest_size: Some(Vec2::new(scaled_boot_width, scaled_boot_height)),
                                flip_x: boot_frame.flip_h,
                                ..Default::default()
                            },
                        );
                    } else {
                        // Old grid-style boot format (matches player sprite sheet layout)
                        draw_texture_ex(
                            equip_sprite,
                            draw_x,
                            draw_y,
                            tint,
                            DrawTextureParams {
                                source: Some(Rect::new(src_x, src_y, src_w, src_h)),
                                dest_size: Some(Vec2::new(scaled_sprite_width, scaled_sprite_height)),
                                flip_x: coords.flip_h,
                                ..Default::default()
                            },
                        );
                    }
                }
            }

            // Draw weapon over-layer (after equipment, for attack frame 2 front overlay)
            if let Some((weapon_sprite, ref weapon_frame, offset_x, offset_y)) = weapon_info {
                if let Some(frame_over) = weapon_frame.frame_over {
                    let weapon_src_x = frame_over as f32 * WEAPON_SPRITE_WIDTH;
                    let weapon_draw_x = draw_x + offset_x * zoom;
                    let weapon_draw_y = draw_y + offset_y * zoom;

                    draw_texture_ex(
                        weapon_sprite,
                        weapon_draw_x,
                        weapon_draw_y,
                        tint,
                        DrawTextureParams {
                            source: Some(Rect::new(weapon_src_x, 0.0, WEAPON_SPRITE_WIDTH, WEAPON_SPRITE_HEIGHT)),
                            dest_size: Some(Vec2::new(scaled_weapon_width, scaled_weapon_height)),
                            flip_x: weapon_frame.flip_h,
                            ..Default::default()
                        },
                    );
                }
            }
        } else {
            // Fallback: colored circle
            let base_color = if is_local {
                self.local_player_color
            } else {
                self.player_color
            };
            let color = Color::from_rgba(
                (base_color.r * 255.0) as u8,
                (base_color.g * 255.0) as u8,
                (base_color.b * 255.0) as u8,
                alpha,
            );

            let radius = 12.0 * zoom;
            draw_circle(screen_x, screen_y - radius, radius, color);

            // Direction indicator
            let (dx, dy) = player.direction.to_unit_vector();
            let indicator_len = 15.0 * zoom;
            draw_line(
                screen_x,
                screen_y - radius,
                screen_x + dx * indicator_len,
                screen_y - radius + dy * indicator_len * 0.5, // Flatten for isometric
                2.0 * zoom,
                WHITE,
            );
        }

        // Player name (positioned just above head) - only show when hovered or selected
        let has_sprite = self.get_player_sprite(&player.gender, &player.skin).is_some();
        let name_y_offset = if has_sprite { scaled_sprite_height - 8.0 * zoom } else { 24.0 * zoom };

        let show_name = is_selected || is_hovered;
        if show_name {
            // Build display name with optional (GM) suffix
            let name_width = self.measure_text_sharp(&player.name, 16.0).width;
            let gm_width = if player.is_admin { self.measure_text_sharp(" (GM)", 16.0).width - 2.0 } else { 0.0 };
            let total_width = name_width + gm_width;
            let name_x = screen_x - total_width / 2.0;
            let name_y = screen_y - name_y_offset + 2.0;

            // Background for readability
            let padding = 4.0;
            draw_rectangle(
                name_x - padding,
                name_y - 14.0,
                total_width + padding * 2.0,
                18.0,
                Color::from_rgba(0, 0, 0, 180),
            );

            // Draw player name in white
            self.draw_text_sharp(
                &player.name,
                name_x,
                name_y,
                16.0,
                WHITE,
            );

            // Draw (GM) suffix in gold if admin
            if player.is_admin {
                let gold_color = Color::from_rgba(255, 215, 0, 255);
                self.draw_text_sharp(
                    " (GM)",
                    name_x + name_width,
                    name_y,
                    16.0,
                    gold_color,
                );
            }
        }

        // Health bar - only show within 3 seconds of taking damage (and when not at full HP)
        let current_time = macroquad::time::get_time();
        let time_since_damage = current_time - player.last_damage_time;
        let show_health_bar = player.hp < player.max_hp && time_since_damage < 3.0;

        if show_health_bar {
            let bar_width = 32.0;
            let bar_height = 6.0;
            let bar_x = screen_x - bar_width / 2.0;
            // Position health bar where name would be if name isn't showing, otherwise above the name
            let bar_y = if show_name {
                screen_y - name_y_offset - 16.0
            } else {
                screen_y - name_y_offset
            };
            let hp_ratio = player.hp as f32 / player.max_hp.max(1) as f32;

            self.draw_entity_health_bar(bar_x, bar_y, bar_width, bar_height, hp_ratio, 1.0);
        }
    }

    /// Renders a semi-transparent silhouette of the player that's always visible
    /// This provides visual feedback when the player is behind tall objects like trees
    fn render_player_silhouette(&self, player: &Player, camera: &Camera) {
        // Don't show silhouette for dead players
        if player.is_dead {
            return;
        }

        let (screen_x, screen_y) = world_to_screen(player.x, player.y, camera);
        let zoom = camera.zoom;

        let scaled_sprite_width = SPRITE_WIDTH * zoom;
        let scaled_sprite_height = SPRITE_HEIGHT * zoom;

        // Subtle semi-transparent tint (~20% opacity)
        let silhouette_tint = Color::from_rgba(255, 255, 255, 50);

        if let Some(sprite) = self.get_player_sprite(&player.gender, &player.skin) {
            let coords = player.animation.get_sprite_coords();
            let (src_x, src_y, src_w, src_h) = coords.to_source_rect();

            let draw_x = screen_x - scaled_sprite_width / 2.0;
            let draw_y = screen_y - scaled_sprite_height + 8.0 * zoom;

            // Calculate weapon frame info if weapon is equipped
            let weapon_info = player.equipped_weapon.as_ref().and_then(|weapon_id| {
                self.weapon_sprites.get(weapon_id).map(|weapon_sprite| {
                    let anim_frame = player.animation.frame as u32;
                    let weapon_frame = get_weapon_frame(player.animation.state, player.animation.direction, anim_frame);
                    let (offset_x, offset_y) = get_weapon_offset(player.animation.state, player.animation.direction, anim_frame);
                    (weapon_sprite, weapon_frame, offset_x, offset_y)
                })
            });

            let scaled_weapon_width = WEAPON_SPRITE_WIDTH * zoom;
            let scaled_weapon_height = WEAPON_SPRITE_HEIGHT * zoom;

            // Draw weapon under-layer (before player sprite)
            if let Some((weapon_sprite, ref weapon_frame, offset_x, offset_y)) = weapon_info {
                let weapon_src_x = weapon_frame.frame_under as f32 * WEAPON_SPRITE_WIDTH;
                let weapon_draw_x = draw_x + offset_x * zoom;
                let weapon_draw_y = draw_y + offset_y * zoom;

                draw_texture_ex(
                    weapon_sprite,
                    weapon_draw_x,
                    weapon_draw_y,
                    silhouette_tint,
                    DrawTextureParams {
                        source: Some(Rect::new(weapon_src_x, 0.0, WEAPON_SPRITE_WIDTH, WEAPON_SPRITE_HEIGHT)),
                        dest_size: Some(Vec2::new(scaled_weapon_width, scaled_weapon_height)),
                        flip_x: weapon_frame.flip_h,
                        ..Default::default()
                    },
                );
            }

            // Draw player base sprite (skip if body armor or head slot equipped to avoid transparency stacking)
            if player.equipped_body.is_none() && player.equipped_head.is_none() {
                draw_texture_ex(
                    sprite,
                    draw_x,
                    draw_y,
                    silhouette_tint,
                    DrawTextureParams {
                        source: Some(Rect::new(src_x, src_y, src_w, src_h)),
                        dest_size: Some(Vec2::new(scaled_sprite_width, scaled_sprite_height)),
                        flip_x: coords.flip_h,
                        ..Default::default()
                    },
                );
            }

            // Draw hair silhouette (skip if head slot is equipped - helmet covers hair)
            const HAIR_SPRITE_WIDTH: f32 = 28.0;
            const HAIR_SPRITE_HEIGHT: f32 = 54.0;
            if player.equipped_head.is_none() {
                if let (Some(style), Some(color)) = (player.hair_style, player.hair_color) {
                    if let Some(hair_tex) = self.hair_sprites.get(&style) {
                        let is_back = matches!(player.animation.direction, Direction::Up | Direction::Left);
                        let frame_index = color * 2 + if is_back { 1 } else { 0 };
                        let hair_src_x = frame_index as f32 * HAIR_SPRITE_WIDTH;

                        let scaled_hair_width = HAIR_SPRITE_WIDTH * zoom;
                        let scaled_hair_height = HAIR_SPRITE_HEIGHT * zoom;

                        // Hair offset based on direction (silhouette uses normal offsets)
                        let x_offset = if is_back {
                            if coords.flip_h { 2.0 } else { -2.0 }  // Back directions: left 2
                        } else {
                            if coords.flip_h { 1.0 } else { -1.0 }  // Front directions: left 1
                        };
                        let hair_draw_x = draw_x + (scaled_sprite_width - scaled_hair_width) / 2.0 + x_offset * zoom;
                        let hair_draw_y = draw_y - 3.0 * zoom;

                        draw_texture_ex(
                            hair_tex,
                            hair_draw_x,
                            hair_draw_y,
                            silhouette_tint,
                            DrawTextureParams {
                                source: Some(Rect::new(hair_src_x, 0.0, HAIR_SPRITE_WIDTH, HAIR_SPRITE_HEIGHT)),
                                dest_size: Some(Vec2::new(scaled_hair_width, scaled_hair_height)),
                                flip_x: coords.flip_h,
                                ..Default::default()
                            },
                        );
                    }
                }
            }

            // Draw equipment silhouette (body armor)
            if let Some(ref body_item_id) = player.equipped_body {
                if let Some(equip_sprite) = self.equipment_sprites.get(body_item_id) {
                    let is_single_row = equip_sprite.width() > equip_sprite.height() * 2.0;

                    if is_single_row {
                        // New single-row body armor format
                        let anim_frame = player.animation.frame as u32;
                        let armor_frame = get_body_armor_frame(player.animation.state, player.animation.direction, anim_frame);
                        let (armor_offset_x, armor_offset_y) = get_body_armor_offset(player.animation.state, player.animation.direction, anim_frame);

                        let armor_src_x = armor_frame.frame as f32 * BODY_ARMOR_SPRITE_WIDTH;
                        let scaled_armor_width = BODY_ARMOR_SPRITE_WIDTH * zoom;
                        let scaled_armor_height = BODY_ARMOR_SPRITE_HEIGHT * zoom;

                        let armor_draw_x = draw_x + armor_offset_x * zoom;
                        let armor_draw_y = draw_y + armor_offset_y * zoom;

                        draw_texture_ex(
                            equip_sprite,
                            armor_draw_x,
                            armor_draw_y,
                            silhouette_tint,
                            DrawTextureParams {
                                source: Some(Rect::new(armor_src_x, 0.0, BODY_ARMOR_SPRITE_WIDTH, BODY_ARMOR_SPRITE_HEIGHT)),
                                dest_size: Some(Vec2::new(scaled_armor_width, scaled_armor_height)),
                                flip_x: armor_frame.flip_h,
                                ..Default::default()
                            },
                        );
                    } else {
                        // Old grid-style body armor format
                        draw_texture_ex(
                            equip_sprite,
                            draw_x,
                            draw_y,
                            silhouette_tint,
                            DrawTextureParams {
                                source: Some(Rect::new(src_x, src_y, src_w, src_h)),
                                dest_size: Some(Vec2::new(scaled_sprite_width, scaled_sprite_height)),
                                flip_x: coords.flip_h,
                                ..Default::default()
                            },
                        );
                    }
                }
            }

            // Draw equipment silhouette (boots)
            if let Some(ref feet_item_id) = player.equipped_feet {
                if let Some(equip_sprite) = self.equipment_sprites.get(feet_item_id) {
                    let is_single_row = equip_sprite.width() > equip_sprite.height();

                    if is_single_row {
                        // New single-row boot format
                        let anim_frame = player.animation.frame as u32;
                        let boot_frame = get_boot_frame(player.animation.state, player.animation.direction, anim_frame);
                        let (boot_offset_x, boot_offset_y) = get_boot_offset(player.animation.state, player.animation.direction, anim_frame);

                        let boot_src_x = boot_frame.frame as f32 * BOOT_SPRITE_WIDTH;
                        let scaled_boot_width = BOOT_SPRITE_WIDTH * zoom;
                        let scaled_boot_height = BOOT_SPRITE_HEIGHT * zoom;

                        let boot_draw_x = draw_x + boot_offset_x * zoom;
                        let boot_draw_y = draw_y + boot_offset_y * zoom;

                        draw_texture_ex(
                            equip_sprite,
                            boot_draw_x,
                            boot_draw_y,
                            silhouette_tint,
                            DrawTextureParams {
                                source: Some(Rect::new(boot_src_x, 0.0, BOOT_SPRITE_WIDTH, BOOT_SPRITE_HEIGHT)),
                                dest_size: Some(Vec2::new(scaled_boot_width, scaled_boot_height)),
                                flip_x: boot_frame.flip_h,
                                ..Default::default()
                            },
                        );
                    } else {
                        // Old grid-style boot format
                        draw_texture_ex(
                            equip_sprite,
                            draw_x,
                            draw_y,
                            silhouette_tint,
                            DrawTextureParams {
                                source: Some(Rect::new(src_x, src_y, src_w, src_h)),
                                dest_size: Some(Vec2::new(scaled_sprite_width, scaled_sprite_height)),
                                flip_x: coords.flip_h,
                                ..Default::default()
                            },
                        );
                    }
                }
            }


            // Draw weapon over-layer (after equipment)
            if let Some((weapon_sprite, ref weapon_frame, offset_x, offset_y)) = weapon_info {
                if let Some(frame_over) = weapon_frame.frame_over {
                    let weapon_src_x = frame_over as f32 * WEAPON_SPRITE_WIDTH;
                    let weapon_draw_x = draw_x + offset_x * zoom;
                    let weapon_draw_y = draw_y + offset_y * zoom;

                    draw_texture_ex(
                        weapon_sprite,
                        weapon_draw_x,
                        weapon_draw_y,
                        silhouette_tint,
                        DrawTextureParams {
                            source: Some(Rect::new(weapon_src_x, 0.0, WEAPON_SPRITE_WIDTH, WEAPON_SPRITE_HEIGHT)),
                            dest_size: Some(Vec2::new(scaled_weapon_width, scaled_weapon_height)),
                            flip_x: weapon_frame.flip_h,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }

    fn render_npc(&self, npc: &Npc, is_selected: bool, is_hovered: bool, camera: &Camera) {
        let (screen_x, screen_y) = world_to_screen(npc.x, npc.y, camera);
        let zoom = camera.zoom;

        // Don't render if death animation is complete
        if npc.is_death_animation_complete() {
            return;
        }

        // Get death tint color if dying, otherwise white
        let tint_color = npc.get_death_color().unwrap_or(WHITE);

        // Selection highlight (draw first, behind NPC) - skip while dying
        if is_selected && npc.death_timer.is_none() {
            self.render_tile_selection(npc.x, npc.y, camera);
        }

        // Name color based on NPC type
        let name_color = if npc.is_hostile() {
            Color::from_rgba(255, 150, 150, 255) // Red for hostile
        } else if npc.is_quest_giver {
            Color::from_rgba(150, 220, 255, 255) // Cyan for quest givers
        } else if npc.is_merchant {
            Color::from_rgba(150, 255, 150, 255) // Light green for merchants
        } else {
            Color::from_rgba(255, 255, 255, 255) // White for other friendly NPCs
        };

        // Try to render with sprite, fall back to ellipse
        let sprite_height = if let Some(sprite) = self.npc_sprites.get(&npc.entity_type) {
            // Auto-detect frame size from texture (16 frames per sheet)
            let frame_width = sprite.width() / 16.0;
            let frame_height = sprite.height();

            // Get current frame based on animation state and direction
            let frame_index = npc.animation.get_frame_index(npc.direction);
            let src_x = frame_index as f32 * frame_width;

            // Flip horizontally for Right/Left directions
            let flip_x = NpcAnimation::should_flip(npc.direction);

            // Position sprite centered horizontally, feet at world position
            // Round to whole pixels to avoid blurry rendering from subpixel positioning
            let scaled_width = (frame_width * zoom).round();
            let scaled_height = (frame_height * zoom).round();
            let draw_x = (screen_x - scaled_width / 2.0).round();
            let draw_y = (screen_y - scaled_height + 4.0 * zoom).round();

            // Draw shadow
            let shadow_scale = (frame_width / 50.0).clamp(0.5, 2.0);
            draw_ellipse(
                screen_x,
                screen_y,
                16.0 * shadow_scale * zoom,
                6.0 * shadow_scale * zoom,
                0.0,
                Color::from_rgba(0, 0, 0, 60),
            );

            draw_texture_ex(
                sprite,
                draw_x,
                draw_y,
                tint_color,
                DrawTextureParams {
                    source: Some(Rect::new(src_x, 0.0, frame_width, frame_height)),
                    dest_size: Some(Vec2::new(scaled_width, scaled_height)),
                    flip_x,
                    ..Default::default()
                },
            );

            scaled_height
        } else {
            // Fallback: colored ellipse rendering
            let (mut base_color, mut highlight_color) = if npc.is_hostile() {
                (
                    Color::from_rgba(80, 180, 80, 255),
                    Color::from_rgba(120, 220, 120, 255),
                )
            } else {
                (
                    Color::from_rgba(100, 120, 200, 255),
                    Color::from_rgba(140, 160, 240, 255),
                )
            };

            // Apply death tint to ellipse colors
            base_color.r *= tint_color.r;
            base_color.g *= tint_color.g;
            base_color.b *= tint_color.b;
            base_color.a *= tint_color.a;
            highlight_color.r *= tint_color.r;
            highlight_color.g *= tint_color.g;
            highlight_color.b *= tint_color.b;
            highlight_color.a *= tint_color.a;

            let wobble = (macroquad::time::get_time() * 4.0 + npc.animation.frame as f64).sin() as f32;
            let radius = (10.0 + wobble * 1.5) * zoom;
            let height_offset = (8.0 + wobble * 2.0) * zoom;

            // Draw shadow
            draw_ellipse(screen_x, screen_y, 16.0 * zoom, 6.0 * zoom, 0.0, Color::from_rgba(0, 0, 0, 60));

            // Draw NPC body (oval blob)
            draw_ellipse(screen_x, screen_y - height_offset, radius, radius * 0.7, 0.0, base_color);

            // Highlight
            draw_ellipse(screen_x - 3.0 * zoom, screen_y - height_offset - 2.0 * zoom, radius * 0.3, radius * 0.2, 0.0, highlight_color);

            (height_offset + radius) * 2.0
        };

        // Skip UI elements (name, health bar, icons) while dying
        if npc.death_timer.is_some() {
            return;
        }

        // Top of NPC for UI elements
        let top_y = screen_y - sprite_height + 4.0 * zoom;

        // Determine icon coords for friendly NPCs (quest givers only)
        let icon_coords: Option<(u32, u32)> = if !npc.is_hostile() && npc.is_quest_giver {
            Some((8, 3))  // Quest giver icon
        } else {
            None
        };

        // Floating icon indicator - only when NOT hovered (when hovered, icon is in name bar)
        if !is_hovered && !is_selected {
            if let (Some((icon_col, icon_row)), Some(ref texture)) = (icon_coords, &self.ui_icons) {
                let icon_size = 24.0;
                let time = macroquad::time::get_time();

                // Use NPC position as offset so icons don't animate in sync
                let phase_offset = (npc.x + npc.y * 1.7) as f64;

                // Pulsing transparency (2 second cycle, 80-100% opacity)
                let alpha_pulse = ((time * 3.14 + phase_offset).sin() * 0.5 + 0.5) as f32;
                let alpha = (204.0 + alpha_pulse * 51.0) as u8; // 204-255 (80-100%)

                let icon_x = screen_x - (icon_size * zoom) / 2.0;
                let icon_y = top_y - 20.0 * zoom;

                let src_rect = Rect::new(
                    icon_col as f32 * icon_size,
                    icon_row as f32 * icon_size,
                    icon_size,
                    icon_size,
                );

                draw_texture_ex(
                    texture,
                    icon_x,
                    icon_y,
                    Color::from_rgba(255, 255, 255, alpha),
                    DrawTextureParams {
                        source: Some(src_rect),
                        dest_size: Some(Vec2::new(icon_size * zoom, icon_size * zoom)),
                        ..Default::default()
                    },
                );
            }
        }

        // NPC name with level - only show when hovered or selected
        let show_name = is_selected || is_hovered;
        if show_name {
            let name = npc.name();
            let name_width = self.measure_text_sharp(&name, 16.0).width;
            let name_y = top_y - 5.0 * zoom;
            let padding = 4.0;

            // Get the small icon texture for the name bar (quest givers only)
            let small_icon: Option<&Texture2D> = if npc.is_quest_giver {
                self.chat_small_icon.as_ref()
            } else {
                None
            };

            let icon_gap = 4.0;  // Gap between icon and text

            // Calculate total width and starting position
            let (total_width, icon_width) = if let Some(tex) = small_icon {
                let w = tex.width();
                (w + icon_gap + name_width, w)
            } else {
                (name_width, 0.0)
            };
            let content_x = screen_x - total_width / 2.0;

            // Background for readability
            let bar_height = 18.0;
            draw_rectangle(
                content_x - padding,
                name_y - 14.0,
                total_width + padding * 2.0,
                bar_height,
                Color::from_rgba(0, 0, 0, 180),
            );

            // Draw small icon if present
            if let Some(tex) = small_icon {
                let icon_h = tex.height();
                // Center icon vertically in the bar
                let bar_top = name_y - 14.0;
                let icon_y = bar_top + (bar_height - icon_h) / 2.0;

                draw_texture_ex(
                    tex,
                    content_x,
                    icon_y,
                    WHITE,
                    DrawTextureParams::default(),
                );
            }

            // Draw name text (offset by icon if present)
            let text_x = if small_icon.is_some() {
                content_x + icon_width + icon_gap
            } else {
                content_x
            };

            self.draw_text_sharp(
                &name,
                text_x,
                name_y,
                16.0,
                name_color,
            );
        }

        // Health bar - only show within 3 seconds of taking damage (and when not at full HP)
        let current_time = macroquad::time::get_time();
        let time_since_damage = current_time - npc.last_damage_time;
        let show_health_bar = npc.hp < npc.max_hp && time_since_damage < 3.0;

        if show_health_bar {
            let bar_width = 30.0 * zoom;
            let bar_height = 5.0 * zoom;
            let bar_x = screen_x - bar_width / 2.0;
            // Position health bar where name would be if name isn't showing, otherwise above the name
            let bar_y = if show_name {
                top_y - 20.0 * zoom
            } else {
                top_y - 5.0 * zoom
            };
            let hp_ratio = npc.hp as f32 / npc.max_hp.max(1) as f32;

            self.draw_entity_health_bar(bar_x, bar_y, bar_width, bar_height, hp_ratio, zoom);
        }
    }

    fn render_ground_item(&self, item: &GroundItem, camera: &Camera, state: &GameState) {
        // Special rendering for gold piles
        if item.item_id == "gold" && item.gold_pile.is_some() {
            self.render_gold_pile(item, camera);
            return;
        }

        let (screen_x, screen_y) = world_to_screen(item.x, item.y, camera);
        let zoom = camera.zoom;

        // Bobbing animation
        let time = macroquad::time::get_time();
        let bob = ((time - item.animation_time) * 3.0).sin() as f32 * 2.0 * zoom;

        let item_def = state.item_registry.get_or_placeholder(&item.item_id);
        let item_y = screen_y - 8.0 * zoom - bob;

        // Try to use item sprite, fall back to colored rectangle
        if let Some(texture) = self.item_sprites.get(&item.item_id) {
            // Use full texture, centered on the ground position
            let icon_width = texture.width() * zoom;
            let icon_height = texture.height() * zoom;

            draw_texture_ex(
                texture,
                screen_x - icon_width / 2.0,
                item_y - icon_height / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(icon_width, icon_height)),
                    ..Default::default()
                },
            );
        } else {
            // Fallback to colored rectangle
            let color = item_def.category_color();
            draw_rectangle(screen_x - 6.0 * zoom, item_y - 6.0 * zoom, 16.0 * zoom, 12.0 * zoom, color);
            draw_rectangle_lines(screen_x - 6.0 * zoom, item_y - 6.0 * zoom, 16.0 * zoom, 12.0 * zoom, 1.0, WHITE);
        }
    }

    /// Render a gold pile with multiple animated nuggets
    fn render_gold_pile(&self, item: &GroundItem, camera: &Camera) {
        let (screen_x, screen_y) = world_to_screen(item.x, item.y, camera);
        let zoom = camera.zoom;
        let time = macroquad::time::get_time();

        let pile = match &item.gold_pile {
            Some(p) => p,
            None => return,
        };

        let texture = match &self.gold_nugget_texture {
            Some(t) => t,
            None => return,
        };

        let elapsed = time - pile.spawn_time;

        // Animation constants
        const SPAWN_DURATION: f64 = 0.5;
        const STAGGER_DELAY: f64 = 0.03;
        const BOB_SPEED: f64 = 2.5;
        const BOB_AMPLITUDE: f32 = 1.5;

        // Sort nuggets by Y offset for proper depth (back to front)
        let mut sorted_indices: Vec<usize> = (0..pile.nuggets.len()).collect();
        sorted_indices.sort_by(|&a, &b| {
            pile.nuggets[a]
                .target_y
                .partial_cmp(&pile.nuggets[b].target_y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Render each nugget
        for (render_idx, &nugget_idx) in sorted_indices.iter().enumerate() {
            let nugget = &pile.nuggets[nugget_idx];

            // Calculate spawn progress with stagger
            let nugget_elapsed = elapsed - (render_idx as f64 * STAGGER_DELAY);
            let spawn_t = (nugget_elapsed / SPAWN_DURATION).clamp(0.0, 1.0) as f32;
            // Ease-out cubic
            let ease_t = 1.0 - (1.0 - spawn_t).powi(3);

            // Interpolate from burst position to target
            let current_x = nugget.offset_x + (nugget.target_x - nugget.offset_x) * ease_t;
            let current_y = nugget.offset_y + (nugget.target_y - nugget.offset_y) * ease_t;

            // Bob animation (only after mostly settled)
            let bob = if spawn_t > 0.7 {
                let bob_strength = ((spawn_t - 0.7) / 0.3).min(1.0);
                ((time * BOB_SPEED + nugget.phase_offset).sin() as f32) * BOB_AMPLITUDE * zoom * bob_strength
            } else {
                0.0
            };

            // Calculate final screen position
            let nugget_x = screen_x + current_x * zoom;
            let nugget_y = screen_y + current_y * zoom - bob - 4.0 * zoom;

            // Draw nugget sprite
            let width = texture.width() * zoom;
            let height = texture.height() * zoom;

            draw_texture_ex(
                texture,
                nugget_x - width / 2.0,
                nugget_y - height / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(width, height)),
                    ..Default::default()
                },
            );
        }
    }

    /// Render a map object (tree, rock, decoration) from chunk data
    fn render_map_object(&self, obj: &MapObject, camera: &Camera) {
        // Get screen position for the tile CENTER (add 0.5 to tile coords)
        let (screen_x, screen_y) = world_to_screen(obj.tile_x as f32 + 0.5, obj.tile_y as f32 + 0.5, camera);
        let zoom = camera.zoom;

        // Try to get the sprite for this gid
        if let Some(texture) = self.get_object_sprite(obj.gid) {
            let tex_width = texture.width();
            let tex_height = texture.height();

            // Scale the sprite (round to avoid fractional scaling artifacts)
            let scaled_width = (tex_width * zoom).round();
            let scaled_height = (tex_height * zoom).round();

            // Position sprite so its bottom-center aligns with the tile center
            // Round to pixel grid to avoid subpixel rendering artifacts
            let draw_x = (screen_x - scaled_width / 2.0).round();
            let draw_y = (screen_y - scaled_height).round();

            draw_texture_ex(
                texture,
                draw_x,
                draw_y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(scaled_width, scaled_height)),
                    ..Default::default()
                },
            );
        } else {
            // Fallback: draw a colored placeholder rectangle
            let placeholder_width = (32.0 * zoom).round();
            let placeholder_height = (64.0 * zoom).round();
            let draw_x = (screen_x - placeholder_width / 2.0).round();
            let draw_y = (screen_y - placeholder_height).round();

            draw_rectangle(
                draw_x,
                draw_y,
                placeholder_width,
                placeholder_height,
                Color::from_rgba(100, 150, 100, 200),
            );
            draw_rectangle_lines(
                draw_x,
                draw_y,
                placeholder_width,
                placeholder_height,
                2.0,
                Color::from_rgba(50, 100, 50, 255),
            );
        }
    }

    fn render_ui(&self, state: &GameState) {
        // Server announcements (top of screen)
        let current_time = macroquad::time::get_time();
        for (i, announcement) in state.ui_state.announcements.iter().enumerate() {
            let age = current_time - announcement.time;
            // Fade out after 6 seconds (announcements last 8 seconds total)
            let alpha = if age > 6.0 { ((8.0 - age) / 2.0 * 255.0) as u8 } else { 255 };

            let font_size = 32.0;
            let text = format!("[ANNOUNCEMENT] {}", announcement.text);
            let text_dims = self.measure_text_sharp(&text, font_size);
            let text_x = (screen_width() - text_dims.width) / 2.0;
            let text_y = 50.0 + (i as f32 * 35.0);

            // Dark background for visibility
            let padding = 10.0;
            let rect_h = text_dims.height + padding;
            let rect_y = text_y - text_dims.offset_y - padding / 2.0;
            draw_rectangle(
                text_x - padding,
                rect_y,
                text_dims.width + padding * 2.0,
                rect_h,
                Color::from_rgba(0, 0, 0, (180.0 * alpha as f32 / 255.0) as u8),
            );

            // Gold text with black outline
            let gold_color = Color::from_rgba(255, 215, 0, alpha);
            for ox in [-1.0, 1.0] {
                for oy in [-1.0, 1.0] {
                    self.draw_text_sharp(&text, text_x + ox, text_y + oy, font_size, Color::from_rgba(0, 0, 0, alpha));
                }
            }
            self.draw_text_sharp(&text, text_x, text_y, font_size, gold_color);
        }

        // "You Died" overlay for local player
        if let Some(player) = state.get_local_player() {
            if player.is_dead {
                // Dark overlay
                draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 150));

                // "YOU DIED" text
                let text = "YOU DIED";
                let font_size = 64.0;
                let text_dims = self.measure_text_sharp(text, font_size);
                let text_x = (screen_width() - text_dims.width) / 2.0;
                let text_y = screen_height() / 2.0 - 20.0;

                // Red text with outline
                for ox in [-2.0, 2.0] {
                    for oy in [-2.0, 2.0] {
                        self.draw_text_sharp(text, text_x + ox, text_y + oy, font_size, BLACK);
                    }
                }
                self.draw_text_sharp(text, text_x, text_y, font_size, RED);

                // Respawn countdown (5 seconds)
                let time_since_death = macroquad::time::get_time() - player.death_time;
                let respawn_time = 5.0 - time_since_death;
                if respawn_time > 0.0 {
                    let countdown_text = format!("Respawning in {:.1}s", respawn_time);
                    let countdown_dims = self.measure_text_sharp(&countdown_text, 16.0);
                    self.draw_text_sharp(
                        &countdown_text,
                        (screen_width() - countdown_dims.width) / 2.0,
                        text_y + 50.0,
                        24.0,
                        WHITE,
                    );
                }
            }
        }

        // Chat messages (bottom-left) with text wrapping
        let chat_x = 10.0;
        let chat_y = screen_height() - 20.0;
        let line_height = 18.0;
        let max_chat_width = 400.0;
        let font_size = 16.0;

        let mut current_y = chat_y;
        for msg in state.ui_state.chat_messages.iter().rev().take(5) {
            // Channel-specific formatting and colors
            let (color, text) = match msg.channel {
                ChatChannel::Local => (WHITE, format!("{}: {}", msg.sender_name, msg.text)),
                ChatChannel::Global => (SKYBLUE, format!("[G] {}: {}", msg.sender_name, msg.text)),
                ChatChannel::System => (YELLOW, format!("{} {}", msg.sender_name, msg.text)),
            };
            let wrapped_lines = self.wrap_text(&text, max_chat_width, font_size);

            // Draw lines from bottom to top (reversed)
            for line in wrapped_lines.iter().rev() {
                self.draw_text_sharp(line, chat_x, current_y, font_size, color);
                current_y -= line_height;
            }
        }

        // Local player stats panel (top-right) - Name, HP bar
        if let Some(player) = state.get_local_player() {
            let panel_width = 160.0;
            let panel_height = 44.0;
            let panel_x = (screen_width() - panel_width - 12.0).floor();
            let panel_y = 25.0;

            // ===== PANEL BACKGROUND (same style as menu buttons, no hover) =====
            let border_alpha = Color::new(SLOT_BORDER.r, SLOT_BORDER.g, SLOT_BORDER.b, 0.9);
            draw_rectangle(panel_x - 1.0, panel_y - 1.0, panel_width + 2.0, panel_height + 2.0, border_alpha);
            let bg_alpha = Color::new(SLOT_BG_EMPTY.r, SLOT_BG_EMPTY.g, SLOT_BG_EMPTY.b, 0.85);
            draw_rectangle(panel_x, panel_y, panel_width, panel_height, bg_alpha);

            let padding = 8.0;
            let bar_width = panel_width - padding * 2.0;
            let bar_height = 16.0;

            // ===== PLAYER NAME + LEVEL =====
            let name_y = panel_y + 6.0;
            let name = &player.name;
            let level_text = format!(" Lv.{}", player.skills.total_level());
            self.draw_text_sharp(name, panel_x + padding, (name_y + 12.0).floor(), 16.0, TEXT_TITLE);
            let name_w = self.measure_text_sharp(name, 16.0).width;
            self.draw_text_sharp(&level_text, panel_x + padding + name_w, (name_y + 12.0).floor(), 16.0, TEXT_DIM);

            // ===== HP BAR =====
            let hp_bar_x = panel_x + padding - 2.0;
            let hp_bar_y = name_y + 18.0;
            let hp_ratio = player.hp as f32 / player.max_hp.max(1) as f32;

            draw_rectangle(hp_bar_x, hp_bar_y, bar_width, bar_height, SLOT_INNER_SHADOW);
            draw_rectangle(hp_bar_x + 1.0, hp_bar_y + 1.0, bar_width - 2.0, bar_height - 2.0, Color::new(0.08, 0.08, 0.10, 1.0));

            let hp_fill_w = (bar_width - 4.0) * hp_ratio;
            if hp_fill_w > 0.0 {
                let hp_color = if hp_ratio > 0.5 {
                    Color::new(0.2, 0.7, 0.3, 1.0)
                } else if hp_ratio > 0.25 {
                    Color::new(0.8, 0.6, 0.1, 1.0)
                } else {
                    Color::new(0.8, 0.2, 0.2, 1.0)
                };
                draw_rectangle(hp_bar_x + 2.0, hp_bar_y + 2.0, hp_fill_w, bar_height - 4.0, hp_color);
                draw_rectangle(hp_bar_x + 2.0, hp_bar_y + 2.0, hp_fill_w, (bar_height - 4.0) / 2.0, Color::new(1.0, 1.0, 1.0, 0.25));
            }

            let hp_text = format!("{}/{}", player.hp, player.max_hp);
            let hp_text_w = self.measure_text_sharp(&hp_text, 16.0).width;
            self.draw_text_sharp(&hp_text, (hp_bar_x + (bar_width - hp_text_w) / 2.0).floor(), (hp_bar_y + 13.0).floor(), 16.0, TEXT_NORMAL);
        }

        // Note: Interactive UI (inventory, crafting, dialogue, quick slots) is rendered
        // by render_interactive_ui() which is called by the main render loop

        // Chat input box (when open)
        if state.ui_state.chat_open {
            let input_x = 10.0;
            let input_y = screen_height() - 50.0;
            let input_width = 400.0;
            let input_height = 24.0;
            let text_padding = 5.0;
            let text_area_width = input_width - text_padding * 2.0 - 12.0; // Extra margin for scroll indicators
            let font_size = 16.0;

            // Background
            draw_rectangle(input_x, input_y, input_width, input_height, Color::from_rgba(0, 0, 0, 180));
            draw_rectangle_lines(input_x, input_y, input_width, input_height, 1.0, WHITE);

            let input_text = &state.ui_state.chat_input;
            let cursor_pos = state.ui_state.chat_cursor;
            let char_count = input_text.chars().count();

            // Calculate how many chars fit by measuring actual text width
            let measure_chars_that_fit = |text: &str, max_width: f32| -> usize {
                let chars: Vec<char> = text.chars().collect();
                for i in (1..=chars.len()).rev() {
                    let substr: String = chars[..i].iter().collect();
                    if self.measure_text_sharp(&substr, font_size).width <= max_width {
                        return i;
                    }
                }
                0
            };

            // Determine scroll offset to keep cursor visible
            let scroll_offset = if self.measure_text_sharp(input_text, font_size).width <= text_area_width {
                // Text fits entirely, no scroll needed
                0
            } else {
                // Find offset that keeps cursor visible
                // Start by trying to show text ending at cursor
                let text_to_cursor: String = input_text.chars().take(cursor_pos).collect();
                let cursor_text_width = self.measure_text_sharp(&text_to_cursor, font_size).width;

                if cursor_text_width <= text_area_width {
                    // Cursor is visible from start
                    0
                } else {
                    // Need to scroll - find how many chars to skip to show cursor
                    let chars: Vec<char> = input_text.chars().collect();
                    let mut offset = 0;
                    for i in 0..cursor_pos {
                        let visible: String = chars[i..cursor_pos].iter().collect();
                        if self.measure_text_sharp(&visible, font_size).width <= text_area_width {
                            offset = i;
                            break;
                        }
                    }
                    offset
                }
            };

            // Get visible portion of text that fits
            let chars_from_offset: String = input_text.chars().skip(scroll_offset).collect();
            let visible_char_count = measure_chars_that_fit(&chars_from_offset, text_area_width);
            let visible_text: String = input_text.chars().skip(scroll_offset).take(visible_char_count).collect();
            let visible_end = scroll_offset + visible_char_count;

            // Draw visible text
            self.draw_text_sharp(&visible_text, input_x + text_padding, input_y + 17.0, font_size, WHITE);

            // Draw scroll indicators if text is clipped
            if scroll_offset > 0 {
                self.draw_text_sharp("<", input_x + 1.0, input_y + 17.0, font_size, GRAY);
            }
            if visible_end < char_count {
                self.draw_text_sharp(">", input_x + input_width - 10.0, input_y + 17.0, font_size, GRAY);
            }

            // Blinking cursor at correct position within visible text
            let cursor_blink = (macroquad::time::get_time() * 2.0) as i32 % 2 == 0;
            if cursor_blink {
                let cursor_visible_pos = cursor_pos.saturating_sub(scroll_offset);
                let text_before_cursor: String = visible_text.chars().take(cursor_visible_pos).collect();
                let cursor_x = self.measure_text_sharp(&text_before_cursor, font_size).width;
                draw_line(
                    input_x + text_padding + cursor_x + 1.0,
                    input_y + 4.0,
                    input_x + text_padding + cursor_x + 1.0,
                    input_y + input_height - 4.0,
                    1.0,
                    WHITE,
                );
            }

            // Hint
            self.draw_text_sharp("Press Enter to send, Escape to cancel", input_x, input_y + input_height + 12.0, 16.0, LIGHTGRAY);
        }
    }


    /// Render all interactive UI elements and return the layout for hit detection
    fn render_interactive_ui(&self, state: &GameState) -> UiLayout {
        let mut layout = UiLayout::new();
        let hovered = &state.ui_state.hovered_element;

        // Ground item clickable areas and hover labels (world-space, registered first)
        self.render_ground_item_overlays(state, hovered, &mut layout);

        // Experience bar (at the very bottom, rendered first)
        self.render_exp_bar(state);

        // Inventory UI (when open)
        if state.ui_state.inventory_open {
            self.render_inventory(state, hovered, &mut layout);
        }

        // Quest Log UI (when open)
        if state.ui_state.quest_log_open {
            self.render_quest_log(state, hovered, &mut layout);
        }

        // Crafting UI (when open)
        if state.ui_state.crafting_open {
            self.render_crafting(state, hovered, &mut layout);
        }

        // Skills panel (when open)
        self.render_skills_panel(state, hovered, &mut layout);

        // Character panel (when open)
        self.render_character_panel(state, hovered, &mut layout);

        // Quick slots (always visible at bottom, above exp bar)
        self.render_quick_slots(state, hovered, &mut layout);

        // Menu buttons (bottom-right, above exp bar)
        self.render_menu_buttons(state, hovered, &mut layout);

        // Quest objective tracker (top-left)
        self.render_quest_tracker(state);

        // Quest completion notifications
        self.render_quest_completed(state);

        // Dialogue box (when active)
        if let Some(dialogue) = &state.ui_state.active_dialogue {
            self.render_dialogue(dialogue, hovered, &mut layout);
        }

        // Gold drop dialog (when active)
        if let Some(ref dialog) = state.ui_state.gold_drop_dialog {
            self.render_gold_drop_dialog(dialog, state.inventory.gold, hovered, &mut layout);
        }

        // Render context menu on top of everything
        if let Some(ref context_menu) = state.ui_state.context_menu {
            self.render_context_menu(context_menu, state, &mut layout);
        } else {
            // Only render tooltips if context menu is not open
            self.render_item_tooltip(state);
            self.render_skill_tooltip(state, hovered);
        }

        // Render dragged item at cursor (on top of everything)
        if let Some(ref drag) = state.ui_state.drag_state {
            self.render_dragged_item(drag, state);
        }

        // Render escape menu on top of everything
        if state.ui_state.escape_menu_open {
            self.render_escape_menu(state, &mut layout);
        }

        layout
    }

    // ========================================================================
    // Inventory UI Helper Functions
    // ========================================================================

    /// Draw the multi-layer medieval panel frame
    pub(crate) fn draw_panel_frame(&self, x: f32, y: f32, w: f32, h: f32) {
        // Layer 1: Outer dark shadow (gives depth from background)
        draw_rectangle(x - 2.0, y - 2.0, w + 4.0, h + 4.0, PANEL_BG_DARK);

        // Layer 2: Dark bronze outer frame
        draw_rectangle(x, y, w, h, FRAME_OUTER);

        // Layer 3: Mid bronze frame (inset 2px)
        draw_rectangle(x + 2.0, y + 2.0, w - 4.0, h - 4.0, FRAME_MID);

        // Layer 4: Main panel background (inset 4px)
        draw_rectangle(x + FRAME_THICKNESS, y + FRAME_THICKNESS, w - FRAME_THICKNESS * 2.0, h - FRAME_THICKNESS * 2.0, PANEL_BG_MID);

        // Layer 5: Inner highlight line (top and left edges - light source simulation)
        draw_line(x + FRAME_THICKNESS, y + FRAME_THICKNESS, x + w - FRAME_THICKNESS, y + FRAME_THICKNESS, 1.0, FRAME_INNER);
        draw_line(x + FRAME_THICKNESS, y + FRAME_THICKNESS, x + FRAME_THICKNESS, y + h - FRAME_THICKNESS, 1.0, FRAME_INNER);

        // Layer 6: Inner shadow line (bottom and right edges)
        let shadow = Color::new(0.0, 0.0, 0.0, 0.235);
        draw_line(x + FRAME_THICKNESS + 1.0, y + h - FRAME_THICKNESS - 1.0, x + w - FRAME_THICKNESS, y + h - FRAME_THICKNESS - 1.0, 1.0, shadow);
        draw_line(x + w - FRAME_THICKNESS - 1.0, y + FRAME_THICKNESS + 1.0, x + w - FRAME_THICKNESS - 1.0, y + h - FRAME_THICKNESS, 1.0, shadow);
    }

    /// Draw decorative corner accents (gold L-shapes at corners)
    pub(crate) fn draw_corner_accents(&self, x: f32, y: f32, w: f32, h: f32) {
        let size = CORNER_ACCENT_SIZE;

        // Top-left corner
        draw_rectangle(x, y, size, 2.0, FRAME_ACCENT);
        draw_rectangle(x, y, 2.0, size, FRAME_ACCENT);

        // Top-right corner
        draw_rectangle(x + w - size, y, size, 2.0, FRAME_ACCENT);
        draw_rectangle(x + w - 2.0, y, 2.0, size, FRAME_ACCENT);

        // Bottom-left corner
        draw_rectangle(x, y + h - 2.0, size, 2.0, FRAME_ACCENT);
        draw_rectangle(x, y + h - size, 2.0, size, FRAME_ACCENT);

        // Bottom-right corner
        draw_rectangle(x + w - size, y + h - 2.0, size, 2.0, FRAME_ACCENT);
        draw_rectangle(x + w - 2.0, y + h - size, 2.0, size, FRAME_ACCENT);
    }

    /// Draw a slim medieval-style health bar above entities
    ///
    /// Creates a polished health bar with:
    /// - Thin 1px dark border with rounded corners
    /// - Jewel-toned health fill with gradient effect
    fn draw_entity_health_bar(&self, x: f32, y: f32, width: f32, height: f32, hp_ratio: f32, _scale: f32) {
        // Pixel-align coordinates for crisp rendering
        let x = x.floor();
        let y = y.floor();
        let width = width.floor();
        let height = height.floor();

        // Border color - deep purple-gray for contrast
        let border_color = Color::new(0.18, 0.16, 0.22, 1.0); // rgba(46, 41, 56, 255)

        // === 1px BORDER with rounded corners ===
        // Top edge (inset 1px from corners)
        draw_rectangle(x, y - 1.0, width, 1.0, border_color);
        // Bottom edge (inset 1px from corners)
        draw_rectangle(x, y + height, width, 1.0, border_color);
        // Left edge (inset 1px from corners)
        draw_rectangle(x - 1.0, y, 1.0, height, border_color);
        // Right edge (inset 1px from corners)
        draw_rectangle(x + width, y, 1.0, height, border_color);

        // === INNER BACKGROUND (Dark recessed look) ===
        draw_rectangle(x, y, width, height, HEALTHBAR_BG_OUTER);

        // === HEALTH FILL ===
        if hp_ratio > 0.0 {
            // Select colors based on health level (jewel tones)
            let (color_dark, color_mid, color_light) = if hp_ratio > 0.5 {
                (HEALTH_GREEN_DARK, HEALTH_GREEN_MID, HEALTH_GREEN_LIGHT)
            } else if hp_ratio > 0.25 {
                (HEALTH_YELLOW_DARK, HEALTH_YELLOW_MID, HEALTH_YELLOW_LIGHT)
            } else {
                (HEALTH_RED_DARK, HEALTH_RED_MID, HEALTH_RED_LIGHT)
            };

            let fill_width = (width * hp_ratio).max(1.0).floor();

            // Base fill (darker tone)
            draw_rectangle(x, y, fill_width, height, color_dark);

            // Mid gradient (main color)
            if height > 2.0 {
                draw_rectangle(x, y + 1.0, fill_width, height - 2.0, color_mid);
            }

            // Top highlight (bright shine effect)
            if height > 3.0 {
                let highlight_height = (height * 0.35).max(1.0).floor();
                draw_rectangle(x, y + 1.0, fill_width, highlight_height, color_light);
            }

            // Specular shine (small white gleam)
            if fill_width > 4.0 && height > 2.0 {
                let shine_width = (fill_width * 0.3).min(6.0).max(2.0).floor();
                let shine_color = Color::new(1.0, 1.0, 1.0, 0.4);
                draw_rectangle(x + 1.0, y + 1.0, shine_width, 1.0, shine_color);
            }
        }
    }

    /// Draw an inventory slot with bevel effect
    pub(crate) fn draw_inventory_slot(&self, x: f32, y: f32, size: f32, has_item: bool, state: SlotState) {
        // Outer slot border (bronze)
        draw_rectangle(x, y, size, size, SLOT_BORDER);

        // Inner recessed area (1px inset)
        let inner_x = x + 1.0;
        let inner_y = y + 1.0;
        let inner_size = size - 2.0;

        // Background based on state
        let bg = match state {
            SlotState::Normal => if has_item { SLOT_BG_FILLED } else { SLOT_BG_EMPTY },
            SlotState::Hovered => SLOT_HOVER_BG,
            SlotState::Dragging => SLOT_DRAG_SOURCE,
        };
        draw_rectangle(inner_x, inner_y, inner_size, inner_size, bg);

        // Inner shadow (top and left - simulates recessed slot)
        draw_line(inner_x, inner_y, inner_x + inner_size, inner_y, 2.0, SLOT_INNER_SHADOW);
        draw_line(inner_x, inner_y, inner_x, inner_y + inner_size, 2.0, SLOT_INNER_SHADOW);

        // Inner highlight (bottom and right - subtle)
        draw_line(inner_x + 1.0, inner_y + inner_size - 1.0, inner_x + inner_size, inner_y + inner_size - 1.0, 1.0, SLOT_HIGHLIGHT);
        draw_line(inner_x + inner_size - 1.0, inner_y + 1.0, inner_x + inner_size - 1.0, inner_y + inner_size, 1.0, SLOT_HIGHLIGHT);

        // State-specific border overlay
        match state {
            SlotState::Hovered => {
                draw_rectangle_lines(x, y, size, size, 2.0, SLOT_HOVER_BORDER);
            },
            SlotState::Dragging => {
                draw_rectangle_lines(x, y, size, size, 2.0, SLOT_SELECTED_BORDER);
            },
            _ => {}
        }
    }



    /// Draw an item icon using sprite or fallback color
    /// Uses the full texture, centered in the slot

    /// Render a dragged item following the cursor

    /// Word-wrap text to fit within a given width (approximate, assumes ~8px per char at size 16)
    /// Prefers breaking on word boundaries, but will break long words if necessary
    pub(crate) fn wrap_text(&self, text: &str, max_width: f32, font_size: f32) -> Vec<String> {
        let char_width = font_size * 0.5; // Approximate character width
        let max_chars = (max_width / char_width) as usize;

        if max_chars == 0 {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();

        // Helper to break a long word into chunks that fit
        let break_long_word = |word: &str, max_len: usize| -> Vec<String> {
            let chars: Vec<char> = word.chars().collect();
            chars.chunks(max_len)
                .map(|chunk| chunk.iter().collect())
                .collect()
        };

        for word in text.split_whitespace() {
            // If word itself is too long, break it up
            if word.chars().count() > max_chars {
                // First, push current line if not empty
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }
                // Break the long word into chunks
                let chunks = break_long_word(word, max_chars);
                for (i, chunk) in chunks.iter().enumerate() {
                    if i < chunks.len() - 1 {
                        lines.push(chunk.clone());
                    } else {
                        // Last chunk becomes the new current line
                        current_line = chunk.clone();
                    }
                }
            } else if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.chars().count() + 1 + word.chars().count() <= max_chars {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }



}
