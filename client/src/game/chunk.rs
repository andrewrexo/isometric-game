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

    /// Unpack collision data for interiors with custom size
    pub fn unpack_collision_sized(packed: &[u8], tile_count: usize) -> Vec<bool> {
        let mut collision = vec![false; tile_count];
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
    /// Interior mode - if Some, we're in an interior with (width, height)
    interior_size: Option<(u32, u32)>,
}

impl ChunkManager {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            pending_requests: HashMap::new(),
            current_chunk: ChunkCoord::new(0, 0),
            view_radius: 2, // Load 5x5 chunks around player
            interior_size: None,
        }
    }

    /// Check if we're in an interior and get its size
    pub fn get_interior_size(&self) -> Option<(u32, u32)> {
        self.interior_size
    }

    /// Check if we're in an interior
    pub fn is_interior(&self) -> bool {
        self.interior_size.is_some()
    }

    /// Update player position and return list of chunks to request
    pub fn update_player_position(&mut self, world_x: f32, world_y: f32) -> Vec<ChunkCoord> {
        // Don't request world chunks when in interior mode
        if self.interior_size.is_some() {
            return Vec::new();
        }

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
        // Don't load world chunks when in interior mode
        if self.interior_size.is_some() {
            log::debug!("Ignoring world chunk ({}, {}) while in interior mode", chunk_x, chunk_y);
            return;
        }

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

        // For interiors, use interior dimensions for collision checking
        if let Some((width, height)) = self.interior_size {
            // In interior mode, chunk is at (0,0) and coords are direct
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                log::info!("Interior bounds BLOCKED at ({}, {}): outside {}x{}", x, y, width, height);
                return false; // Out of bounds
            }
            let coord = ChunkCoord::new(0, 0);
            if let Some(chunk) = self.chunks.get(&coord) {
                let idx = (y as u32 * width + x as u32) as usize;
                let blocked = chunk.collision.get(idx).copied().unwrap_or(true);
                if blocked {
                    log::info!("Interior collision BLOCKED at ({}, {}): idx={}", x, y, idx);
                }
                return !blocked;
            }
            log::info!("Interior chunk NOT FOUND at (0,0) - {} chunks loaded", self.chunks.len());
            return false;
        }

        // Standard chunk-based collision - if we get here, we're NOT in interior mode
        let coord = ChunkCoord::from_world(x, y);
        if let Some(chunk) = self.chunks.get(&coord) {
            let (local_x, local_y) = world_to_local(x, y);
            let walkable = chunk.is_walkable_local(local_x, local_y);
            if !walkable {
                log::info!("World collision BLOCKED at ({}, {}) chunk ({}, {}) local ({}, {})",
                    x, y, coord.x, coord.y, local_x, local_y);
            }
            walkable
        } else {
            log::info!("World chunk NOT LOADED at ({}, {}) -> chunk ({}, {}), interior_size={:?}",
                x, y, coord.x, coord.y, self.interior_size);
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

    /// Find a portal at the given world position
    pub fn get_portal_at(&self, x: f32, y: f32) -> Option<&Portal> {
        let coord = ChunkCoord::from_world_f32(x, y);
        log::info!("get_portal_at: world ({}, {}) -> chunk ({}, {})", x, y, coord.x, coord.y);

        let chunk = match self.chunks.get(&coord) {
            Some(c) => c,
            None => {
                log::info!("get_portal_at: chunk not loaded");
                return None;
            }
        };

        let tile_x = x.floor() as i32;
        let tile_y = y.floor() as i32;

        // Portal x/y are LOCAL coords (0-31), need to convert to WORLD coords
        let chunk_base_x = coord.x * CHUNK_SIZE as i32;
        let chunk_base_y = coord.y * CHUNK_SIZE as i32;

        log::info!("get_portal_at: chunk has {} portals, tile ({}, {}), chunk_base ({}, {})",
            chunk.portals.len(), tile_x, tile_y, chunk_base_x, chunk_base_y);

        for p in &chunk.portals {
            let world_px = chunk_base_x + p.x;
            let world_py = chunk_base_y + p.y;
            log::info!("  portal '{}': local ({}, {}) -> world ({}, {}) to ({}, {})",
                p.id, p.x, p.y, world_px, world_py, world_px + p.width, world_py + p.height);
        }

        chunk.portals.iter().find(|p| {
            let world_px = chunk_base_x + p.x;
            let world_py = chunk_base_y + p.y;
            tile_x >= world_px && tile_x < world_px + p.width &&
            tile_y >= world_py && tile_y < world_py + p.height
        })
    }

    /// Load an interior as a single chunk at (0,0)
    pub fn load_interior(&mut self, width: u32, height: u32, layers: Vec<(u8, Vec<u32>)>, collision: &[u8], portals: Vec<Portal>, objects: Vec<MapObject>, walls: Vec<Wall>) {
        // Clear existing chunks
        self.chunks.clear();
        self.pending_requests.clear();

        // Set interior mode with dimensions
        self.interior_size = Some((width, height));

        // Create interior chunk at (0,0)
        let coord = ChunkCoord { x: 0, y: 0 };

        let chunk_layers: Vec<ChunkLayer> = layers.into_iter().map(|(layer_type, tiles)| {
            ChunkLayer {
                layer_type: ChunkLayerType::from_u8(layer_type),
                tiles,
            }
        }).collect();

        let collision_data = Chunk::unpack_collision_sized(collision, (width * height) as usize);
        let blocked_count = collision_data.iter().filter(|&&b| b).count();
        log::info!("Interior collision: {} bytes packed, {} tiles total, {} blocked",
            collision.len(), collision_data.len(), blocked_count);

        let chunk = Chunk {
            coord,
            layers: chunk_layers,
            collision: collision_data,
            objects,
            walls,
            portals,
        };

        let portal_count = chunk.portals.len();
        let object_count = chunk.objects.len();
        let wall_count = chunk.walls.len();
        self.chunks.insert(coord, chunk);
        self.current_chunk = coord;

        log::info!("Loaded interior map: {}x{} with {} portals, {} objects, {} walls", width, height, portal_count, object_count, wall_count);
    }

    /// Clear interior and prepare for overworld
    pub fn clear_interior(&mut self) {
        self.chunks.clear();
        self.pending_requests.clear();
        self.interior_size = None;
    }
}
