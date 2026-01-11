use macroquad::prelude::*;
use std::collections::HashMap;
use crate::game::{GameState, Player, Camera, ConnectionStatus, LayerType, GroundItem, ChunkLayerType, CHUNK_SIZE, ActiveDialogue, ActiveQuest, RecipeDefinition, ContextMenu, DragState, DragSource};
use crate::game::npc::{Npc, NpcState};
use crate::game::tilemap::get_tile_color;
use crate::ui::{UiElementId, UiLayout};
use super::isometric::{world_to_screen, TILE_WIDTH, TILE_HEIGHT, calculate_depth};
use super::animation::{SPRITE_WIDTH, SPRITE_HEIGHT};
use super::font::BitmapFont;

/// Tileset configuration
const TILESET_TILE_WIDTH: f32 = 64.0;
const TILESET_TILE_HEIGHT: f32 = 32.0;
const TILESET_COLUMNS: u32 = 32;

/// Available player appearance options
pub const GENDERS: &[&str] = &["male", "female"];
pub const SKINS: &[&str] = &["tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"];

pub struct Renderer {
    player_color: Color,
    local_player_color: Color,
    /// Loaded tileset texture
    tileset: Option<Texture2D>,
    /// Player sprite sheets by appearance key (e.g., "male_tan")
    player_sprites: HashMap<String, Texture2D>,
    /// Equipment sprite sheets by item ID (e.g., "peasant_suit")
    equipment_sprites: HashMap<String, Texture2D>,
    /// Item inventory sprites by item ID (sprite sheets with icon on left half)
    item_sprites: HashMap<String, Texture2D>,
    /// Multi-size pixel font for sharp text rendering at various sizes
    font: BitmapFont,
    /// Quest complete banner texture
    quest_complete_texture: Option<Texture2D>,
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

        // Load equipment sprites from assets/sprites/equipment/ (scan directory)
        let mut equipment_sprites = HashMap::new();
        if let Ok(entries) = std::fs::read_dir("assets/sprites/equipment") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "png") {
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
        log::info!("Loaded {} equipment sprite variants", equipment_sprites.len());

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

        Self {
            player_color: Color::from_rgba(100, 150, 255, 255),
            local_player_color: Color::from_rgba(100, 255, 150, 255),
            tileset,
            player_sprites,
            equipment_sprites,
            item_sprites,
            font,
            quest_complete_texture,
        }
    }

    /// Get the sprite texture for a given player appearance
    fn get_player_sprite(&self, gender: &str, skin: &str) -> Option<&Texture2D> {
        let key = format!("{}_{}", gender, skin);
        self.player_sprites.get(&key)
            // Fallback to male_tan if sprite not found
            .or_else(|| self.player_sprites.get("male_tan"))
    }

    /// Draw text with pixel font for sharp rendering
    /// Uses multi-size bitmap font for crisp text at any size
    pub fn draw_text_sharp(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        self.font.draw_text(text, x, y, font_size, color);
    }

    /// Measure text with pixel font
    fn measure_text_sharp(&self, text: &str, font_size: f32) -> TextDimensions {
        self.font.measure_text(text, font_size)
    }

    /// Draw text with word wrapping to fit within max_width
    /// Returns the total height used
    fn draw_text_wrapped(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color, max_width: f32, line_height: f32) -> f32 {
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

    pub fn render(&self, state: &GameState) -> UiLayout {
        // 1. Render ground layer tiles
        self.render_tilemap_layer(state, LayerType::Ground);

        // 1.5. Render hovered tile border if hovering over a tile
        if let Some((tile_x, tile_y)) = state.hovered_tile {
            self.render_tile_hover(tile_x, tile_y, &state.camera);
        }

        // 2. Collect renderable items (players + NPCs + items + object tiles) for depth sorting
        #[derive(Clone)]
        enum Renderable<'a> {
            Player(&'a Player, bool),
            Npc(&'a Npc),
            Item(&'a GroundItem),
            Tile { x: u32, y: u32, tile_id: u32 },
        }

        let mut renderables: Vec<(f32, Renderable)> = Vec::new();

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

        // Add object layer tiles (trees, rocks, buildings)
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
                    self.render_player(player, is_local, is_selected, &state.camera);
                }
                Renderable::Npc(npc) => {
                    let is_selected = state.selected_entity_id.as_ref() == Some(&npc.id);
                    self.render_npc(npc, is_selected, &state.camera);
                }
                Renderable::Tile { x, y, tile_id } => {
                    let (screen_x, screen_y) = world_to_screen(x as f32, y as f32, &state.camera);
                    self.draw_isometric_object(screen_x, screen_y, tile_id, state.camera.zoom);
                }
            }
        }

        // 4. Render overhead layer (always on top)
        self.render_tilemap_layer(state, LayerType::Overhead);

        // 5. Render floating damage numbers
        self.render_damage_numbers(state);

        // 6. Render floating level up text
        self.render_level_up_events(state);

        // 7. Render chat bubbles above players
        self.render_chat_bubbles(state);

        // 8. Render UI (non-interactive elements)
        self.render_ui(state);

        // 9. Render interactive UI elements and return layout for hit detection
        self.render_interactive_ui(state)
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
            let padding = 2.0;
            let tail_height = 6.0;
            let corner_radius = 3.0;

            let lines = self.wrap_text(&bubble.text, max_bubble_width - padding * 2.0, font_size);
            let num_lines = lines.len().max(1);

            // Calculate bubble dimensions
            let mut max_line_width = 0.0f32;
            for line in &lines {
                let width = self.measure_text_sharp(line, font_size).width;
                max_line_width = max_line_width.max(width);
            }

            let bubble_width = (max_line_width + padding * 2.0).max(30.0);
            let bubble_height = num_lines as f32 * line_height + padding * 2.0;

            // Position bubble above player (above name tag)
            let bubble_x = screen_x - bubble_width / 2.0;
            let bubble_y = screen_y - 75.0 - bubble_height - tail_height;

            // Colors with alpha - off-white paper/comic book style
            let bg_color = Color::from_rgba(255, 250, 240, alpha); // Warm off-white/cream
            let border_color = Color::from_rgba(60, 50, 40, alpha); // Dark brown border
            let text_color = Color::from_rgba(30, 25, 20, alpha); // Dark brown text

            // Draw rounded rectangle bubble body (pixel-aligned for clean edges)
            // Using chamfered corners - no circles, just two overlapping rectangles
            let r = corner_radius;
            let bx = bubble_x.floor();
            let by = bubble_y.floor();
            let bw = bubble_width.floor();
            let bh = bubble_height.floor();

            // Horizontal strip (full width, inset top/bottom by radius)
            draw_rectangle(bx, by + r, bw, bh - r * 2.0, bg_color);
            // Vertical strip (full height, inset left/right by radius)
            draw_rectangle(bx + r, by, bw - r * 2.0, bh, bg_color);
            // Corner triangles to fill the chamfered corners
            // Top-left
            draw_triangle(Vec2::new(bx, by + r), Vec2::new(bx + r, by), Vec2::new(bx + r, by + r), bg_color);
            // Top-right
            draw_triangle(Vec2::new(bx + bw - r, by), Vec2::new(bx + bw, by + r), Vec2::new(bx + bw - r, by + r), bg_color);
            // Bottom-left
            draw_triangle(Vec2::new(bx, by + bh - r), Vec2::new(bx + r, by + bh - r), Vec2::new(bx + r, by + bh), bg_color);
            // Bottom-right
            draw_triangle(Vec2::new(bx + bw - r, by + bh - r), Vec2::new(bx + bw, by + bh - r), Vec2::new(bx + bw - r, by + bh), bg_color);

            // Draw tail (triangle pointing down)
            let tail_x = screen_x.floor();
            let tail_top_y = by + bh;
            let tail_bottom_y = tail_top_y + tail_height;
            let tail_half_width = 5.0;

            draw_triangle(
                Vec2::new(tail_x - tail_half_width, tail_top_y),
                Vec2::new(tail_x + tail_half_width, tail_top_y),
                Vec2::new(tail_x, tail_bottom_y),
                bg_color,
            );

            // Draw border - rounded corners with lines
            // Top edge
            draw_line(bx + r, by, bx + bw - r, by, 1.0, border_color);
            // Bottom edge (with gap for tail)
            draw_line(bx + r, by + bh, tail_x - tail_half_width, by + bh, 1.0, border_color);
            draw_line(tail_x + tail_half_width, by + bh, bx + bw - r, by + bh, 1.0, border_color);
            // Left edge
            draw_line(bx, by + r, bx, by + bh - r, 1.0, border_color);
            // Right edge
            draw_line(bx + bw, by + r, bx + bw, by + bh - r, 1.0, border_color);
            // Corner arcs (diagonal lines for pixel-art look)
            // Top-left
            draw_line(bx, by + r, bx + r, by, 1.0, border_color);
            // Top-right
            draw_line(bx + bw - r, by, bx + bw, by + r, 1.0, border_color);
            // Bottom-left
            draw_line(bx, by + bh - r, bx + r, by + bh, 1.0, border_color);
            // Bottom-right
            draw_line(bx + bw - r, by + bh, bx + bw, by + bh - r, 1.0, border_color);
            // Tail edges
            draw_line(tail_x - tail_half_width, tail_top_y, tail_x, tail_bottom_y, 1.0, border_color);
            draw_line(tail_x + tail_half_width, tail_top_y, tail_x, tail_bottom_y, 1.0, border_color);

            // Draw text lines (centered)
            let bubble_center_x = bx + bw / 2.0;
            let mut text_y = by + padding + font_size * 0.85;

            for line in &lines {
                let line_width = self.measure_text_sharp(line, font_size).width;
                let text_x = bubble_center_x - line_width / 2.0;
                self.draw_text_sharp(line, text_x, text_y, font_size, text_color);
                text_y += line_height;
            }
        }
    }

    fn render_damage_numbers(&self, state: &GameState) {
        let current_time = macroquad::time::get_time();

        for event in &state.damage_events {
            let age = (current_time - event.time) as f32;
            if age > 1.5 {
                continue;
            }

            // Calculate position with upward float
            let float_offset = age * 30.0; // Float up over time
            let (screen_x, screen_y) = world_to_screen(event.x, event.y, &state.camera);
            let final_y = screen_y - 20.0 - float_offset;

            // Fade out over time
            let alpha = ((1.5 - age) / 1.5 * 255.0) as u8;

            // Draw damage number with outline for visibility
            let text = format!("-{}", event.damage);
            let font_size = 20.0;
            let text_width = measure_text(&text, None, font_size as u16, 1.0).width;

            // Outline
            let outline_color = Color::from_rgba(0, 0, 0, alpha);
            for ox in [-1.0, 1.0] {
                for oy in [-1.0, 1.0] {
                    draw_text(
                        &text,
                        screen_x - text_width / 2.0 + ox,
                        final_y + oy,
                        font_size,
                        outline_color,
                    );
                }
            }

            // Main text (red for damage)
            draw_text(
                &text,
                screen_x - text_width / 2.0,
                final_y,
                font_size,
                Color::from_rgba(255, 50, 50, alpha),
            );
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
            // Render from chunk manager
            for (coord, chunk) in chunks.iter() {
                let chunk_offset_x = coord.x * CHUNK_SIZE as i32;
                let chunk_offset_y = coord.y * CHUNK_SIZE as i32;

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
    fn render_tile_hover(&self, tile_x: i32, tile_y: i32, camera: &Camera) {
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

    fn render_player(&self, player: &Player, is_local: bool, is_selected: bool, camera: &Camera) {
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

            // Draw equipment overlay (body armor)
            if let Some(ref body_item_id) = player.equipped_body {
                if let Some(equip_sprite) = self.equipment_sprites.get(body_item_id) {
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

            // Draw equipment overlay (boots)
            if let Some(ref feet_item_id) = player.equipped_feet {
                if let Some(equip_sprite) = self.equipment_sprites.get(feet_item_id) {
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

        // Player name (positioned just above head)
        let has_sprite = self.get_player_sprite(&player.gender, &player.skin).is_some();
        let name_y_offset = if has_sprite { scaled_sprite_height - 8.0 * zoom } else { 24.0 * zoom };

        // Build display name with optional (GM) suffix
        let name_width = self.measure_text_sharp(&player.name, 16.0).width;
        let gm_width = if player.is_admin { self.measure_text_sharp(" (GM)", 16.0).width } else { 0.0 };
        let total_width = name_width + gm_width;
        let name_x = screen_x - total_width / 2.0;
        let name_y = screen_y - name_y_offset + 2.0;

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

        // Health bar (if not full HP)
        if player.hp < player.max_hp {
            let bar_width = 30.0;
            let bar_height = 4.0;
            let bar_x = screen_x - bar_width / 2.0;
            let bar_y = screen_y - name_y_offset - 13.0;

            // Background
            draw_rectangle(bar_x, bar_y, bar_width, bar_height, DARKGRAY);

            // Health
            let hp_ratio = player.hp as f32 / player.max_hp as f32;
            let hp_color = if hp_ratio > 0.5 {
                GREEN
            } else if hp_ratio > 0.25 {
                YELLOW
            } else {
                RED
            };
            draw_rectangle(bar_x, bar_y, bar_width * hp_ratio, bar_height, hp_color);
        }
    }

    fn render_npc(&self, npc: &Npc, is_selected: bool, camera: &Camera) {
        let (screen_x, screen_y) = world_to_screen(npc.x, npc.y, camera);
        let zoom = camera.zoom;

        // Don't render dead NPCs (or render them faded)
        if npc.state == NpcState::Dead {
            // Draw faded corpse
            let fade_color = Color::from_rgba(50, 80, 50, 100);
            draw_circle(screen_x, screen_y - 8.0 * zoom, 16.0 * zoom, fade_color);
            return;
        }

        // Selection highlight (draw first, behind NPC)
        if is_selected {
            self.render_tile_selection(npc.x, npc.y, camera);
        }

        // NPC body color based on hostility
        let (base_color, highlight_color, name_color) = if npc.is_hostile() {
            // Hostile = green slime blob, red name
            (
                Color::from_rgba(80, 180, 80, 255),
                Color::from_rgba(120, 220, 120, 255),
                Color::from_rgba(255, 150, 150, 255),
            )
        } else {
            // Friendly = blue/purple humanoid indicator, cyan name
            (
                Color::from_rgba(100, 120, 200, 255),
                Color::from_rgba(140, 160, 240, 255),
                Color::from_rgba(150, 220, 255, 255),
            )
        };

        // Wobble animation based on movement
        let wobble = (macroquad::time::get_time() * 4.0 + npc.animation_frame as f64).sin() as f32;
        let radius = (10.0 + wobble * 1.5) * zoom;
        let height_offset = (8.0 + wobble * 2.0) * zoom;

        // Draw shadow
        draw_ellipse(screen_x, screen_y, 16.0 * zoom, 6.0 * zoom, 0.0, Color::from_rgba(0, 0, 0, 60));

        // Draw NPC body (oval blob) - TODO: use sprites based on entity_type
        draw_ellipse(screen_x, screen_y - height_offset, radius, radius * 0.7, 0.0, base_color);

        // Highlight
        draw_ellipse(screen_x - 3.0 * zoom, screen_y - height_offset - 2.0 * zoom, radius * 0.3, radius * 0.2, 0.0, highlight_color);

        // Interaction indicator for friendly NPCs (yellow exclamation mark above head)
        if !npc.is_hostile() {
            let pulse = (macroquad::time::get_time() * 2.0).sin() as f32 * 0.2 + 0.8;
            let indicator_y = screen_y - height_offset - radius - 25.0 * zoom;
            self.draw_text_sharp("!", screen_x - 3.0 * zoom, indicator_y, 16.0, Color::from_rgba(255, 220, 50, (pulse * 255.0) as u8));
        }

        // NPC name with level
        let name = npc.name();
        let name_width = self.measure_text_sharp(&name, 16.0).width;
        self.draw_text_sharp(
            &name,
            screen_x - name_width / 2.0,
            screen_y - height_offset - radius - 5.0 * zoom,
            16.0,
            name_color,
        );

        // Health bar (only show for hostile NPCs or when damaged)
        if npc.is_hostile() || npc.hp < npc.max_hp {
            let bar_width = 28.0 * zoom;
            let bar_height = 3.0 * zoom;
            let bar_x = screen_x - bar_width / 2.0;
            let bar_y = screen_y - height_offset - radius - 18.0 * zoom;

            // Background
            draw_rectangle(bar_x, bar_y, bar_width, bar_height, DARKGRAY);

            // Health
            let hp_ratio = npc.hp as f32 / npc.max_hp as f32;
            let hp_color = if hp_ratio > 0.5 {
                GREEN
            } else if hp_ratio > 0.25 {
                YELLOW
            } else {
                RED
            };
            draw_rectangle(bar_x, bar_y, bar_width * hp_ratio, bar_height, hp_color);
        }
    }

    fn render_ground_item(&self, item: &GroundItem, camera: &Camera, state: &GameState) {
        let (screen_x, screen_y) = world_to_screen(item.x, item.y, camera);
        let zoom = camera.zoom;

        // Bobbing animation
        let time = macroquad::time::get_time();
        let bob = ((time - item.animation_time) * 3.0).sin() as f32 * 2.0 * zoom;

        // Draw shadow
        draw_ellipse(screen_x, screen_y, 8.0 * zoom, 4.0 * zoom, 0.0, Color::from_rgba(0, 0, 0, 40));

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
                item_y - icon_height,
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

        // Draw quantity if > 1
        if item.quantity > 1 {
            let qty_text = format!("x{}", item.quantity);
            let text_width = self.measure_text_sharp(&qty_text, 16.0).width;
            self.draw_text_sharp(&qty_text, screen_x - text_width / 2.0, item_y + 14.0 * zoom, 16.0, WHITE);
        }
    }

    fn render_ui(&self, state: &GameState) {
        // Server announcements (top of screen)
        let current_time = macroquad::time::get_time();
        for (i, announcement) in state.ui_state.announcements.iter().enumerate() {
            let age = current_time - announcement.time;
            // Fade out after 6 seconds (announcements last 8 seconds total)
            let alpha = if age > 6.0 { ((8.0 - age) / 2.0 * 255.0) as u8 } else { 255 };

            let font_size = 24.0;
            let text = format!("[ANNOUNCEMENT] {}", announcement.text);
            let text_dims = self.measure_text_sharp(&text, font_size);
            let text_x = (screen_width() - text_dims.width) / 2.0;
            let text_y = 50.0 + (i as f32 * 35.0);

            // Dark background for visibility
            let padding = 10.0;
            draw_rectangle(
                text_x - padding,
                text_y - font_size - padding / 2.0,
                text_dims.width + padding * 2.0,
                font_size + padding,
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

        // Connection status (bottom-right, mirroring controls guide)
        let status_text = match state.connection_status {
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Disconnected => "Disconnected",
        };
        let status_color = match state.connection_status {
            ConnectionStatus::Connected => GREEN,
            ConnectionStatus::Connecting => YELLOW,
            ConnectionStatus::Disconnected => RED,
        };
        let status_width = self.measure_text_sharp(status_text, 16.0).width;
        self.draw_text_sharp(status_text, screen_width() - status_width - 16.0, screen_height() - 10.0, 16.0, status_color);

        // Chat messages (bottom-left)
        let chat_x = 10.0;
        let chat_y = screen_height() - 30.0;
        let line_height = 18.0;

        for (i, msg) in state.ui_state.chat_messages.iter().rev().take(5).enumerate() {
            let y = chat_y - (i as f32 * line_height);
            let text = format!("{}: {}", msg.sender_name, msg.text);
            self.draw_text_sharp(&text, chat_x, y, 16.0, WHITE);
        }

        // Local player stats (top-right)
        if let Some(player) = state.get_local_player() {
            let stats_x = screen_width() - 200.0;
            let stats_y = 20.0;
            let bar_width = 120.0;
            let bar_height = 12.0;

            self.draw_text_sharp(&format!("Level: {}", player.level), stats_x, stats_y, 16.0, WHITE);

            // HP Bar
            self.draw_text_sharp("HP:", stats_x, stats_y + 20.0, 16.0, WHITE);
            let hp_bar_x = stats_x + 30.0;
            let hp_ratio = player.hp as f32 / player.max_hp.max(1) as f32;
            draw_rectangle(hp_bar_x, stats_y + 10.0, bar_width, bar_height, DARKGRAY);
            draw_rectangle(hp_bar_x, stats_y + 10.0, bar_width * hp_ratio, bar_height, GREEN);
            self.draw_text_sharp(
                &format!("{}/{}", player.hp, player.max_hp),
                hp_bar_x + bar_width + 5.0,
                stats_y + 20.0,
                16.0,
                WHITE,
            );

            // EXP Bar
            self.draw_text_sharp("EXP:", stats_x, stats_y + 40.0, 16.0, WHITE);
            let exp_bar_x = stats_x + 30.0;
            let exp_ratio = player.exp as f32 / player.exp_to_next_level.max(1) as f32;
            draw_rectangle(exp_bar_x, stats_y + 30.0, bar_width, bar_height, DARKGRAY);
            draw_rectangle(exp_bar_x, stats_y + 30.0, bar_width * exp_ratio, bar_height, Color::from_rgba(100, 100, 255, 255));
            self.draw_text_sharp(
                &format!("{}/{}", player.exp, player.exp_to_next_level),
                exp_bar_x + bar_width + 5.0,
                stats_y + 40.0,
                16.0,
                WHITE,
            );
        }

        // Note: Interactive UI (inventory, crafting, dialogue, quick slots) is rendered
        // by render_interactive_ui() which is called by the main render loop

        // Chat input box (when open)
        if state.ui_state.chat_open {
            let input_x = 10.0;
            let input_y = screen_height() - 50.0;
            let input_width = 400.0;
            let input_height = 24.0;

            // Background
            draw_rectangle(input_x, input_y, input_width, input_height, Color::from_rgba(0, 0, 0, 180));
            draw_rectangle_lines(input_x, input_y, input_width, input_height, 1.0, WHITE);

            // Text
            let display_text = format!("{}", state.ui_state.chat_input);
            self.draw_text_sharp(&display_text, input_x + 5.0, input_y + 17.0, 16.0, WHITE);

            // Blinking cursor
            let cursor_blink = (macroquad::time::get_time() * 2.0) as i32 % 2 == 0;
            if cursor_blink {
                let text_width = self.measure_text_sharp(&display_text, 16.0).width;
                draw_line(
                    input_x + 5.0 + text_width + 2.0,
                    input_y + 4.0,
                    input_x + 5.0 + text_width + 2.0,
                    input_y + input_height - 4.0,
                    1.0,
                    WHITE,
                );
            }

            // Hint
            self.draw_text_sharp("Press Enter to send, Escape to cancel", input_x, input_y + input_height + 12.0, 16.0, GRAY);
        } else {
            // Controls hint (only show when chat is closed)
            self.draw_text_sharp("WASD: Move | Space: Attack | I: Inventory | E: Interact | Q: Quests | F: Pickup | F3: Debug", 16.0, screen_height() - 10.0, 16.0, LIGHTGRAY);
        }
    }

    /// Register ground item clickable areas and render hover labels
    fn render_ground_item_overlays(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        for (item_id, item) in &state.ground_items {
            let (screen_x, screen_y) = world_to_screen(item.x, item.y, &state.camera);

            // Clickable area (centered on item position)
            let click_width = 44.0;
            let click_height = 32.0;
            let bounds = Rect::new(
                screen_x - click_width / 2.0,
                screen_y - click_height - 8.0,
                click_width,
                click_height,
            );
            layout.add(UiElementId::GroundItem(item_id.clone()), bounds);

            // Check if hovered
            let is_hovered = matches!(hovered, Some(UiElementId::GroundItem(id)) if id == item_id);

            if is_hovered {
                // Get item definition for display name
                let item_def = state.item_registry.get_or_placeholder(&item.item_id);

                // Build label text
                let label = if item.quantity > 1 {
                    format!("{} (x{})", item_def.display_name, item.quantity)
                } else {
                    item_def.display_name.clone()
                };

                // Draw label above the item
                let label_width = self.measure_text_sharp(&label, 16.0).width;
                let label_x = screen_x - label_width / 2.0;
                let label_y = screen_y - click_height - 20.0;

                // Background for readability
                let padding = 4.0;
                draw_rectangle(
                    label_x - padding,
                    label_y - 14.0,
                    label_width + padding * 2.0,
                    18.0,
                    Color::from_rgba(0, 0, 0, 180),
                );

                // Label text
                self.draw_text_sharp(&label, label_x, label_y, 16.0, WHITE);
            }
        }
    }

    /// Render all interactive UI elements and return the layout for hit detection
    fn render_interactive_ui(&self, state: &GameState) -> UiLayout {
        let mut layout = UiLayout::new();
        let hovered = &state.ui_state.hovered_element;

        // Ground item clickable areas and hover labels (world-space, registered first)
        self.render_ground_item_overlays(state, hovered, &mut layout);

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

        // Quick slots (always visible at bottom)
        self.render_quick_slots(state, hovered, &mut layout);

        // Quest objective tracker (top-left)
        self.render_quest_tracker(state);

        // Quest completion notifications
        self.render_quest_completed(state);

        // Dialogue box (when active)
        if let Some(dialogue) = &state.ui_state.active_dialogue {
            self.render_dialogue(dialogue, hovered, &mut layout);
        }

        // Render item tooltip last so it appears on top of everything
        self.render_item_tooltip(state);

        // Render context menu on top of everything
        if let Some(ref context_menu) = state.ui_state.context_menu {
            self.render_context_menu(context_menu, state, &mut layout);
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

    fn render_inventory(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let slot_size = 50.0; // Sized to fit 44x32 item icons
        let slots_per_row = 5;
        let inv_width = 380.0; // Adjusted for smaller slots + equipment panel
        let inv_height = 300.0;
        let inv_x = (screen_width() - inv_width) / 2.0;
        let inv_y = (screen_height() - inv_height) / 2.0;

        // Background
        draw_rectangle(inv_x, inv_y, inv_width, inv_height, Color::from_rgba(30, 30, 40, 220));
        draw_rectangle_lines(inv_x, inv_y, inv_width, inv_height, 2.0, WHITE);

        // Title
        self.draw_text_sharp("Inventory", inv_x + 10.0, inv_y + 25.0, 16.0, WHITE);
        self.draw_text_sharp(&format!("Gold: {}", state.inventory.gold), inv_x + inv_width - 100.0, inv_y + 25.0, 16.0, GOLD);

        // Inventory Slots (left side)
        let grid_x = inv_x + 20.0;
        let grid_y = inv_y + 40.0;

        for i in 0..20 {
            let row = i / slots_per_row;
            let col = i % slots_per_row;
            let x = grid_x + col as f32 * slot_size;
            let y = grid_y + row as f32 * slot_size;

            // Register slot bounds for hit detection
            let bounds = Rect::new(x, y, slot_size - 4.0, slot_size - 4.0);
            layout.add(UiElementId::InventorySlot(i), bounds);

            // Check if this slot is hovered
            let is_hovered = matches!(hovered, Some(UiElementId::InventorySlot(idx)) if *idx == i);

            // Check if this slot is being dragged
            let is_dragging = matches!(&state.ui_state.drag_state, Some(drag) if matches!(&drag.source, DragSource::Inventory(idx) if *idx == i));

            // Slot background with hover/drag effect
            let bg_color = if is_dragging {
                Color::from_rgba(30, 30, 40, 255) // Dimmed when dragging
            } else if is_hovered {
                Color::from_rgba(70, 70, 90, 255)
            } else {
                Color::from_rgba(50, 50, 60, 255)
            };
            draw_rectangle(x, y, slot_size - 4.0, slot_size - 4.0, bg_color);

            // Border with hover/drag effect
            let border_color = if is_dragging {
                Color::from_rgba(100, 100, 255, 255) // Blue border for source slot
            } else if is_hovered {
                WHITE
            } else {
                GRAY
            };
            draw_rectangle_lines(x, y, slot_size - 4.0, slot_size - 4.0, 1.0, border_color);

            // Draw item if present (dimmed if being dragged)
            if let Some(slot) = &state.inventory.slots[i] {
                if !is_dragging {
                    self.draw_item_icon(&slot.item_id, x, y, slot_size - 4.0, slot_size - 4.0, state);

                    // Quantity
                    if slot.quantity > 1 {
                        self.draw_text_sharp(&slot.quantity.to_string(), x + 2.0, y + slot_size - 8.0, 16.0, WHITE);
                    }
                }
            }

            // Show slot number for first 5 (quick slots)
            if i < 5 {
                self.draw_text_sharp(&(i + 1).to_string(), x + slot_size - 14.0, y + 12.0, 16.0, GRAY);
            }
        }

        // Equipment Section (right side)
        let equip_x = inv_x + 280.0; // Adjusted for inventory grid
        let equip_y = inv_y + 40.0;
        let equip_slot_size = 50.0; // Match inventory slot size

        // Equipment section label
        self.draw_text_sharp("Equipment", equip_x, equip_y, 16.0, WHITE);

        // Body equipment slot
        let body_slot_x = equip_x;
        let body_slot_y = equip_y + 20.0;

        // Register equipment slot bounds
        let body_bounds = Rect::new(body_slot_x, body_slot_y, equip_slot_size, equip_slot_size);
        layout.add(UiElementId::EquipmentSlot("body".to_string()), body_bounds);

        // Check if body slot is hovered
        let body_hovered = matches!(hovered, Some(UiElementId::EquipmentSlot(slot)) if slot == "body");

        // Check if body slot is being dragged
        let body_dragging = matches!(&state.ui_state.drag_state, Some(drag) if matches!(&drag.source, DragSource::Equipment(slot) if slot == "body"));

        // Body slot background
        let body_bg_color = if body_dragging {
            Color::from_rgba(30, 30, 40, 255) // Dimmed when dragging
        } else if body_hovered {
            Color::from_rgba(70, 70, 100, 255)
        } else {
            Color::from_rgba(40, 40, 60, 255)
        };
        draw_rectangle(body_slot_x, body_slot_y, equip_slot_size, equip_slot_size, body_bg_color);

        // Body slot border
        let body_border_color = if body_dragging {
            Color::from_rgba(100, 100, 255, 255) // Blue border when dragging
        } else if body_hovered {
            WHITE
        } else {
            Color::from_rgba(80, 80, 120, 255)
        };
        draw_rectangle_lines(body_slot_x, body_slot_y, equip_slot_size, equip_slot_size, 2.0, body_border_color);

        // Draw equipped body item or placeholder (hide if being dragged)
        if !body_dragging {
            if let Some(local_player) = state.get_local_player() {
                if let Some(ref body_item_id) = local_player.equipped_body {
                    // Draw equipped item sprite
                    self.draw_item_icon(body_item_id, body_slot_x, body_slot_y, equip_slot_size, equip_slot_size, state);
                } else {
                    // Empty slot indicator
                    self.draw_text_sharp("Body", body_slot_x + 8.0, body_slot_y + 32.0, 16.0, DARKGRAY);
                }
            }
        }

        // Slot label below
        self.draw_text_sharp("Body", body_slot_x + 10.0, body_slot_y + equip_slot_size + 12.0, 16.0, GRAY);

        // Feet equipment slot (below body slot)
        let feet_slot_x = equip_x;
        let feet_slot_y = body_slot_y + equip_slot_size + 30.0;

        // Register feet equipment slot bounds
        let feet_bounds = Rect::new(feet_slot_x, feet_slot_y, equip_slot_size, equip_slot_size);
        layout.add(UiElementId::EquipmentSlot("feet".to_string()), feet_bounds);

        // Check if feet slot is hovered
        let feet_hovered = matches!(hovered, Some(UiElementId::EquipmentSlot(slot)) if slot == "feet");

        // Check if feet slot is being dragged
        let feet_dragging = matches!(&state.ui_state.drag_state, Some(drag) if matches!(&drag.source, DragSource::Equipment(slot) if slot == "feet"));

        // Feet slot background
        let feet_bg_color = if feet_dragging {
            Color::from_rgba(30, 30, 40, 255) // Dimmed when dragging
        } else if feet_hovered {
            Color::from_rgba(70, 70, 100, 255)
        } else {
            Color::from_rgba(40, 40, 60, 255)
        };
        draw_rectangle(feet_slot_x, feet_slot_y, equip_slot_size, equip_slot_size, feet_bg_color);

        // Feet slot border
        let feet_border_color = if feet_dragging {
            Color::from_rgba(100, 100, 255, 255) // Blue border when dragging
        } else if feet_hovered {
            WHITE
        } else {
            Color::from_rgba(80, 80, 120, 255)
        };
        draw_rectangle_lines(feet_slot_x, feet_slot_y, equip_slot_size, equip_slot_size, 2.0, feet_border_color);

        // Draw equipped feet item or placeholder (hide if being dragged)
        if !feet_dragging {
            if let Some(local_player) = state.get_local_player() {
                if let Some(ref feet_item_id) = local_player.equipped_feet {
                    // Draw equipped item sprite
                    self.draw_item_icon(feet_item_id, feet_slot_x, feet_slot_y, equip_slot_size, equip_slot_size, state);
                } else {
                    // Empty slot indicator
                    self.draw_text_sharp("Feet", feet_slot_x + 8.0, feet_slot_y + 32.0, 16.0, DARKGRAY);
                }
            }
        }

        // Slot label below
        self.draw_text_sharp("Feet", feet_slot_x + 12.0, feet_slot_y + equip_slot_size + 12.0, 16.0, GRAY);

        // Close hint
        self.draw_text_sharp("Press I to close", inv_x + 10.0, inv_y + inv_height - 15.0, 16.0, GRAY);
        self.draw_text_sharp("Right-click to equip", inv_x + 10.0, inv_y + inv_height - 30.0, 16.0, DARKGRAY);
    }

    fn render_quest_log(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let panel_width = 350.0;
        let panel_height = 400.0;
        let panel_x = (screen_width() - panel_width) / 2.0;
        let panel_y = (screen_height() - panel_height) / 2.0;

        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 100));

        // Panel background
        draw_rectangle(panel_x, panel_y, panel_width, panel_height, Color::from_rgba(30, 30, 40, 240));
        draw_rectangle_lines(panel_x, panel_y, panel_width, panel_height, 2.0, Color::from_rgba(100, 100, 120, 255));

        // Title
        let title = "Quest Log";
        self.draw_text_sharp(title, panel_x + 15.0, panel_y + 28.0, 16.0, Color::from_rgba(255, 220, 100, 255));

        // Separator line
        draw_line(panel_x + 10.0, panel_y + 40.0, panel_x + panel_width - 10.0, panel_y + 40.0, 1.0, GRAY);

        let mut y = panel_y + 60.0;
        let line_height = 20.0;

        if state.ui_state.active_quests.is_empty() {
            self.draw_text_sharp("No active quests", panel_x + 20.0, y, 16.0, GRAY);
            self.draw_text_sharp("Talk to NPCs with ! above their heads", panel_x + 20.0, y + line_height, 16.0, DARKGRAY);
        } else {
            for (quest_idx, quest) in state.ui_state.active_quests.iter().enumerate() {
                let quest_start_y = y;

                // Register quest entry bounds for hover detection
                let entry_height = line_height + 5.0 + (quest.objectives.len() as f32 * line_height);
                let bounds = Rect::new(panel_x + 10.0, quest_start_y - 5.0, panel_width - 20.0, entry_height);
                layout.add(UiElementId::QuestLogEntry(quest_idx), bounds);

                // Check if this quest is hovered
                let is_hovered = matches!(hovered, Some(UiElementId::QuestLogEntry(idx)) if *idx == quest_idx);

                // Draw highlight background if hovered
                if is_hovered {
                    draw_rectangle(panel_x + 10.0, quest_start_y - 5.0, panel_width - 20.0, entry_height, Color::from_rgba(50, 50, 70, 100));
                }

                // Quest name with icon
                let name_color = if is_hovered { Color::from_rgba(255, 240, 150, 255) } else { WHITE };
                self.draw_text_sharp("*", panel_x + 15.0, y, 16.0, Color::from_rgba(255, 220, 100, 255));
                self.draw_text_sharp(&quest.name, panel_x + 30.0, y, 16.0, name_color);
                y += line_height + 5.0;

                // Objectives
                for obj in &quest.objectives {
                    let (check_char, status_color) = if obj.completed {
                        ("v", Color::from_rgba(100, 255, 100, 255))
                    } else {
                        ("o", Color::from_rgba(180, 180, 180, 255))
                    };

                    self.draw_text_sharp(check_char, panel_x + 25.0, y, 16.0, status_color);

                    let obj_text = format!("{} ({}/{})", obj.description, obj.current, obj.target);
                    self.draw_text_sharp(&obj_text, panel_x + 40.0, y, 16.0, status_color);
                    y += line_height;
                }

                y += 10.0; // Space between quests

                // Check if we're about to overflow the panel
                if y > panel_y + panel_height - 50.0 {
                    let remaining = state.ui_state.active_quests.len().saturating_sub(1);
                    if remaining > 0 {
                        self.draw_text_sharp(&format!("...and {} more quests", remaining), panel_x + 20.0, y, 16.0, GRAY);
                    }
                    break;
                }
            }
        }

        // Close hint at bottom
        self.draw_text_sharp("Press Q to close", panel_x + 15.0, panel_y + panel_height - 20.0, 16.0, GRAY);
    }

    fn render_quick_slots(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let slot_size = 50.0; // Match inventory slot size for 44x32 icons
        let padding = 4.0;
        let total_width = 5.0 * (slot_size + padding) - padding;
        let start_x = (screen_width() - total_width) / 2.0;
        let start_y = screen_height() - slot_size - 20.0;

        for i in 0..5 {
            let x = start_x + i as f32 * (slot_size + padding);
            let y = start_y;

            // Register slot bounds for hit detection
            let bounds = Rect::new(x, y, slot_size, slot_size);
            layout.add(UiElementId::QuickSlot(i), bounds);

            // Check if this slot is hovered
            let is_hovered = matches!(hovered, Some(UiElementId::QuickSlot(idx)) if *idx == i);

            // Check if this slot is being dragged (quick slots are first 5 inventory slots)
            let is_dragging = matches!(&state.ui_state.drag_state, Some(drag) if matches!(&drag.source, DragSource::Inventory(idx) if *idx == i));

            // Slot background with hover/drag effect
            let bg_color = if is_dragging {
                Color::from_rgba(20, 20, 30, 200) // Dimmed when dragging
            } else if is_hovered {
                Color::from_rgba(50, 50, 70, 255)
            } else {
                Color::from_rgba(30, 30, 40, 200)
            };
            draw_rectangle(x, y, slot_size, slot_size, bg_color);

            // Border with hover/drag effect
            let border_color = if is_dragging {
                Color::from_rgba(100, 100, 255, 255) // Blue border for source slot
            } else if is_hovered {
                WHITE
            } else {
                GRAY
            };
            draw_rectangle_lines(x, y, slot_size, slot_size, 1.0, border_color);

            // Draw item if present (hide if being dragged)
            if let Some(slot) = &state.inventory.slots[i] {
                if !is_dragging {
                    self.draw_item_icon(&slot.item_id, x, y, slot_size, slot_size, state);

                    // Quantity
                    if slot.quantity > 1 {
                        self.draw_text_sharp(&slot.quantity.to_string(), x + 2.0, y + slot_size - 4.0, 16.0, WHITE);
                    }
                }
            }

            // Slot number
            self.draw_text_sharp(&(i + 1).to_string(), x + slot_size - 10.0, y + 12.0, 16.0, WHITE);
        }
    }

    /// Draw an item icon using sprite or fallback color
    /// Uses the full texture, centered in the slot
    fn draw_item_icon(&self, item_id: &str, x: f32, y: f32, slot_width: f32, slot_height: f32, state: &GameState) {
        // Try to get the item sprite
        if let Some(texture) = self.item_sprites.get(item_id) {
            // Use full texture, centered in the slot
            let icon_width = texture.width();
            let icon_height = texture.height();

            // Center the icon in the slot
            let offset_x = (slot_width - icon_width) / 2.0;
            let offset_y = (slot_height - icon_height) / 2.0;

            draw_texture_ex(
                texture,
                x + offset_x,
                y + offset_y,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            // Fallback: colored rectangle based on category (centered)
            let item_def = state.item_registry.get_or_placeholder(item_id);
            let color = item_def.category_color();
            let icon_width = 32.0;
            let icon_height = 32.0;
            let offset_x = (slot_width - icon_width) / 2.0;
            let offset_y = (slot_height - icon_height) / 2.0;
            draw_rectangle(x + offset_x, y + offset_y, icon_width, icon_height, color);
        }
    }

    /// Render a dragged item following the cursor
    fn render_dragged_item(&self, drag: &DragState, state: &GameState) {
        let (mx, my) = mouse_position();
        let slot_size = 46.0;

        // Semi-transparent background for the dragged item
        draw_rectangle(
            mx - slot_size / 2.0,
            my - slot_size / 2.0,
            slot_size,
            slot_size,
            Color::from_rgba(50, 50, 60, 180),
        );
        draw_rectangle_lines(
            mx - slot_size / 2.0,
            my - slot_size / 2.0,
            slot_size,
            slot_size,
            2.0,
            Color::from_rgba(255, 255, 255, 200),
        );

        // Draw the item icon centered on cursor
        self.draw_item_icon(&drag.item_id, mx - slot_size / 2.0, my - slot_size / 2.0, slot_size, slot_size, state);

        // Draw quantity if > 1
        if drag.quantity > 1 {
            self.draw_text_sharp(&drag.quantity.to_string(), mx - slot_size / 2.0 + 2.0, my + slot_size / 2.0 - 4.0, 16.0, WHITE);
        }
    }

    /// Word-wrap text to fit within a given width (approximate, assumes ~8px per char at size 16)
    fn wrap_text(&self, text: &str, max_width: f32, font_size: f32) -> Vec<String> {
        let char_width = font_size * 0.5; // Approximate character width
        let max_chars = (max_width / char_width) as usize;

        if max_chars == 0 {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= max_chars {
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

    /// Render tooltip for hovered inventory/quick slot items
    fn render_item_tooltip(&self, state: &GameState) {
        // Check if we're hovering over an inventory or quick slot
        let slot_idx = match &state.ui_state.hovered_element {
            Some(UiElementId::InventorySlot(idx)) if state.ui_state.inventory_open => Some(*idx),
            Some(UiElementId::QuickSlot(idx)) => Some(*idx),
            _ => None,
        };

        let Some(idx) = slot_idx else { return };
        let Some(slot) = &state.inventory.slots.get(idx).and_then(|s| s.as_ref()) else { return };

        // Get item definition from registry
        let item_def = state.item_registry.get_or_placeholder(&slot.item_id);

        // Get player level for requirement checking
        let player_level = state.get_local_player().map(|p| p.level).unwrap_or(1);

        // Get mouse position for tooltip placement
        let (mouse_x, mouse_y) = mouse_position();

        // Calculate tooltip dimensions based on content
        let padding = 10.0;
        let line_height = 18.0;
        let tooltip_width = 220.0;
        let text_width = tooltip_width - padding * 2.0;

        // Word-wrap the description
        let desc_lines = if !item_def.description.is_empty() {
            self.wrap_text(&item_def.description, text_width, 16.0)
        } else {
            vec![]
        };

        // Count lines: name, category, description lines, equipment stats, quantity
        let mut line_count = 2; // name + category
        if !desc_lines.is_empty() {
            line_count += 1; // blank line before description
            line_count += desc_lines.len();
        }

        // Count equipment stat lines
        if let Some(ref equip) = item_def.equipment {
            line_count += 1; // blank line before stats
            line_count += 1; // slot type
            if equip.damage_bonus != 0 {
                line_count += 1;
            }
            if equip.defense_bonus != 0 {
                line_count += 1;
            }
            line_count += 1; // level requirement (always show for equipment)
        }

        if slot.quantity > 1 {
            line_count += 1; // quantity line
        }

        let tooltip_height = padding * 2.0 + line_count as f32 * line_height;

        // Position tooltip near cursor, but keep on screen
        let mut tooltip_x = mouse_x + 16.0;
        let mut tooltip_y = mouse_y + 16.0;

        // Clamp to screen bounds
        if tooltip_x + tooltip_width > screen_width() {
            tooltip_x = mouse_x - tooltip_width - 8.0;
        }
        if tooltip_y + tooltip_height > screen_height() {
            tooltip_y = mouse_y - tooltip_height - 8.0;
        }

        // Draw tooltip background
        draw_rectangle(
            tooltip_x,
            tooltip_y,
            tooltip_width,
            tooltip_height,
            Color::from_rgba(20, 20, 30, 240),
        );
        draw_rectangle_lines(
            tooltip_x,
            tooltip_y,
            tooltip_width,
            tooltip_height,
            1.0,
            Color::from_rgba(100, 100, 120, 255),
        );

        let mut y = tooltip_y + padding + 12.0;

        // Item name (white, bold feel)
        self.draw_text_sharp(&item_def.display_name, tooltip_x + padding, y, 16.0, WHITE);
        y += line_height;

        // Category (colored by type)
        let category_text = format!("[{}]", item_def.category);
        let category_color = item_def.category_color();
        self.draw_text_sharp(&category_text, tooltip_x + padding, y, 16.0, category_color);
        y += line_height;

        // Description (word-wrapped)
        if !desc_lines.is_empty() {
            y += line_height * 0.5; // Small gap before description
            for line in &desc_lines {
                self.draw_text_sharp(line, tooltip_x + padding, y, 16.0, LIGHTGRAY);
                y += line_height;
            }
        }

        // Equipment stats section
        if let Some(ref equip) = item_def.equipment {
            y += line_height * 0.5; // Small gap before stats

            // Slot type
            let slot_text = format!("Slot: {}", equip.slot_type);
            self.draw_text_sharp(&slot_text, tooltip_x + padding, y, 16.0, Color::from_rgba(150, 150, 180, 255));
            y += line_height;

            // Damage bonus (green for positive)
            if equip.damage_bonus != 0 {
                let damage_text = if equip.damage_bonus > 0 {
                    format!("+{} Damage", equip.damage_bonus)
                } else {
                    format!("{} Damage", equip.damage_bonus)
                };
                let damage_color = if equip.damage_bonus > 0 { GREEN } else { RED };
                self.draw_text_sharp(&damage_text, tooltip_x + padding, y, 16.0, damage_color);
                y += line_height;
            }

            // Defense bonus (green for positive)
            if equip.defense_bonus != 0 {
                let defense_text = if equip.defense_bonus > 0 {
                    format!("+{} Defense", equip.defense_bonus)
                } else {
                    format!("{} Defense", equip.defense_bonus)
                };
                let defense_color = if equip.defense_bonus > 0 { GREEN } else { RED };
                self.draw_text_sharp(&defense_text, tooltip_x + padding, y, 16.0, defense_color);
                y += line_height;
            }

            // Level requirement (green if met, red if not)
            let meets_requirement = player_level >= equip.level_required;
            let req_color = if meets_requirement {
                Color::from_rgba(100, 200, 100, 255) // Soft green
            } else {
                Color::from_rgba(255, 100, 100, 255) // Soft red
            };
            let req_text = format!("Requires Level {}", equip.level_required);
            self.draw_text_sharp(&req_text, tooltip_x + padding, y, 16.0, req_color);
            y += line_height;
        }

        // Quantity (if more than 1)
        if slot.quantity > 1 {
            let qty_text = format!("Quantity: {}", slot.quantity);
            self.draw_text_sharp(&qty_text, tooltip_x + padding, y, 16.0, WHITE);
        }
    }

    /// Render the right-click context menu for items
    fn render_context_menu(&self, menu: &ContextMenu, state: &GameState, layout: &mut UiLayout) {
        let padding = 8.0;
        let option_height = 28.0;
        let menu_width = 120.0;

        // Determine which options to show
        let mut options: Vec<(&str, UiElementId)> = Vec::new();

        if menu.is_equipment {
            // Equipment slot - only unequip option
            options.push(("Unequip", UiElementId::ContextMenuOption(0)));
        } else {
            // Inventory slot - check if item is equippable
            if let Some(slot) = state.inventory.slots.get(menu.slot_index).and_then(|s| s.as_ref()) {
                let item_def = state.item_registry.get_or_placeholder(&slot.item_id);
                if item_def.equipment.is_some() {
                    options.push(("Equip", UiElementId::ContextMenuOption(0)));
                }
            }
            options.push(("Drop", UiElementId::ContextMenuOption(options.len())));
        }

        let menu_height = padding * 2.0 + options.len() as f32 * option_height;

        // Position menu at cursor, but keep on screen
        let mut menu_x = menu.x;
        let mut menu_y = menu.y;

        if menu_x + menu_width > screen_width() {
            menu_x = screen_width() - menu_width - 5.0;
        }
        if menu_y + menu_height > screen_height() {
            menu_y = screen_height() - menu_height - 5.0;
        }

        // Background
        draw_rectangle(menu_x, menu_y, menu_width, menu_height, Color::from_rgba(30, 30, 40, 245));
        draw_rectangle_lines(menu_x, menu_y, menu_width, menu_height, 1.0, Color::from_rgba(100, 100, 120, 255));

        // Draw options
        let (mouse_x, mouse_y) = mouse_position();
        let mut y = menu_y + padding;

        for (i, (label, element_id)) in options.iter().enumerate() {
            let option_bounds = Rect::new(menu_x + 2.0, y, menu_width - 4.0, option_height - 2.0);
            layout.add(element_id.clone(), option_bounds);

            // Check hover
            let is_hovered = mouse_x >= option_bounds.x && mouse_x <= option_bounds.x + option_bounds.w
                && mouse_y >= option_bounds.y && mouse_y <= option_bounds.y + option_bounds.h;

            // Hover highlight
            if is_hovered {
                draw_rectangle(option_bounds.x, option_bounds.y, option_bounds.w, option_bounds.h, Color::from_rgba(60, 60, 80, 255));
            }

            // Label
            let text_color = if is_hovered { WHITE } else { LIGHTGRAY };
            self.draw_text_sharp(label, menu_x + padding, y + 18.0, 16.0, text_color);

            y += option_height;
        }
    }

    /// Render the escape menu (settings and disconnect)
    fn render_escape_menu(&self, state: &GameState, layout: &mut UiLayout) {
        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 150));

        let menu_width = 280.0;
        let menu_height = 220.0;
        let menu_x = (screen_width() - menu_width) / 2.0;
        let menu_y = (screen_height() - menu_height) / 2.0;

        // Background
        draw_rectangle(menu_x, menu_y, menu_width, menu_height, Color::from_rgba(30, 30, 40, 245));
        draw_rectangle_lines(menu_x, menu_y, menu_width, menu_height, 2.0, Color::from_rgba(100, 100, 120, 255));

        // Title
        let title = "Menu";
        let title_width = self.measure_text_sharp(title, 20.0).width;
        self.draw_text_sharp(title, menu_x + (menu_width - title_width) / 2.0, menu_y + 35.0, 20.0, WHITE);

        // Camera Zoom section
        self.draw_text_sharp("Camera Zoom", menu_x + 20.0, menu_y + 70.0, 16.0, LIGHTGRAY);

        let button_width = 100.0;
        let button_height = 36.0;
        let button_y = menu_y + 85.0;
        let button_spacing = 20.0;
        let buttons_total_width = button_width * 2.0 + button_spacing;
        let buttons_start_x = menu_x + (menu_width - buttons_total_width) / 2.0;

        // Get current mouse position for hover detection
        let (mouse_x, mouse_y) = mouse_position();

        // 1x Zoom button
        let zoom_1x_bounds = Rect::new(buttons_start_x, button_y, button_width, button_height);
        layout.add(UiElementId::EscapeMenuZoom1x, zoom_1x_bounds);
        let is_1x_hovered = mouse_x >= zoom_1x_bounds.x && mouse_x <= zoom_1x_bounds.x + zoom_1x_bounds.w
            && mouse_y >= zoom_1x_bounds.y && mouse_y <= zoom_1x_bounds.y + zoom_1x_bounds.h;
        let is_1x_selected = (state.camera.zoom - 1.0).abs() < 0.1;
        let bg_1x = if is_1x_selected {
            Color::from_rgba(60, 100, 60, 255)
        } else if is_1x_hovered {
            Color::from_rgba(70, 70, 90, 255)
        } else {
            Color::from_rgba(50, 50, 60, 255)
        };
        draw_rectangle(zoom_1x_bounds.x, zoom_1x_bounds.y, zoom_1x_bounds.w, zoom_1x_bounds.h, bg_1x);
        draw_rectangle_lines(zoom_1x_bounds.x, zoom_1x_bounds.y, zoom_1x_bounds.w, zoom_1x_bounds.h, 1.0, if is_1x_selected { GREEN } else { GRAY });
        let text_1x = "1x";
        let text_1x_width = self.measure_text_sharp(text_1x, 16.0).width;
        self.draw_text_sharp(text_1x, zoom_1x_bounds.x + (button_width - text_1x_width) / 2.0, zoom_1x_bounds.y + 24.0, 16.0, WHITE);

        // 2x Zoom button
        let zoom_2x_bounds = Rect::new(buttons_start_x + button_width + button_spacing, button_y, button_width, button_height);
        layout.add(UiElementId::EscapeMenuZoom2x, zoom_2x_bounds);
        let is_2x_hovered = mouse_x >= zoom_2x_bounds.x && mouse_x <= zoom_2x_bounds.x + zoom_2x_bounds.w
            && mouse_y >= zoom_2x_bounds.y && mouse_y <= zoom_2x_bounds.y + zoom_2x_bounds.h;
        let is_2x_selected = (state.camera.zoom - 2.0).abs() < 0.1;
        let bg_2x = if is_2x_selected {
            Color::from_rgba(60, 100, 60, 255)
        } else if is_2x_hovered {
            Color::from_rgba(70, 70, 90, 255)
        } else {
            Color::from_rgba(50, 50, 60, 255)
        };
        draw_rectangle(zoom_2x_bounds.x, zoom_2x_bounds.y, zoom_2x_bounds.w, zoom_2x_bounds.h, bg_2x);
        draw_rectangle_lines(zoom_2x_bounds.x, zoom_2x_bounds.y, zoom_2x_bounds.w, zoom_2x_bounds.h, 1.0, if is_2x_selected { GREEN } else { GRAY });
        let text_2x = "2x";
        let text_2x_width = self.measure_text_sharp(text_2x, 16.0).width;
        self.draw_text_sharp(text_2x, zoom_2x_bounds.x + (button_width - text_2x_width) / 2.0, zoom_2x_bounds.y + 24.0, 16.0, WHITE);

        // Disconnect button
        let disconnect_width = 180.0;
        let disconnect_height = 40.0;
        let disconnect_x = menu_x + (menu_width - disconnect_width) / 2.0;
        let disconnect_y = menu_y + menu_height - disconnect_height - 30.0;
        let disconnect_bounds = Rect::new(disconnect_x, disconnect_y, disconnect_width, disconnect_height);
        layout.add(UiElementId::EscapeMenuDisconnect, disconnect_bounds);
        let is_disconnect_hovered = mouse_x >= disconnect_bounds.x && mouse_x <= disconnect_bounds.x + disconnect_bounds.w
            && mouse_y >= disconnect_bounds.y && mouse_y <= disconnect_bounds.y + disconnect_bounds.h;
        let bg_disconnect = if is_disconnect_hovered {
            Color::from_rgba(120, 50, 50, 255)
        } else {
            Color::from_rgba(80, 40, 40, 255)
        };
        draw_rectangle(disconnect_bounds.x, disconnect_bounds.y, disconnect_bounds.w, disconnect_bounds.h, bg_disconnect);
        draw_rectangle_lines(disconnect_bounds.x, disconnect_bounds.y, disconnect_bounds.w, disconnect_bounds.h, 1.0, Color::from_rgba(180, 80, 80, 255));
        let disconnect_text = "Disconnect";
        let disconnect_text_width = self.measure_text_sharp(disconnect_text, 16.0).width;
        self.draw_text_sharp(disconnect_text, disconnect_bounds.x + (disconnect_width - disconnect_text_width) / 2.0, disconnect_bounds.y + 26.0, 16.0, WHITE);

        // Hint text
        let hint = "Press ESC to close";
        let hint_width = self.measure_text_sharp(hint, 14.0).width;
        self.draw_text_sharp(hint, menu_x + (menu_width - hint_width) / 2.0, menu_y + menu_height - 10.0, 14.0, GRAY);
    }

    /// Render the dialogue box for NPC conversations
    fn render_dialogue(&self, dialogue: &ActiveDialogue, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        // Semi-transparent overlay to focus attention
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 100));

        let box_width = 600.0;
        let box_height = 200.0 + (dialogue.choices.len() as f32 * 30.0);
        let box_x = (screen_width() - box_width) / 2.0;
        let box_y = screen_height() - box_height - 80.0;

        // Main dialogue box
        draw_rectangle(box_x, box_y, box_width, box_height, Color::from_rgba(20, 20, 30, 240));
        draw_rectangle_lines(box_x, box_y, box_width, box_height, 2.0, Color::from_rgba(100, 100, 120, 255));

        // Speaker name with highlight
        let speaker_box_width = self.measure_text_sharp(&dialogue.speaker, 16.0).width + 20.0;
        draw_rectangle(box_x + 15.0, box_y - 12.0, speaker_box_width, 24.0, Color::from_rgba(60, 60, 80, 255));
        draw_rectangle_lines(box_x + 15.0, box_y - 12.0, speaker_box_width, 24.0, 1.0, Color::from_rgba(100, 100, 120, 255));
        self.draw_text_sharp(&dialogue.speaker, box_x + 25.0, box_y + 5.0, 16.0, Color::from_rgba(255, 220, 100, 255));

        // Dialogue text with word wrap
        let text_x = box_x + 20.0;
        let text_y = box_y + 40.0;
        let max_line_width = box_width - 40.0;

        // Simple word wrap
        let words: Vec<&str> = dialogue.text.split_whitespace().collect();
        let mut current_line = String::new();
        let mut line_y = text_y;

        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            let line_width = self.measure_text_sharp(&test_line, 16.0).width;
            if line_width > max_line_width && !current_line.is_empty() {
                self.draw_text_sharp(&current_line, text_x, line_y, 16.0, WHITE);
                line_y += 22.0;
                current_line = word.to_string();
            } else {
                current_line = test_line;
            }
        }
        if !current_line.is_empty() {
            self.draw_text_sharp(&current_line, text_x, line_y, 16.0, WHITE);
        }

        // Choices
        if dialogue.choices.is_empty() {
            // No choices - show continue hint (clickable area)
            let hint = "Click or press [Enter] to continue...";
            let hint_width = self.measure_text_sharp(hint, 16.0).width;
            let hint_x = box_x + box_width - hint_width - 20.0;
            let hint_y = box_y + box_height - 20.0;

            // Register continue area for click detection
            let bounds = Rect::new(hint_x - 5.0, hint_y - 16.0, hint_width + 10.0, 22.0);
            layout.add(UiElementId::DialogueContinue, bounds);

            let is_hovered = matches!(hovered, Some(UiElementId::DialogueContinue));
            let hint_color = if is_hovered { WHITE } else { GRAY };
            self.draw_text_sharp(hint, hint_x, hint_y, 16.0, hint_color);
        } else {
            // Render choices
            let choice_start_y = box_y + box_height - 30.0 - (dialogue.choices.len() as f32 * 30.0);

            for (i, choice) in dialogue.choices.iter().enumerate() {
                let choice_y = choice_start_y + (i as f32 * 30.0);
                let choice_text = format!("[{}] {}", i + 1, choice.text);

                // Register choice bounds for click detection
                let bounds = Rect::new(text_x - 5.0, choice_y - 16.0, max_line_width, 26.0);
                layout.add(UiElementId::DialogueChoice(i), bounds);

                // Check if this choice is hovered
                let is_hovered = matches!(hovered, Some(UiElementId::DialogueChoice(idx)) if *idx == i);

                // Choice background with hover effect
                let bg_color = if is_hovered {
                    Color::from_rgba(70, 70, 100, 255)
                } else {
                    Color::from_rgba(50, 50, 70, 200)
                };
                draw_rectangle(text_x - 5.0, choice_y - 16.0, max_line_width, 26.0, bg_color);

                // Choice text with hover effect
                let text_color = if is_hovered {
                    WHITE
                } else {
                    Color::from_rgba(200, 200, 255, 255)
                };
                self.draw_text_sharp(&choice_text, text_x, choice_y, 16.0, text_color);
            }

            // Hint (updated to mention clicking)
            self.draw_text_sharp("Click or press [1-4] to select | [Esc] to close", box_x + 20.0, box_y + box_height - 15.0, 16.0, GRAY);
        }
    }

    /// Render the quest objective tracker (top-left corner, below debug info)
    fn render_quest_tracker(&self, state: &GameState) {
        if state.ui_state.active_quests.is_empty() {
            return;
        }

        let tracker_x = 10.0;
        // Start below debug info (which ends at ~Y=120 when enabled)
        let tracker_y = if state.debug_mode { 150.0 } else { 20.0 };
        let line_height = 18.0;

        let mut y = tracker_y;

        // Header
        self.draw_text_sharp("QUESTS", tracker_x, y, 16.0, Color::from_rgba(255, 220, 100, 255));
        y += line_height + 5.0;

        // Only show first 2 active quests to avoid cluttering the screen
        for quest in state.ui_state.active_quests.iter().take(2) {
            // Quest name
            self.draw_text_sharp(&quest.name, tracker_x, y, 16.0, WHITE);
            y += line_height;

            // Objectives
            for obj in &quest.objectives {
                let status_color = if obj.completed {
                    Color::from_rgba(100, 255, 100, 255) // Green for complete
                } else {
                    Color::from_rgba(200, 200, 200, 255) // Gray for incomplete
                };

                let check = if obj.completed { "[x]" } else { "[ ]" };
                let obj_text = format!("{} {} ({}/{})", check, obj.description, obj.current, obj.target);
                self.draw_text_sharp(&obj_text, tracker_x + 10.0, y, 16.0, status_color);
                y += line_height - 2.0;
            }

            y += 8.0; // Space between quests
        }

        // Show more quests hint if there are more
        if state.ui_state.active_quests.len() > 2 {
            let more = format!("...and {} more (Q to view)", state.ui_state.active_quests.len() - 2);
            self.draw_text_sharp(&more, tracker_x, y, 16.0, GRAY);
        }
    }

    /// Render quest completion notifications (center screen, floating)
    fn render_quest_completed(&self, state: &GameState) {
        let current_time = macroquad::time::get_time();

        for event in &state.ui_state.quest_completed_events {
            let age = (current_time - event.time) as f32;
            if age > 4.0 {
                continue;
            }

            // Fade out over the last second
            let alpha = if age > 3.0 {
                ((4.0 - age) * 255.0) as u8
            } else {
                255
            };

            // Pop-in scale animation (starts big, settles to normal)
            let scale = if age < 0.3 {
                // Ease out: start at 1.3x, settle to 1.0x
                let t = age / 0.3;
                1.3 - 0.3 * t * t
            } else {
                1.0
            };

            // Float up slightly
            let float_offset = (age * 10.0).min(30.0);

            // Position at top-center
            let base_y = 120.0 - float_offset;

            // Use quest complete sprite if available
            if let Some(texture) = &self.quest_complete_texture {
                let tex_width = texture.width() * scale;
                let tex_height = texture.height() * scale;
                let x = (screen_width() - tex_width) / 2.0;
                let y = base_y - tex_height / 2.0;

                draw_texture_ex(
                    texture,
                    x,
                    y,
                    Color::from_rgba(255, 255, 255, alpha),
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(tex_width, tex_height)),
                        ..Default::default()
                    },
                );

                // Quest name (below the sprite)
                let name_width = self.measure_text_sharp(&event.quest_name, 16.0).width;
                self.draw_text_sharp(
                    &event.quest_name,
                    (screen_width() - name_width) / 2.0,
                    y + tex_height + 8.0,
                    16.0,
                    Color::from_rgba(255, 255, 255, alpha),
                );

                // Rewards
                let rewards = format!("+{} EXP  +{} Gold", event.exp_reward, event.gold_reward);
                let rewards_width = self.measure_text_sharp(&rewards, 16.0).width;
                self.draw_text_sharp(
                    &rewards,
                    (screen_width() - rewards_width) / 2.0,
                    y + tex_height + 28.0,
                    16.0,
                    Color::from_rgba(100, 255, 100, alpha),
                );
            } else {
                // Fallback to text rendering if texture not loaded
                let title = "QUEST COMPLETE!";
                let title_width = self.measure_text_sharp(title, 32.0).width;
                let x = (screen_width() - title_width) / 2.0;

                // Outline
                let outline_color = Color::from_rgba(0, 0, 0, alpha);
                for ox in [-2.0, 2.0] {
                    for oy in [-2.0, 2.0] {
                        self.draw_text_sharp(title, x + ox, base_y + oy, 32.0, outline_color);
                    }
                }

                // Main text (gold)
                self.draw_text_sharp(title, x, base_y, 32.0, Color::from_rgba(255, 215, 0, alpha));

                // Quest name
                let name_width = self.measure_text_sharp(&event.quest_name, 16.0).width;
                self.draw_text_sharp(
                    &event.quest_name,
                    (screen_width() - name_width) / 2.0,
                    base_y + 25.0,
                    16.0,
                    Color::from_rgba(255, 255, 255, alpha),
                );

                // Rewards
                let rewards = format!("+{} EXP  +{} Gold", event.exp_reward, event.gold_reward);
                let rewards_width = self.measure_text_sharp(&rewards, 16.0).width;
                self.draw_text_sharp(
                    &rewards,
                    (screen_width() - rewards_width) / 2.0,
                    base_y + 45.0,
                    16.0,
                    Color::from_rgba(100, 255, 100, alpha),
                );
            }
        }
    }

    /// Render the crafting panel (shop UI)
    fn render_crafting(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let panel_width = 650.0;
        let panel_height = 450.0;
        let panel_x = (screen_width() - panel_width) / 2.0;
        let panel_y = (screen_height() - panel_height) / 2.0;

        // Semi-transparent overlay
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 150));

        // Panel background
        draw_rectangle(panel_x, panel_y, panel_width, panel_height, Color::from_rgba(30, 30, 45, 245));
        draw_rectangle_lines(panel_x, panel_y, panel_width, panel_height, 2.0, Color::from_rgba(100, 100, 140, 255));

        // Title
        self.draw_text_sharp("CRAFTING", panel_x + 15.0, panel_y + 28.0, 16.0, Color::from_rgba(255, 220, 100, 255));
        self.draw_text_sharp("[E] Close", panel_x + panel_width - 80.0, panel_y + 25.0, 16.0, GRAY);

        // Separator
        draw_line(panel_x + 10.0, panel_y + 40.0, panel_x + panel_width - 10.0, panel_y + 40.0, 1.0, GRAY);

        // Get unique categories
        let categories: Vec<&str> = {
            let mut cats: Vec<&str> = state.recipe_definitions.iter()
                .map(|r| r.category.as_str())
                .collect();
            cats.sort();
            cats.dedup();
            cats
        };

        if categories.is_empty() {
            self.draw_text_sharp("No recipes available", panel_x + 20.0, panel_y + 80.0, 16.0, GRAY);
            return;
        }

        // Category tabs
        let tab_y = panel_y + 55.0;
        let tab_height = 28.0;
        let mut tab_x = panel_x + 15.0;

        for (i, category) in categories.iter().enumerate() {
            let is_selected = i == state.ui_state.crafting_selected_category;
            let tab_width = self.measure_text_sharp(category, 16.0).width + 20.0;

            // Register tab bounds for click detection
            let bounds = Rect::new(tab_x, tab_y, tab_width, tab_height);
            layout.add(UiElementId::CraftingCategoryTab(i), bounds);

            // Check if this tab is hovered
            let is_hovered = matches!(hovered, Some(UiElementId::CraftingCategoryTab(idx)) if *idx == i);

            let bg_color = if is_selected {
                Color::from_rgba(70, 70, 100, 255)
            } else if is_hovered {
                Color::from_rgba(60, 60, 85, 255)
            } else {
                Color::from_rgba(50, 50, 70, 255)
            };
            let text_color = if is_selected || is_hovered { WHITE } else { LIGHTGRAY };

            draw_rectangle(tab_x, tab_y, tab_width, tab_height, bg_color);
            if is_selected {
                draw_rectangle_lines(tab_x, tab_y, tab_width, tab_height, 1.0, WHITE);
            } else if is_hovered {
                draw_rectangle_lines(tab_x, tab_y, tab_width, tab_height, 1.0, LIGHTGRAY);
            }

            // Capitalize first letter
            let display_name: String = category.chars().enumerate()
                .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
                .collect();
            self.draw_text_sharp(&display_name, tab_x + 10.0, tab_y + 19.0, 16.0, text_color);

            tab_x += tab_width + 5.0;
        }

        // Get recipes for current category
        let current_category = categories.get(state.ui_state.crafting_selected_category).copied().unwrap_or("consumables");
        let recipes: Vec<&RecipeDefinition> = state.recipe_definitions.iter()
            .filter(|r| r.category == current_category)
            .collect();

        // Split panel: left = recipe list, right = details
        let list_width = 220.0;
        let list_x = panel_x + 15.0;
        let content_y = tab_y + tab_height + 15.0;
        let content_height = panel_height - (content_y - panel_y) - 40.0;

        // Recipe list background
        draw_rectangle(list_x, content_y, list_width, content_height, Color::from_rgba(25, 25, 35, 255));
        draw_rectangle_lines(list_x, content_y, list_width, content_height, 1.0, Color::from_rgba(70, 70, 90, 255));

        // Recipe list
        let line_height = 26.0;
        let mut y = content_y + 5.0;

        for (i, recipe) in recipes.iter().enumerate() {
            if y > content_y + content_height - line_height {
                break;
            }

            let is_selected = i == state.ui_state.crafting_selected_recipe;

            // Register recipe item bounds for click detection
            let bounds = Rect::new(list_x + 2.0, y, list_width - 4.0, line_height - 2.0);
            layout.add(UiElementId::CraftingRecipeItem(i), bounds);

            // Check if this recipe is hovered
            let is_hovered = matches!(hovered, Some(UiElementId::CraftingRecipeItem(idx)) if *idx == i);

            if is_selected {
                draw_rectangle(list_x + 2.0, y, list_width - 4.0, line_height - 2.0, Color::from_rgba(60, 80, 120, 255));
            } else if is_hovered {
                draw_rectangle(list_x + 2.0, y, list_width - 4.0, line_height - 2.0, Color::from_rgba(50, 65, 100, 255));
            }

            let marker = if is_selected { ">" } else { " " };
            let text_color = if is_selected || is_hovered { WHITE } else { LIGHTGRAY };

            self.draw_text_sharp(&format!("{} {}", marker, recipe.display_name), list_x + 8.0, y + 18.0, 16.0, text_color);

            // Level indicator
            if recipe.level_required > 1 {
                let level_text = format!("Lv{}", recipe.level_required);
                let level_width = self.measure_text_sharp(&level_text, 16.0).width;
                self.draw_text_sharp(&level_text, list_x + list_width - level_width - 10.0, y + 16.0, 16.0, GRAY);
            }

            y += line_height;
        }

        // Detail panel
        let detail_x = list_x + list_width + 15.0;
        let detail_width = panel_width - list_width - 45.0;

        if let Some(recipe) = recipes.get(state.ui_state.crafting_selected_recipe) {
            // Recipe name
            self.draw_text_sharp(&recipe.display_name, detail_x, content_y + 22.0, 16.0, WHITE);

            // Description (wrapped to fit detail panel width)
            let desc_height = self.draw_text_wrapped(
                &recipe.description,
                detail_x,
                content_y + 45.0,
                16.0,
                LIGHTGRAY,
                detail_width - 10.0,  // Leave some padding
                20.0,  // Line height
            );

            // Track vertical offset after description
            let mut section_y = content_y + 45.0 + desc_height + 5.0;

            // Level requirement
            if recipe.level_required > 1 {
                let level_color = if let Some(player) = state.get_local_player() {
                    if player.level >= recipe.level_required { GREEN } else { RED }
                } else {
                    GRAY
                };
                self.draw_text_sharp(&format!("Requires Level {}", recipe.level_required), detail_x, section_y, 16.0, level_color);
                section_y += 25.0;
            }

            // Ingredients section
            self.draw_text_sharp("Ingredients:", detail_x, section_y, 16.0, Color::from_rgba(200, 200, 200, 255));

            let mut y = section_y + 20.0;
            let mut can_craft = true;

            for ingredient in &recipe.ingredients {
                let have_count = state.inventory.count_item_by_id(&ingredient.item_id);
                let need_count = ingredient.count;
                let has_enough = have_count >= need_count;

                if !has_enough {
                    can_craft = false;
                }

                let (marker, color) = if has_enough {
                    ("[v]", Color::from_rgba(100, 255, 100, 255))
                } else {
                    ("[x]", Color::from_rgba(255, 100, 100, 255))
                };

                // Look up display name from item registry
                let display_name = state.item_registry.get_display_name(&ingredient.item_id);
                let text = format!("{} {} ({}/{})", marker, display_name, have_count, need_count);
                self.draw_text_sharp(&text, detail_x + 10.0, y, 16.0, color);
                y += 20.0;
            }

            // Results section
            y += 10.0;
            self.draw_text_sharp("Creates:", detail_x, y, 16.0, Color::from_rgba(200, 200, 200, 255));
            y += 20.0;

            for result in &recipe.results {
                // Look up display name from item registry
                let display_name = state.item_registry.get_display_name(&result.item_id);
                let text = format!("  {} x{}", display_name, result.count);
                self.draw_text_sharp(&text, detail_x + 10.0, y, 16.0, Color::from_rgba(150, 200, 255, 255));
                y += 20.0;
            }

            // Craft button
            let craft_y = content_y + content_height - 25.0;
            let btn_width = if can_craft { 120.0 } else { 140.0 };

            // Register craft button bounds for click detection (only if can craft)
            if can_craft {
                let bounds = Rect::new(detail_x, craft_y, btn_width, 24.0);
                layout.add(UiElementId::CraftingButton, bounds);
            }

            // Check if craft button is hovered
            let is_btn_hovered = can_craft && matches!(hovered, Some(UiElementId::CraftingButton));

            if can_craft {
                let btn_color = if is_btn_hovered {
                    Color::from_rgba(70, 160, 70, 255)
                } else {
                    Color::from_rgba(50, 120, 50, 255)
                };
                draw_rectangle(detail_x, craft_y, btn_width, 24.0, btn_color);
                draw_rectangle_lines(detail_x, craft_y, btn_width, 24.0, 1.0, GREEN);
                self.draw_text_sharp("Craft", detail_x + 42.0, craft_y + 17.0, 16.0, WHITE);
            } else {
                draw_rectangle(detail_x, craft_y, btn_width, 24.0, Color::from_rgba(80, 50, 50, 255));
                self.draw_text_sharp("Missing Materials", detail_x + 10.0, craft_y + 17.0, 16.0, RED);
            }
        }

        // Navigation hints at bottom (updated to mention clicking)
        self.draw_text_sharp("Click or [A/D] Category   [W/S] Select   Click or [Enter/C] Craft   [E/Esc] Close",
            panel_x + 15.0, panel_y + panel_height - 15.0, 16.0, GRAY);
    }
}
