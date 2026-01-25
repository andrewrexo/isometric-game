use std::collections::HashMap;

pub const CHUNK_SIZE: u32 = 32;

/// Chunk coordinates in the world grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Convert world position to chunk coordinate
    pub fn from_world(world_x: i32, world_y: i32) -> Self {
        Self {
            x: world_x.div_euclid(CHUNK_SIZE as i32),
            y: world_y.div_euclid(CHUNK_SIZE as i32),
        }
    }

    /// Convert float world position to chunk coordinate
    pub fn from_world_f32(world_x: f32, world_y: f32) -> Self {
        Self::from_world(world_x.floor() as i32, world_y.floor() as i32)
    }
}

/// Convert world position to local chunk position
pub fn world_to_local(world_x: i32, world_y: i32) -> (u32, u32) {
    (
        world_x.rem_euclid(CHUNK_SIZE as i32) as u32,
        world_y.rem_euclid(CHUNK_SIZE as i32) as u32,
    )
}

/// Map object placed from Tiled's object layer (trees, rocks, decorations)
#[derive(Debug, Clone)]
pub struct MapObject {
    /// Global tile ID from objects.tsx tileset
    pub gid: u32,
    /// World tile X coordinate
    pub tile_x: i32,
    /// World tile Y coordinate
    pub tile_y: i32,
    /// Sprite width in pixels
    pub width: u32,
    /// Sprite height in pixels
    pub height: u32,
}

/// Wall edge direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallEdge {
    Down,
    Right,
}

/// Wall placed on a tile edge
#[derive(Debug, Clone)]
pub struct Wall {
    /// Global tile ID for wall sprite
    pub gid: u32,
    /// World tile X coordinate
    pub tile_x: i32,
    /// World tile Y coordinate
    pub tile_y: i32,
    /// Which edge of the tile this wall is on
    pub edge: WallEdge,
}

/// A portal that teleports players to another map
#[derive(Debug, Clone)]
pub struct Portal {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub target_map: String,
    pub target_spawn: String,
}

/// Layer types matching server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkLayerType {
    Ground = 0,
    Objects = 1,
    Overhead = 2,
}

impl ChunkLayerType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => ChunkLayerType::Ground,
            1 => ChunkLayerType::Objects,
            2 => ChunkLayerType::Overhead,
            _ => ChunkLayerType::Ground,
        }
    }
}

/// A layer of tiles within a chunk
#[derive(Debug, Clone)]
pub struct ChunkLayer {
    pub layer_type: ChunkLayerType,
    pub tiles: Vec<u32>,
}

impl ChunkLayer {
    pub fn new(layer_type: ChunkLayerType) -> Self {
        Self {
            layer_type,
            tiles: vec![0; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }

    pub fn get_tile(&self, local_x: u32, local_y: u32) -> u32 {
        if local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return 0;
        }
        let idx = (local_y * CHUNK_SIZE + local_x) as usize;
        self.tiles.get(idx).copied().unwrap_or(0)
    }
}

/// Client-side chunk data
#[derive(Debug, Clone)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub layers: Vec<ChunkLayer>,
    pub collision: Vec<bool>,
    /// Map objects (trees, rocks, decorations) from object layer
    pub objects: Vec<MapObject>,
    /// Walls placed on tile edges
    pub walls: Vec<Wall>,
    /// Portals that teleport players to other maps
    pub portals: Vec<Portal>,
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            layers: vec![
                ChunkLayer::new(ChunkLayerType::Ground),
                ChunkLayer::new(ChunkLayerType::Objects),
                ChunkLayer::new(ChunkLayerType::Overhead),
            ],
            collision: vec![false; (CHUNK_SIZE * CHUNK_SIZE) as usize],
            objects: Vec::new(),
            walls: Vec::new(),
            portals: Vec::new(),
        }
    }

    /// Get tile from a specific layer
    pub fn get_tile(&self, layer_type: ChunkLayerType, local_x: u32, local_y: u32) -> u32 {
        for layer in &self.layers {
            if layer.layer_type == layer_type {
                return layer.get_tile(local_x, local_y);
            }
        }
        0
    }

    /// Check if a local position is walkable
    pub fn is_walkable_local(&self, local_x: u32, local_y: u32) -> bool {
        if local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return false;
        }
        let idx = (local_y * CHUNK_SIZE + local_x) as usize;
        !self.collision.get(idx).copied().unwrap_or(true)
    }

    /// Unpack collision data from server bytes
    pub fn unpack_collision(packed: &[u8]) -> Vec<bool> {
        let mut collision = vec![false; (CHUNK_SIZE * CHUNK_SIZE) as usize];
        for (i, blocked) in collision.iter_mut().enumerate() {
            if i / 8 < packed.len() {
                *blocked = (packed[i / 8] >> (i % 8)) & 1 == 1;
            }
        }
        collision
    }
}

/// Chunk manager handles loading and caching chunks
pub struct ChunkManager {
    chunks: HashMap<ChunkCoord, Chunk>,
    /// Chunks that we've requested but haven't received yet
    pending_requests: HashMap<ChunkCoord, f64>,
    /// Current player chunk for tracking movement
    current_chunk: ChunkCoord,
    /// View radius in chunks
    view_radius: i32,
}

impl ChunkManager {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            pending_requests: HashMap::new(),
            current_chunk: ChunkCoord::new(0, 0),
            view_radius: 2, // Load 5x5 chunks around player
        }
    }

    /// Update player position and return list of chunks to request
    pub fn update_player_position(&mut self, world_x: f32, world_y: f32) -> Vec<ChunkCoord> {
        let new_chunk = ChunkCoord::from_world_f32(world_x, world_y);
        self.current_chunk = new_chunk;

        // Check which chunks need to be loaded
        let mut to_request = Vec::new();
        let current_time = macroquad::time::get_time();

        for dy in -self.view_radius..=self.view_radius {
            for dx in -self.view_radius..=self.view_radius {
                let coord = ChunkCoord::new(new_chunk.x + dx, new_chunk.y + dy);

                // Skip if already loaded
                if self.chunks.contains_key(&coord) {
                    continue;
                }

                // Skip if recently requested (wait 2 seconds before re-requesting)
                if let Some(&request_time) = self.pending_requests.get(&coord) {
                    if current_time - request_time < 2.0 {
                        continue;
                    }
                }

                // Request this chunk
                self.pending_requests.insert(coord, current_time);
                to_request.push(coord);
            }
        }

        to_request
    }

    /// Load chunk data received from server
    pub fn load_chunk(
        &mut self,
        chunk_x: i32,
        chunk_y: i32,
        layers: Vec<(u8, Vec<u32>)>,
        collision: &[u8],
        objects: Vec<MapObject>,
        walls: Vec<Wall>,
        portals: Vec<Portal>,
    ) {
        let coord = ChunkCoord::new(chunk_x, chunk_y);
        let mut chunk = Chunk::new(coord);

        // Load layers
        for (layer_type, tiles) in layers {
            let lt = ChunkLayerType::from_u8(layer_type);
            for layer in &mut chunk.layers {
                if layer.layer_type == lt {
                    layer.tiles = tiles.clone();
                    break;
                }
            }
        }

        // Load collision
        chunk.collision = Chunk::unpack_collision(collision);

        // Load map objects
        chunk.objects = objects;

        // Load walls
        chunk.walls = walls;

        // Load portals
        chunk.portals = portals;

        // Remove from pending
        self.pending_requests.remove(&coord);

        let object_count = chunk.objects.len();
        let portal_count = chunk.portals.len();

        // Store chunk
        self.chunks.insert(coord, chunk);

        log::info!("Loaded chunk ({}, {}) with {} objects, {} portals", chunk_x, chunk_y, object_count, portal_count);
    }

    /// Check if a world position is walkable
    pub fn is_walkable(&self, world_x: f32, world_y: f32) -> bool {
        let x = world_x.floor() as i32;
        let y = world_y.floor() as i32;
        let coord = ChunkCoord::from_world(x, y);

        if let Some(chunk) = self.chunks.get(&coord) {
            let (local_x, local_y) = world_to_local(x, y);
            chunk.is_walkable_local(local_x, local_y)
        } else {
            false // Unloaded chunks are impassable
        }
    }

    /// Get tile at world position
    pub fn get_tile(&self, layer_type: ChunkLayerType, world_x: i32, world_y: i32) -> u32 {
        let coord = ChunkCoord::from_world(world_x, world_y);

        if let Some(chunk) = self.chunks.get(&coord) {
            let (local_x, local_y) = world_to_local(world_x, world_y);
            chunk.get_tile(layer_type, local_x, local_y)
        } else {
            0
        }
    }

    /// Get all loaded chunks for rendering
    pub fn chunks(&self) -> &HashMap<ChunkCoord, Chunk> {
        &self.chunks
    }

    /// Check if a chunk is loaded
    pub fn is_chunk_loaded(&self, coord: ChunkCoord) -> bool {
        self.chunks.contains_key(&coord)
    }

    /// Get number of loaded chunks
    pub fn loaded_count(&self) -> usize {
        self.chunks.len()
    }

    /// Unload chunks far from current position
    pub fn unload_distant_chunks(&mut self) {
        let keep_radius = self.view_radius + 1;
        let current = self.current_chunk;

        self.chunks.retain(|coord, _| {
            (coord.x - current.x).abs() <= keep_radius
                && (coord.y - current.y).abs() <= keep_radius
        });
    }
}
