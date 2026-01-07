use macroquad::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

/// Tiled JSON tilemap format (simplified for isometric maps)
#[derive(Debug, Deserialize)]
pub struct TiledMap {
    pub width: u32,
    pub height: u32,
    pub tilewidth: u32,
    pub tileheight: u32,
    pub orientation: String,
    pub layers: Vec<TiledLayer>,
    pub tilesets: Vec<TiledTileset>,
}

#[derive(Debug, Deserialize)]
pub struct TiledLayer {
    pub name: String,
    #[serde(default)]
    pub data: Vec<u32>,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub visible: bool,
    #[serde(rename = "type")]
    pub layer_type: String,
    #[serde(default)]
    pub objects: Vec<TiledObject>,
}

#[derive(Debug, Deserialize)]
pub struct TiledObject {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(rename = "type", default)]
    pub object_type: String,
}

#[derive(Debug, Deserialize)]
pub struct TiledTileset {
    pub firstgid: u32,
    pub name: String,
    pub tilewidth: u32,
    pub tileheight: u32,
    pub tilecount: u32,
    pub columns: u32,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub imagewidth: u32,
    #[serde(default)]
    pub imageheight: u32,
    #[serde(default)]
    pub tiles: Vec<TileProperties>,
}

#[derive(Debug, Deserialize)]
pub struct TileProperties {
    pub id: u32,
    #[serde(default)]
    pub properties: Vec<TileProperty>,
}

#[derive(Debug, Deserialize)]
pub struct TileProperty {
    pub name: String,
    #[serde(rename = "type")]
    pub property_type: String,
    pub value: serde_json::Value,
}

/// Processed tilemap ready for rendering
pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub layers: Vec<TilemapLayer>,
    pub collision: Vec<bool>, // Flattened collision data
    pub spawn_points: Vec<(f32, f32)>,
}

pub struct TilemapLayer {
    pub name: String,
    pub tiles: Vec<u32>, // Tile IDs (0 = empty)
    pub layer_type: LayerType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    Ground,
    Objects,
    Overhead,
}

impl Tilemap {
    /// Load tilemap from embedded Tiled JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        let tiled: TiledMap = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse tilemap: {}", e))?;

        if tiled.orientation != "isometric" {
            log::warn!("Tilemap orientation is '{}', expected 'isometric'", tiled.orientation);
        }

        let mut layers = Vec::new();
        let mut collision = vec![false; (tiled.width * tiled.height) as usize];
        let mut spawn_points = Vec::new();

        for layer in &tiled.layers {
            match layer.layer_type.as_str() {
                "tilelayer" => {
                    let layer_type = match layer.name.to_lowercase().as_str() {
                        "ground" | "floor" | "terrain" => LayerType::Ground,
                        "overhead" | "roof" | "sky" => LayerType::Overhead,
                        _ => LayerType::Objects,
                    };

                    layers.push(TilemapLayer {
                        name: layer.name.clone(),
                        tiles: layer.data.clone(),
                        layer_type,
                    });
                }
                "objectgroup" => {
                    for obj in &layer.objects {
                        match obj.object_type.to_lowercase().as_str() {
                            "collision" => {
                                // Mark collision tiles
                                let start_x = (obj.x / tiled.tilewidth as f32) as u32;
                                let start_y = (obj.y / tiled.tileheight as f32) as u32;
                                let end_x = start_x + (obj.width / tiled.tilewidth as f32).ceil() as u32;
                                let end_y = start_y + (obj.height / tiled.tileheight as f32).ceil() as u32;

                                for y in start_y..end_y.min(tiled.height) {
                                    for x in start_x..end_x.min(tiled.width) {
                                        let idx = (y * tiled.width + x) as usize;
                                        if idx < collision.len() {
                                            collision[idx] = true;
                                        }
                                    }
                                }
                            }
                            "spawn" | "spawnpoint" => {
                                // Convert pixel coords to tile coords
                                let tile_x = obj.x / tiled.tilewidth as f32;
                                let tile_y = obj.y / tiled.tileheight as f32;
                                spawn_points.push((tile_x, tile_y));
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        // If no spawn points defined, add a default one
        if spawn_points.is_empty() {
            spawn_points.push((tiled.width as f32 / 2.0, tiled.height as f32 / 2.0));
        }

        Ok(Self {
            width: tiled.width,
            height: tiled.height,
            tile_width: tiled.tilewidth,
            tile_height: tiled.tileheight,
            layers,
            collision,
            spawn_points,
        })
    }

    /// Create a simple procedural tilemap for testing
    pub fn new_test_map(width: u32, height: u32) -> Self {
        let mut ground_tiles = vec![0u32; (width * height) as usize];
        let mut collision = vec![false; (width * height) as usize];

        // Fill with grass (tile 1) and some variation
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;

                // Base grass
                ground_tiles[idx] = 1;

                // Add some variety
                let noise = ((x as f32 * 0.3).sin() + (y as f32 * 0.4).cos()).abs();
                if noise > 0.8 {
                    ground_tiles[idx] = 2; // Dirt
                }

                // Water edges
                if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    ground_tiles[idx] = 3; // Water/edge
                    collision[idx] = true;
                }

                // Some rocks/obstacles
                if (x + y * 3) % 17 == 0 && x > 2 && y > 2 && x < width - 3 && y < height - 3 {
                    ground_tiles[idx] = 4; // Rock
                    collision[idx] = true;
                }
            }
        }

        Self {
            width,
            height,
            tile_width: 64,
            tile_height: 32,
            layers: vec![TilemapLayer {
                name: "ground".to_string(),
                tiles: ground_tiles,
                layer_type: LayerType::Ground,
            }],
            collision,
            spawn_points: vec![(width as f32 / 2.0, height as f32 / 2.0)],
        }
    }

    /// Get tile ID at position
    pub fn get_tile(&self, layer_idx: usize, x: u32, y: u32) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        if layer_idx >= self.layers.len() {
            return 0;
        }
        let idx = (y * self.width + x) as usize;
        self.layers[layer_idx].tiles.get(idx).copied().unwrap_or(0)
    }

    /// Check if position is walkable
    pub fn is_walkable(&self, x: f32, y: f32) -> bool {
        // Check bounds
        if x < 0.0 || y < 0.0 || x >= self.width as f32 || y >= self.height as f32 {
            return false;
        }

        let idx = (y as u32 * self.width + x as u32) as usize;
        !self.collision.get(idx).copied().unwrap_or(true)
    }

    /// Check if a rectangle overlaps any collision
    pub fn check_collision(&self, x: f32, y: f32, half_width: f32, half_height: f32) -> bool {
        // Check corners and center
        let points = [
            (x, y),
            (x - half_width, y - half_height),
            (x + half_width, y - half_height),
            (x - half_width, y + half_height),
            (x + half_width, y + half_height),
        ];

        for (px, py) in points {
            if !self.is_walkable(px, py) {
                return true;
            }
        }
        false
    }
}

/// Tile colors for procedural rendering (before we have sprites)
pub fn get_tile_color(tile_id: u32) -> Color {
    match tile_id {
        0 => Color::from_rgba(0, 0, 0, 0),          // Empty/transparent
        1 => Color::from_rgba(60, 90, 50, 255),     // Grass
        2 => Color::from_rgba(90, 70, 50, 255),     // Dirt
        3 => Color::from_rgba(40, 60, 100, 255),    // Water
        4 => Color::from_rgba(80, 80, 90, 255),     // Rock
        5 => Color::from_rgba(50, 80, 45, 255),     // Dark grass
        6 => Color::from_rgba(100, 85, 60, 255),    // Sand
        7 => Color::from_rgba(70, 70, 75, 255),     // Stone floor
        8 => Color::from_rgba(60, 50, 40, 255),     // Wood
        _ => Color::from_rgba(100, 50, 100, 255),   // Unknown (purple for debugging)
    }
}
