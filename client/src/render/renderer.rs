use macroquad::prelude::*;
use crate::game::{GameState, Player, Camera, ConnectionStatus, LayerType, GroundItem, ChunkLayerType, CHUNK_SIZE, ActiveDialogue, ActiveQuest, RecipeDefinition};
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

pub struct Renderer {
    player_color: Color,
    local_player_color: Color,
    /// Loaded tileset texture
    tileset: Option<Texture2D>,
    /// Player sprite sheet texture
    player_sprite: Option<Texture2D>,
    /// Multi-size pixel font for sharp text rendering at various sizes
    font: BitmapFont,
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

        // Try to load the player sprite sheet
        let player_sprite = match load_texture("assets/sprites/player_base_0.png").await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                log::info!("Loaded player sprite: {}x{}", tex.width(), tex.height());
                Some(tex)
            }
            Err(e) => {
                log::warn!("Failed to load player sprite: {}. Using fallback shapes.", e);
                None
            }
        };

        // Load monogram pixel font at multiple sizes for crisp rendering
        let font = BitmapFont::load_or_default("assets/fonts/monogram/ttf/monogram-extended.ttf").await;
        if font.is_loaded() {
            log::info!("Loaded monogram bitmap font at multiple sizes");
        } else {
            log::warn!("Failed to load monogram font, using default");
        }

        Self {
            player_color: Color::from_rgba(100, 150, 255, 255),
            local_player_color: Color::from_rgba(100, 255, 150, 255),
            tileset,
            player_sprite,
            font,
        }
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
    fn draw_tile_sprite(&self, screen_x: f32, screen_y: f32, tile_id: u32) {
        if let (Some(tileset), Some(uv)) = (&self.tileset, self.get_tile_uv(tile_id)) {
            // Center the tile on screen position
            let draw_x = screen_x - TILE_WIDTH / 2.0;
            let draw_y = screen_y - TILE_HEIGHT / 2.0;

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
                    dest_size: Some(Vec2::new(TILE_WIDTH, TILE_HEIGHT)),
                    ..Default::default()
                },
            );
        } else {
            // Fallback to colored tile
            let color = get_tile_color(tile_id);
            self.draw_isometric_tile(screen_x, screen_y, color);
        }
    }

    pub fn render(&self, state: &GameState) -> UiLayout {
        // 1. Render ground layer tiles
        self.render_tilemap_layer(state, LayerType::Ground);

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
                    self.draw_isometric_object(screen_x, screen_y, tile_id);
                }
            }
        }

        // 4. Render overhead layer (always on top)
        self.render_tilemap_layer(state, LayerType::Overhead);

        // 5. Render floating damage numbers
        self.render_damage_numbers(state);

        // 6. Render floating level up text
        self.render_level_up_events(state);

        // 7. Render UI (non-interactive elements)
        self.render_ui(state);

        // 8. Render interactive UI elements and return layout for hit detection
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
                            self.draw_tile_sprite(screen_x, screen_y, tile_id);

                            // Draw collision indicator in debug mode
                            if state.debug_mode && chunk.collision.get(idx).copied().unwrap_or(false) {
                                self.draw_collision_indicator(screen_x, screen_y);
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
                    self.draw_tile_sprite(screen_x, screen_y, tile_id);

                    // Draw collision indicator in debug mode
                    if state.debug_mode && tilemap.collision.get(idx).copied().unwrap_or(false) {
                        self.draw_collision_indicator(screen_x, screen_y);
                    }
                }
            }
        }
    }

    fn draw_collision_indicator(&self, screen_x: f32, screen_y: f32) {
        let half_w = TILE_WIDTH / 4.0;
        let half_h = TILE_HEIGHT / 4.0;
        draw_rectangle_lines(
            screen_x - half_w,
            screen_y - half_h,
            half_w * 2.0,
            half_h * 2.0,
            2.0,
            Color::from_rgba(255, 0, 0, 150),
        );
    }

    fn draw_isometric_object(&self, screen_x: f32, screen_y: f32, tile_id: u32) {
        // Draw shadow ellipse for objects
        draw_ellipse(screen_x, screen_y + 4.0, 24.0, 16.0, 0.0, Color::from_rgba(0, 0, 0, 50));

        // Draw object tile sprite (slightly elevated)
        let elevated_y = screen_y - TILE_HEIGHT * 0.25;
        self.draw_tile_sprite(screen_x, elevated_y, tile_id);
    }

    fn draw_isometric_tile(&self, screen_x: f32, screen_y: f32, color: Color) {
        // Draw a diamond-shaped tile
        let half_w = TILE_WIDTH / 2.0;
        let half_h = TILE_HEIGHT / 2.0;

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
        let center_y = center_y - TILE_HEIGHT / 2.0;

        // Tile dimensions (half-sizes for diamond corners)
        let half_w = TILE_WIDTH / 2.0;
        let half_h = TILE_HEIGHT / 2.0;

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
        draw_line(top.0, top.1, right.0, right.1, 2.0, color);
        draw_line(right.0, right.1, bottom.0, bottom.1, 2.0, color);
        draw_line(bottom.0, bottom.1, left.0, left.1, 2.0, color);
        draw_line(left.0, left.1, top.0, top.1, 2.0, color);
    }

    fn render_player(&self, player: &Player, is_local: bool, is_selected: bool, camera: &Camera) {
        let (screen_x, screen_y) = world_to_screen(player.x, player.y, camera);

        // Dead players are faded
        let alpha = if player.is_dead { 100 } else { 255 };

        // Selection highlight (draw first, behind player)
        if is_selected && !player.is_dead {
            self.render_tile_selection(player.x, player.y, camera);
        }

        // Draw shadow under player
        draw_ellipse(screen_x, screen_y, 16.0, 7.0, 0.0, Color::from_rgba(0, 0, 0, 60));

        // Try to render sprite, fall back to colored circle
        if let Some(sprite) = &self.player_sprite {
            let coords = player.animation.get_sprite_coords();
            let (src_x, src_y, src_w, src_h) = coords.to_source_rect();

            // Tint for local player distinction (slight green tint)
            let tint = if is_local {
                Color::from_rgba(220, 255, 220, alpha)
            } else {
                Color::from_rgba(255, 255, 255, alpha)
            };

            // Position sprite so feet are at screen_y
            let draw_x = screen_x - SPRITE_WIDTH / 2.0;
            let draw_y = screen_y - SPRITE_HEIGHT + 8.0; // Offset to align feet with tile

            draw_texture_ex(
                sprite,
                draw_x,
                draw_y,
                tint,
                DrawTextureParams {
                    source: Some(Rect::new(src_x, src_y, src_w, src_h)),
                    dest_size: Some(Vec2::new(SPRITE_WIDTH, SPRITE_HEIGHT)),
                    flip_x: coords.flip_h,
                    ..Default::default()
                },
            );
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

            let radius = 12.0;
            draw_circle(screen_x, screen_y - radius, radius, color);

            // Direction indicator
            let (dx, dy) = player.direction.to_unit_vector();
            let indicator_len = 15.0;
            draw_line(
                screen_x,
                screen_y - radius,
                screen_x + dx * indicator_len,
                screen_y - radius + dy * indicator_len * 0.5, // Flatten for isometric
                2.0,
                WHITE,
            );
        }

        // Player name
        let name_y_offset = if self.player_sprite.is_some() { SPRITE_HEIGHT - 8.0 } else { 24.0 };
        let name_width = self.measure_text_sharp(&player.name, 16.0).width;
        self.draw_text_sharp(
            &player.name,
            screen_x - name_width / 2.0,
            screen_y - name_y_offset - 5.0,
            16.0,
            WHITE,
        );

        // Health bar (if not full HP)
        if player.hp < player.max_hp {
            let bar_width = 30.0;
            let bar_height = 4.0;
            let bar_x = screen_x - bar_width / 2.0;
            let bar_y = screen_y - name_y_offset - 20.0;

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

        // Don't render dead NPCs (or render them faded)
        if npc.state == NpcState::Dead {
            // Draw faded corpse
            let fade_color = Color::from_rgba(50, 80, 50, 100);
            draw_circle(screen_x, screen_y - 8.0, 16.0, fade_color);
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
        let radius = 10.0 + wobble * 1.5;
        let height_offset = 8.0 + wobble * 2.0;

        // Draw shadow
        draw_ellipse(screen_x, screen_y, 16.0, 6.0, 0.0, Color::from_rgba(0, 0, 0, 60));

        // Draw NPC body (oval blob) - TODO: use sprites based on entity_type
        draw_ellipse(screen_x, screen_y - height_offset, radius, radius * 0.7, 0.0, base_color);

        // Highlight
        draw_ellipse(screen_x - 3.0, screen_y - height_offset - 2.0, radius * 0.3, radius * 0.2, 0.0, highlight_color);

        // Interaction indicator for friendly NPCs (yellow exclamation mark above head)
        if !npc.is_hostile() {
            let pulse = (macroquad::time::get_time() * 2.0).sin() as f32 * 0.2 + 0.8;
            let indicator_y = screen_y - height_offset - radius - 25.0;
            self.draw_text_sharp("!", screen_x - 3.0, indicator_y, 16.0, Color::from_rgba(255, 220, 50, (pulse * 255.0) as u8));
        }

        // NPC name with level
        let name = npc.name();
        let name_width = self.measure_text_sharp(&name, 16.0).width;
        self.draw_text_sharp(
            &name,
            screen_x - name_width / 2.0,
            screen_y - height_offset - radius - 5.0,
            16.0,
            name_color,
        );

        // Health bar (only show for hostile NPCs or when damaged)
        if npc.is_hostile() || npc.hp < npc.max_hp {
            let bar_width = 28.0;
            let bar_height = 3.0;
            let bar_x = screen_x - bar_width / 2.0;
            let bar_y = screen_y - height_offset - radius - 18.0;

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

        // Bobbing animation
        let time = macroquad::time::get_time();
        let bob = ((time - item.animation_time) * 3.0).sin() as f32 * 2.0;

        // Draw shadow
        draw_ellipse(screen_x, screen_y, 8.0, 4.0, 0.0, Color::from_rgba(0, 0, 0, 40));

        // Draw item (colored rectangle based on category from registry)
        let item_y = screen_y - 8.0 - bob;
        let item_def = state.item_registry.get_or_placeholder(&item.item_id);
        let color = item_def.category_color();

        // Draw item shape
        draw_rectangle(screen_x - 6.0, item_y - 6.0, 16.0, 12.0, color);
        draw_rectangle_lines(screen_x - 6.0, item_y - 6.0, 16.0, 12.0, 1.0, WHITE);

        // Draw quantity if > 1
        if item.quantity > 1 {
            let qty_text = format!("x{}", item.quantity);
            let text_width = self.measure_text_sharp(&qty_text, 16.0).width;
            self.draw_text_sharp(&qty_text, screen_x - text_width / 2.0, item_y + 14.0, 16.0, WHITE);
        }
    }

    fn render_ui(&self, state: &GameState) {
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
            self.draw_text_sharp("WASD: Move | Space: Attack | I: Inventory | E: Interact | Q: Quests | F: Pickup | F3: Debug", 16.0, screen_height() - 10.0, 16.0, GRAY);
        }
    }

    /// Render all interactive UI elements and return the layout for hit detection
    fn render_interactive_ui(&self, state: &GameState) -> UiLayout {
        let mut layout = UiLayout::new();
        let hovered = &state.ui_state.hovered_element;

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

        layout
    }

    fn render_inventory(&self, state: &GameState, hovered: &Option<UiElementId>, layout: &mut UiLayout) {
        let inv_width = 240.0;
        let inv_height = 320.0;
        let inv_x = (screen_width() - inv_width) / 2.0;
        let inv_y = (screen_height() - inv_height) / 2.0;
        let slot_size = 40.0;
        let slots_per_row = 5;

        // Background
        draw_rectangle(inv_x, inv_y, inv_width, inv_height, Color::from_rgba(30, 30, 40, 220));
        draw_rectangle_lines(inv_x, inv_y, inv_width, inv_height, 2.0, WHITE);

        // Title
        self.draw_text_sharp("Inventory", inv_x + 10.0, inv_y + 25.0, 16.0, WHITE);
        self.draw_text_sharp(&format!("Gold: {}", state.inventory.gold), inv_x + inv_width - 100.0, inv_y + 25.0, 16.0, GOLD);

        // Slots
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

            // Slot background with hover effect
            let bg_color = if is_hovered {
                Color::from_rgba(70, 70, 90, 255)
            } else {
                Color::from_rgba(50, 50, 60, 255)
            };
            draw_rectangle(x, y, slot_size - 4.0, slot_size - 4.0, bg_color);

            // Border with hover effect
            let border_color = if is_hovered { WHITE } else { GRAY };
            draw_rectangle_lines(x, y, slot_size - 4.0, slot_size - 4.0, 1.0, border_color);

            // Draw item if present
            if let Some(slot) = &state.inventory.slots[i] {
                let item_def = state.item_registry.get_or_placeholder(&slot.item_id);
                let color = item_def.category_color();
                draw_rectangle(x + 4.0, y + 4.0, slot_size - 12.0, slot_size - 12.0, color);

                // Quantity
                if slot.quantity > 1 {
                    self.draw_text_sharp(&slot.quantity.to_string(), x + 2.0, y + slot_size - 8.0, 16.0, WHITE);
                }
            }

            // Show slot number for first 5 (quick slots)
            if i < 5 {
                self.draw_text_sharp(&(i + 1).to_string(), x + slot_size - 14.0, y + 12.0, 16.0, GRAY);
            }
        }

        // Close hint
        self.draw_text_sharp("Press I to close", inv_x + 10.0, inv_y + inv_height - 15.0, 16.0, GRAY);
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
        let slot_size = 36.0;
        let padding = 4.0;
        let total_width = 5.0 * (slot_size + padding) - padding;
        let start_x = (screen_width() - total_width) / 2.0;
        let start_y = screen_height() - slot_size - 40.0;

        for i in 0..5 {
            let x = start_x + i as f32 * (slot_size + padding);
            let y = start_y;

            // Register slot bounds for hit detection
            let bounds = Rect::new(x, y, slot_size, slot_size);
            layout.add(UiElementId::QuickSlot(i), bounds);

            // Check if this slot is hovered
            let is_hovered = matches!(hovered, Some(UiElementId::QuickSlot(idx)) if *idx == i);

            // Slot background with hover effect
            let bg_color = if is_hovered {
                Color::from_rgba(50, 50, 70, 255)
            } else {
                Color::from_rgba(30, 30, 40, 200)
            };
            draw_rectangle(x, y, slot_size, slot_size, bg_color);

            // Border with hover effect
            let border_color = if is_hovered { WHITE } else { GRAY };
            draw_rectangle_lines(x, y, slot_size, slot_size, 1.0, border_color);

            // Draw item if present
            if let Some(slot) = &state.inventory.slots[i] {
                let item_def = state.item_registry.get_or_placeholder(&slot.item_id);
                let color = item_def.category_color();
                draw_rectangle(x + 4.0, y + 4.0, slot_size - 8.0, slot_size - 8.0, color);

                // Quantity
                if slot.quantity > 1 {
                    self.draw_text_sharp(&slot.quantity.to_string(), x + 2.0, y + slot_size - 4.0, 16.0, WHITE);
                }
            }

            // Slot number
            self.draw_text_sharp(&(i + 1).to_string(), x + slot_size - 10.0, y + 12.0, 16.0, WHITE);
        }
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

        // Get mouse position for tooltip placement
        let (mouse_x, mouse_y) = mouse_position();

        // Calculate tooltip dimensions based on content
        let padding = 10.0;
        let line_height = 18.0;
        let tooltip_width = 200.0;

        // Count lines: name, category, description (may wrap), quantity if > 1
        let mut lines = 2; // name + category always
        if !item_def.description.is_empty() {
            lines += 1;
        }
        if slot.quantity > 1 {
            lines += 1;
        }

        let tooltip_height = padding * 2.0 + lines as f32 * line_height;

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

        // Item name (colored by category)
        let name_color = item_def.category_color();
        self.draw_text_sharp(&item_def.display_name, tooltip_x + padding, y, 16.0, name_color);
        y += line_height;

        // Category
        let category_text = format!("[{}]", item_def.category);
        self.draw_text_sharp(&category_text, tooltip_x + padding, y, 16.0, GRAY);
        y += line_height;

        // Description
        if !item_def.description.is_empty() {
            self.draw_text_sharp(&item_def.description, tooltip_x + padding, y, 16.0, LIGHTGRAY);
            y += line_height;
        }

        // Quantity (if more than 1)
        if slot.quantity > 1 {
            let qty_text = format!("Quantity: {}", slot.quantity);
            self.draw_text_sharp(&qty_text, tooltip_x + padding, y, 16.0, WHITE);
        }
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

            // Float up slightly
            let float_offset = (age * 10.0).min(30.0);

            // Position at top-center
            let y = 120.0 - float_offset;

            // "QUEST COMPLETE!" banner
            let title = "QUEST COMPLETE!";
            let title_width = self.measure_text_sharp(title, 32.0).width;
            let x = (screen_width() - title_width) / 2.0;

            // Outline
            let outline_color = Color::from_rgba(0, 0, 0, alpha);
            for ox in [-2.0, 2.0] {
                for oy in [-2.0, 2.0] {
                    self.draw_text_sharp(title, x + ox, y + oy, 32.0, outline_color);
                }
            }

            // Main text (gold)
            self.draw_text_sharp(title, x, y, 32.0, Color::from_rgba(255, 215, 0, alpha));

            // Quest name
            let name_width = self.measure_text_sharp(&event.quest_name, 16.0).width;
            self.draw_text_sharp(
                &event.quest_name,
                (screen_width() - name_width) / 2.0,
                y + 25.0,
                16.0,
                Color::from_rgba(255, 255, 255, alpha),
            );

            // Rewards
            let rewards = format!("+{} EXP  +{} Gold", event.exp_reward, event.gold_reward);
            let rewards_width = self.measure_text_sharp(&rewards, 16.0).width;
            self.draw_text_sharp(
                &rewards,
                (screen_width() - rewards_width) / 2.0,
                y + 45.0,
                16.0,
                Color::from_rgba(100, 255, 100, alpha),
            );
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
