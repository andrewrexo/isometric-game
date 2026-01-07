use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::chunk::{world_to_local, Chunk, ChunkCoord, ChunkLayer, ChunkLayerType, CHUNK_SIZE};

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

    /// Load chunk from Tiled JSON file
    async fn load_chunk_from_file(&self, coord: ChunkCoord) -> Option<Chunk> {
        let filename = format!("chunk_{}_{}.json", coord.x, coord.y);
        let path = Path::new(&self.chunk_dir).join(&filename);

        if !path.exists() {
            return None;
        }

        match tokio::fs::read_to_string(&path).await {
            Ok(json) => match self.parse_tiled_json(coord, &json) {
                Ok(chunk) => {
                    info!("Loaded chunk from {:?}", path);
                    Some(chunk)
                }
                Err(e) => {
                    warn!("Failed to parse chunk {:?}: {}", path, e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read chunk {:?}: {}", path, e);
                None
            }
        }
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
                    // Parse object groups for collision and spawn points
                    if let Some(objects) = layer_value["objects"].as_array() {
                        for obj in objects {
                            let obj_type = obj["type"].as_str().unwrap_or("");
                            let x = obj["x"].as_f64().unwrap_or(0.0) as u32 / 64; // Assuming 64px tiles
                            let y = obj["y"].as_f64().unwrap_or(0.0) as u32 / 32;
                            let obj_width = obj["width"].as_f64().unwrap_or(64.0) as u32 / 64;
                            let obj_height = obj["height"].as_f64().unwrap_or(32.0) as u32 / 32;

                            if obj_type == "collision" || name.to_lowercase().contains("collision") {
                                // Mark collision area
                                for dy in 0..obj_height.max(1) {
                                    for dx in 0..obj_width.max(1) {
                                        chunk.set_collision(x + dx, y + dy, true);
                                    }
                                }
                            } else if obj_type == "spawn" || name.to_lowercase().contains("spawn") {
                                chunk.spawn_points.push((x, y));
                            }
                        }
                    }
                }
            }
        }

        Ok(chunk)
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
