use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::chunk::{world_to_local, Chunk, ChunkCoord, ChunkLayer, ChunkLayerType, EntitySpawn, MapObject, Portal, Wall, WallEdge, CHUNK_SIZE};

/// World manager that handles loading and caching chunks
pub struct World {
    chunks: RwLock<HashMap<ChunkCoord, Arc<Chunk>>>,
    chunk_dir: String,
    /// If true, generate test chunks for missing files
    generate_missing: bool,
}

impl World {
    pub fn new(chunk_dir: &str) -> Self {
        Self {
            chunks: RwLock::new(HashMap::new()),
            chunk_dir: chunk_dir.to_string(),
            generate_missing: true, // For development
        }
    }

    /// Get or load a chunk at the given coordinates
    pub async fn get_or_load_chunk(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        // Check cache first
        {
            let chunks = self.chunks.read().await;
            if let Some(chunk) = chunks.get(&coord) {
                return Some(chunk.clone());
            }
        }

        // Try to load from file
        let chunk = self.load_chunk_from_file(coord).await;

        if let Some(chunk) = chunk {
            let chunk = Arc::new(chunk);
            let mut chunks = self.chunks.write().await;
            chunks.insert(coord, chunk.clone());
            return Some(chunk);
        }

        // Generate test chunk if enabled
        if self.generate_missing {
            let chunk = Arc::new(Chunk::new_test(coord));
            info!("Generated test chunk at ({}, {})", coord.x, coord.y);
            let mut chunks = self.chunks.write().await;
            chunks.insert(coord, chunk.clone());
            return Some(chunk);
        }

        None
    }

    /// Load chunk from JSON file (supports both Tiled and simplified formats)
    async fn load_chunk_from_file(&self, coord: ChunkCoord) -> Option<Chunk> {
        let filename = format!("chunk_{}_{}.json", coord.x, coord.y);
        let path = Path::new(&self.chunk_dir).join(&filename);

        if !path.exists() {
            return None;
        }

        match tokio::fs::read_to_string(&path).await {
            Ok(json) => {
                // Detect format by checking for version field
                let value: serde_json::Value = match serde_json::from_str(&json) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("Failed to parse JSON {:?}: {}", path, e);
                        return None;
                    }
                };

                let result = if value.get("version").is_some() {
                    // New simplified format (has version field)
                    self.parse_simplified_json(coord, &value)
                } else {
                    // Legacy Tiled format
                    self.parse_tiled_json(coord, &json)
                };

                match result {
                    Ok(chunk) => {
                        info!("Loaded chunk from {:?}", path);
                        Some(chunk)
                    }
                    Err(e) => {
                        warn!("Failed to parse chunk {:?}: {}", path, e);
                        None
                    }
                }
            },
            Err(e) => {
                warn!("Failed to read chunk {:?}: {}", path, e);
                None
            }
        }
    }

    /// Parse new simplified JSON format (version 2+)
    fn parse_simplified_json(&self, coord: ChunkCoord, value: &serde_json::Value) -> Result<Chunk, String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

        let size = value["size"].as_u64().unwrap_or(CHUNK_SIZE as u64) as u32;
        if size != CHUNK_SIZE {
            return Err(format!(
                "Chunk size mismatch: expected {}, got {}",
                CHUNK_SIZE, size
            ));
        }

        let mut chunk = Chunk::new(coord);

        // Parse layers
        if let Some(layers) = value.get("layers") {
            // Ground layer
            if let Some(ground) = layers["ground"].as_array() {
                let tiles: Vec<u32> = ground
                    .iter()
                    .map(|v| v.as_u64().unwrap_or(0) as u32)
                    .collect();
                if tiles.len() == (CHUNK_SIZE * CHUNK_SIZE) as usize {
                    chunk.layers[0].tiles = tiles;
                }
            }

            // Objects layer
            if let Some(objects) = layers["objects"].as_array() {
                let tiles: Vec<u32> = objects
                    .iter()
                    .map(|v| v.as_u64().unwrap_or(0) as u32)
                    .collect();
                if tiles.len() == (CHUNK_SIZE * CHUNK_SIZE) as usize {
                    chunk.layers[1].tiles = tiles;
                }
            }

            // Overhead layer
            if let Some(overhead) = layers["overhead"].as_array() {
                let tiles: Vec<u32> = overhead
                    .iter()
                    .map(|v| v.as_u64().unwrap_or(0) as u32)
                    .collect();
                if tiles.len() == (CHUNK_SIZE * CHUNK_SIZE) as usize {
                    chunk.layers[2].tiles = tiles;
                }
            }
        }

        // Parse collision from base64
        if let Some(collision_b64) = value["collision"].as_str() {
            if let Ok(collision_bytes) = BASE64.decode(collision_b64) {
                chunk.collision = Chunk::unpack_collision(&collision_bytes);
            }
        }

        // Parse entities
        if let Some(entities) = value["entities"].as_array() {
            for entity in entities {
                let entity_id = entity["entityId"].as_str().unwrap_or("").to_string();
                if entity_id.is_empty() {
                    continue;
                }

                let local_x = entity["x"].as_u64().unwrap_or(0) as u32;
                let local_y = entity["y"].as_u64().unwrap_or(0) as u32;
                let world_x = coord.x * CHUNK_SIZE as i32 + local_x as i32;
                let world_y = coord.y * CHUNK_SIZE as i32 + local_y as i32;

                let level = entity["level"].as_i64().unwrap_or(1) as i32;
                let respawn = entity["respawn"].as_bool().unwrap_or(true);
                let facing = entity["facing"].as_str().map(|s| s.to_string());
                let unique_id = entity["uniqueId"].as_str().map(|s| s.to_string());

                chunk.entity_spawns.push(EntitySpawn {
                    entity_id,
                    world_x,
                    world_y,
                    level,
                    respawn,
                    respawn_time_override: None,
                    facing,
                    unique_id,
                });
            }
        }

        // Parse map objects (new cartesian format)
        if let Some(map_objects) = value["mapObjects"].as_array() {
            for obj in map_objects {
                let gid = obj["gid"].as_u64().unwrap_or(0) as u32;
                if gid == 0 {
                    continue;
                }

                let local_x = obj["x"].as_i64().unwrap_or(0) as i32;
                let local_y = obj["y"].as_i64().unwrap_or(0) as i32;
                let world_x = coord.x * CHUNK_SIZE as i32 + local_x;
                let world_y = coord.y * CHUNK_SIZE as i32 + local_y;
                let width = obj["width"].as_u64().unwrap_or(64) as u32;
                let height = obj["height"].as_u64().unwrap_or(64) as u32;

                chunk.objects.push(MapObject {
                    gid,
                    tile_x: world_x,
                    tile_y: world_y,
                    width,
                    height,
                });
            }
        }

        // Parse walls
        if let Some(walls_array) = value["walls"].as_array() {
            for wall_value in walls_array {
                if let (Some(gid), Some(x), Some(y), Some(edge_str)) = (
                    wall_value["gid"].as_u64(),
                    wall_value["x"].as_i64(),
                    wall_value["y"].as_i64(),
                    wall_value["edge"].as_str(),
                ) {
                    let edge = match edge_str {
                        "down" => WallEdge::Down,
                        "right" => WallEdge::Right,
                        _ => continue,
                    };
                    chunk.walls.push(Wall {
                        gid: gid as u32,
                        tile_x: coord.x * CHUNK_SIZE as i32 + x as i32,
                        tile_y: coord.y * CHUNK_SIZE as i32 + y as i32,
                        edge,
                    });
                }
            }
        }

        // Parse portals
        let portals: Vec<Portal> = value
            .get("portals")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        chunk.portals = portals;

        Ok(chunk)
    }

    /// Parse Tiled JSON format into a Chunk
    fn parse_tiled_json(&self, coord: ChunkCoord, json: &str) -> Result<Chunk, String> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))?;

        let width = value["width"].as_u64().unwrap_or(CHUNK_SIZE as u64) as u32;
        let height = value["height"].as_u64().unwrap_or(CHUNK_SIZE as u64) as u32;

        if width != CHUNK_SIZE || height != CHUNK_SIZE {
            return Err(format!(
                "Chunk size mismatch: expected {}x{}, got {}x{}",
                CHUNK_SIZE, CHUNK_SIZE, width, height
            ));
        }

        let mut chunk = Chunk::new(coord);

        // Parse layers
        if let Some(layers) = value["layers"].as_array() {
            for layer_value in layers {
                let name = layer_value["name"].as_str().unwrap_or("");
                let layer_type_str = layer_value["type"].as_str().unwrap_or("");

                if layer_type_str == "tilelayer" {
                    // Get tile data
                    if let Some(data) = layer_value["data"].as_array() {
                        let tiles: Vec<u32> = data
                            .iter()
                            .map(|v| v.as_u64().unwrap_or(0) as u32)
                            .collect();

                        if tiles.len() == (CHUNK_SIZE * CHUNK_SIZE) as usize {
                            // Check if this is a collision layer
                            if name.to_lowercase().contains("collision") {
                                // Mark collision for any non-zero tile
                                for (idx, &tile_id) in tiles.iter().enumerate() {
                                    if tile_id != 0 {
                                        chunk.collision[idx] = true;
                                    }
                                }
                            } else {
                                // Regular tile layer - determine type from name
                                let layer_type = if name.to_lowercase().contains("ground") {
                                    ChunkLayerType::Ground
                                } else if name.to_lowercase().contains("overhead") {
                                    ChunkLayerType::Overhead
                                } else {
                                    ChunkLayerType::Objects
                                };

                                // Find or create layer
                                let layer_idx = chunk
                                    .layers
                                    .iter()
                                    .position(|l| l.layer_type == layer_type)
                                    .unwrap_or_else(|| {
                                        chunk.layers.push(ChunkLayer::new(layer_type));
                                        chunk.layers.len() - 1
                                    });
                                chunk.layers[layer_idx].tiles = tiles;
                            }
                        }
                    }
                } else if layer_type_str == "objectgroup" {
                    // Get layer offset (Tiled applies this to object positions)
                    let offset_x = layer_value["offsetx"].as_f64().unwrap_or(0.0);
                    let offset_y = layer_value["offsety"].as_f64().unwrap_or(0.0);

                    // Parse object groups for collision, spawn points, and map objects
                    if let Some(objects) = layer_value["objects"].as_array() {
                        for obj in objects {
                            let obj_type = obj["type"].as_str().unwrap_or("");
                            let pixel_x = obj["x"].as_f64().unwrap_or(0.0);
                            let pixel_y = obj["y"].as_f64().unwrap_or(0.0);
                            let obj_width = obj["width"].as_f64().unwrap_or(0.0);
                            let obj_height = obj["height"].as_f64().unwrap_or(0.0);

                            // Check if this object has a gid (it's a tileset object like tree/rock)
                            if let Some(gid) = obj["gid"].as_u64() {
                                // Tiled isometric: pixel coords use tileHeight (32) for both
                                let tile_x = (pixel_x / 32.0).floor() as i32;
                                let tile_y = (pixel_y / 32.0).floor() as i32;

                                // Convert to world coordinates (add chunk offset)
                                let world_tile_x = coord.x * CHUNK_SIZE as i32 + tile_x;
                                let world_tile_y = coord.y * CHUNK_SIZE as i32 + tile_y;

                                info!("SERVER Object gid {} | pixel ({:.1}, {:.1}) | local ({}, {}) | chunk ({}, {}) | WORLD ({}, {})",
                                    gid, pixel_x, pixel_y, tile_x, tile_y, coord.x, coord.y, world_tile_x, world_tile_y);

                                chunk.objects.push(MapObject {
                                    gid: gid as u32,
                                    tile_x: world_tile_x,
                                    tile_y: world_tile_y,
                                    width: obj_width as u32,
                                    height: obj_height as u32,
                                });
                            } else {
                                // Legacy handling for non-gid objects (collision, spawn points)
                                let x = (pixel_x / 64.0) as u32;
                                let y = (pixel_y / 32.0) as u32;
                                let collision_width = (obj_width / 64.0).ceil() as u32;
                                let collision_height = (obj_height / 32.0).ceil() as u32;

                                if obj_type == "collision" || name.to_lowercase().contains("collision") {
                                    // Mark collision area
                                    for dy in 0..collision_height.max(1) {
                                        for dx in 0..collision_width.max(1) {
                                            chunk.set_collision(x + dx, y + dy, true);
                                        }
                                    }
                                } else if obj_type == "entity_spawn" || obj_type == "npc_spawn" {
                                    // Parse entity spawn from Tiled object properties
                                    if let Some(spawn) = self.parse_entity_spawn(obj, coord) {
                                        info!("Found entity spawn: {} at ({}, {})",
                                            spawn.entity_id, spawn.world_x, spawn.world_y);
                                        chunk.entity_spawns.push(spawn);
                                    }
                                } else if obj_type == "player_spawn" || obj_type == "spawn"
                                    || name.to_lowercase().contains("spawn")
                                {
                                    chunk.player_spawns.push((x, y));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(chunk)
    }

    /// Parse an entity spawn object from Tiled JSON
    fn parse_entity_spawn(&self, obj: &serde_json::Value, chunk_coord: ChunkCoord) -> Option<EntitySpawn> {
        // Get pixel position and convert to grid
        let pixel_x = obj["x"].as_f64()?;
        let pixel_y = obj["y"].as_f64()?;
        let local_x = (pixel_x / 64.0) as u32;
        let local_y = (pixel_y / 32.0) as u32;

        // Convert to world coordinates
        let world_x = chunk_coord.x * CHUNK_SIZE as i32 + local_x as i32;
        let world_y = chunk_coord.y * CHUNK_SIZE as i32 + local_y as i32;

        // Parse properties array
        let properties = obj["properties"].as_array();

        // Extract entity_id (required)
        let entity_id = self.get_property_string(properties, "entity_id")?;

        // Extract optional properties
        let level = self.get_property_int(properties, "level").unwrap_or(1);
        let respawn = self.get_property_bool(properties, "respawn").unwrap_or(true);
        let respawn_time_override = self.get_property_int(properties, "respawn_time_override")
            .map(|v| v as u64);
        let facing = self.get_property_string(properties, "facing");
        let unique_id = self.get_property_string(properties, "unique_id");

        Some(EntitySpawn {
            entity_id,
            world_x,
            world_y,
            level,
            respawn,
            respawn_time_override,
            facing,
            unique_id,
        })
    }

    /// Get a string property from Tiled properties array
    fn get_property_string(&self, props: Option<&Vec<serde_json::Value>>, name: &str) -> Option<String> {
        props?.iter()
            .find(|p| p["name"].as_str() == Some(name))
            .and_then(|p| p["value"].as_str().map(|s| s.to_string()))
    }

    /// Get an int property from Tiled properties array
    fn get_property_int(&self, props: Option<&Vec<serde_json::Value>>, name: &str) -> Option<i32> {
        props?.iter()
            .find(|p| p["name"].as_str() == Some(name))
            .and_then(|p| p["value"].as_i64().map(|i| i as i32))
    }

    /// Get a bool property from Tiled properties array
    fn get_property_bool(&self, props: Option<&Vec<serde_json::Value>>, name: &str) -> Option<bool> {
        props?.iter()
            .find(|p| p["name"].as_str() == Some(name))
            .and_then(|p| p["value"].as_bool())
    }

    /// Check if a world position is walkable
    pub async fn is_tile_walkable(&self, world_x: i32, world_y: i32) -> bool {
        let coord = ChunkCoord::from_world(world_x, world_y);

        if let Some(chunk) = self.get_or_load_chunk(coord).await {
            let (local_x, local_y) = world_to_local(world_x, world_y);
            chunk.is_walkable_local(local_x, local_y)
        } else {
            false // Non-existent chunks are impassable
        }
    }

    /// Check if there's a clear line of sight between two points (Bresenham's line)
    /// Returns true if no solid tiles block the path
    pub async fn has_line_of_sight(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            // Don't check start position (attacker's tile)
            if (x != x0 || y != y0) && !self.is_tile_walkable(x, y).await {
                return false;
            }

            if x == x1 && y == y1 {
                return true;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                if x == x1 { return true; }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == y1 { return true; }
                err += dx;
                y += sy;
            }
        }
    }

    /// Get chunks in a radius around a center coordinate
    pub fn get_chunks_in_radius(&self, center: ChunkCoord, radius: i32) -> Vec<ChunkCoord> {
        center.in_radius(radius)
    }

    /// Preload chunks around a position
    pub async fn preload_chunks(&self, center: ChunkCoord, radius: i32) {
        for coord in center.in_radius(radius) {
            let _ = self.get_or_load_chunk(coord).await;
        }
    }

    /// Get a safe spawn position in the world (prefers chunk 0,0)
    pub async fn get_spawn_position(&self) -> (i32, i32) {
        // Try chunk (0, 0) first
        if let Some(chunk) = self.get_or_load_chunk(ChunkCoord::new(0, 0)).await {
            if let Some((local_x, local_y)) = chunk.get_safe_spawn() {
                return (local_x as i32, local_y as i32);
            }
        }

        // Default to center of chunk 0,0
        (16, 16)
    }

    /// Check if a chunk exists (either loaded or loadable)
    pub async fn chunk_exists(&self, coord: ChunkCoord) -> bool {
        // Check cache
        {
            let chunks = self.chunks.read().await;
            if chunks.contains_key(&coord) {
                return true;
            }
        }

        // Check file
        let filename = format!("chunk_{}_{}.json", coord.x, coord.y);
        let path = Path::new(&self.chunk_dir).join(&filename);

        if path.exists() {
            return true;
        }

        // If generating missing chunks, always return true
        self.generate_missing
    }

    /// Get chunk data for sending to client
    pub async fn get_chunk_data(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        self.get_or_load_chunk(coord).await
    }

    /// Get number of loaded chunks
    pub async fn loaded_chunk_count(&self) -> usize {
        self.chunks.read().await.len()
    }

    /// Unload chunks outside of active area (for memory management)
    pub async fn unload_distant_chunks(&self, active_coords: &[ChunkCoord], keep_radius: i32) {
        let mut chunks = self.chunks.write().await;
        chunks.retain(|coord, _| {
            active_coords.iter().any(|active| {
                (coord.x - active.x).abs() <= keep_radius
                    && (coord.y - active.y).abs() <= keep_radius
            })
        });
    }

    /// Discover all chunk files in the chunk directory and return their coordinates
    pub fn discover_chunk_coords(&self) -> Vec<ChunkCoord> {
        let mut coords = Vec::new();
        let path = Path::new(&self.chunk_dir);

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let filename = entry.file_name();
                let filename_str = filename.to_string_lossy();

                // Parse chunk_X_Y.json format
                if let Some(name) = filename_str.strip_prefix("chunk_") {
                    if let Some(name) = name.strip_suffix(".json") {
                        let parts: Vec<&str> = name.split('_').collect();
                        if parts.len() == 2 {
                            if let (Ok(x), Ok(y)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                                coords.push(ChunkCoord::new(x, y));
                            }
                        }
                    }
                }
            }
        }

        coords
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_world_generates_missing_chunks() {
        let world = World::new("nonexistent_dir");
        let chunk = world.get_or_load_chunk(ChunkCoord::new(0, 0)).await;
        assert!(chunk.is_some());
    }

    #[tokio::test]
    async fn test_world_walkability() {
        let world = World::new("nonexistent_dir");

        // Load chunk to ensure it exists
        let _ = world.get_or_load_chunk(ChunkCoord::new(0, 0)).await;

        // Center should be walkable
        assert!(world.is_tile_walkable(16, 16).await);
    }

    #[tokio::test]
    async fn test_preload_chunks() {
        let world = World::new("nonexistent_dir");
        world.preload_chunks(ChunkCoord::new(0, 0), 1).await;

        // Should have 9 chunks loaded (3x3)
        assert_eq!(world.loaded_chunk_count().await, 9);
    }
}
