use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: u32 = 32;

/// Entity spawn point data parsed from Tiled map
#[derive(Debug, Clone)]
pub struct EntitySpawn {
    /// Prototype ID from entity registry (e.g., "pig", "elder_villager")
    pub entity_id: String,
    /// World X coordinate
    pub world_x: i32,
    /// World Y coordinate
    pub world_y: i32,
    /// Entity level
    pub level: i32,
    /// Whether entity respawns after death
    pub respawn: bool,
    /// Optional respawn time override (ms)
    pub respawn_time_override: Option<u64>,
    /// Initial facing direction
    pub facing: Option<String>,
    /// Optional unique instance ID (for quest targets)
    pub unique_id: Option<String>,
}

/// Map object placed from Tiled's object layer (trees, rocks, decorations)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallEdge {
    Down,
    Right,
}

/// Wall placed on a tile edge
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Portal {
    #[serde(default)]
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub target_map: String,
    pub target_spawn: String,
}

/// Chunk coordinates in the world grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    /// Get the 8 neighboring chunk coordinates
    pub fn neighbors(&self) -> [ChunkCoord; 8] {
        [
            ChunkCoord::new(self.x - 1, self.y - 1),
            ChunkCoord::new(self.x, self.y - 1),
            ChunkCoord::new(self.x + 1, self.y - 1),
            ChunkCoord::new(self.x - 1, self.y),
            ChunkCoord::new(self.x + 1, self.y),
            ChunkCoord::new(self.x - 1, self.y + 1),
            ChunkCoord::new(self.x, self.y + 1),
            ChunkCoord::new(self.x + 1, self.y + 1),
        ]
    }

    /// Get all chunk coordinates in a square radius (including center)
    pub fn in_radius(&self, radius: i32) -> Vec<ChunkCoord> {
        let mut coords = Vec::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                coords.push(ChunkCoord::new(self.x + dx, self.y + dy));
            }
        }
        coords
    }
}

/// Convert world position to local chunk position
pub fn world_to_local(world_x: i32, world_y: i32) -> (u32, u32) {
    (
        world_x.rem_euclid(CHUNK_SIZE as i32) as u32,
        world_y.rem_euclid(CHUNK_SIZE as i32) as u32,
    )
}

/// Convert chunk + local position to world position
pub fn local_to_world(chunk: ChunkCoord, local_x: u32, local_y: u32) -> (i32, i32) {
    (
        chunk.x * CHUNK_SIZE as i32 + local_x as i32,
        chunk.y * CHUNK_SIZE as i32 + local_y as i32,
    )
}

/// Layer types for chunk tile data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkLayerType {
    Ground = 0,
    Objects = 1,
    Overhead = 2,
}

/// A layer of tiles within a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkLayer {
    pub layer_type: ChunkLayerType,
    pub tiles: Vec<u32>, // CHUNK_SIZE * CHUNK_SIZE tile IDs
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

    pub fn set_tile(&mut self, local_x: u32, local_y: u32, tile_id: u32) {
        if local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return;
        }
        let idx = (local_y * CHUNK_SIZE + local_x) as usize;
        if idx < self.tiles.len() {
            self.tiles[idx] = tile_id;
        }
    }
}

/// Server-side chunk containing collision and tile data
#[derive(Debug, Clone)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub collision: Vec<bool>, // CHUNK_SIZE * CHUNK_SIZE collision flags
    pub layers: Vec<ChunkLayer>,
    /// Entity spawn points parsed from Tiled map objects
    pub entity_spawns: Vec<EntitySpawn>,
    /// Simple spawn points for players (local coordinates)
    pub player_spawns: Vec<(u32, u32)>,
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
            collision: vec![false; (CHUNK_SIZE * CHUNK_SIZE) as usize],
            layers: vec![
                ChunkLayer::new(ChunkLayerType::Ground),
                ChunkLayer::new(ChunkLayerType::Objects),
                ChunkLayer::new(ChunkLayerType::Overhead),
            ],
            entity_spawns: Vec::new(),
            player_spawns: Vec::new(),
            objects: Vec::new(),
            walls: Vec::new(),
            portals: Vec::new(),
        }
    }

    /// Create a test chunk with procedural content (for development)
    pub fn new_test(coord: ChunkCoord) -> Self {
        let mut chunk = Self::new(coord);

        // Fill with grass and some variation
        let ground = &mut chunk.layers[0];
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let idx = (y * CHUNK_SIZE + x) as usize;
                // Base grass with some dirt patches
                let world_x = coord.x * CHUNK_SIZE as i32 + x as i32;
                let world_y = coord.y * CHUNK_SIZE as i32 + y as i32;

                if (world_x + world_y * 7) % 23 == 0 {
                    ground.tiles[idx] = 2; // Dirt
                } else {
                    ground.tiles[idx] = 1; // Grass
                }
            }
        }

        // Add some rocks as obstacles
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let idx = (y * CHUNK_SIZE + x) as usize;
                let world_x = coord.x * CHUNK_SIZE as i32 + x as i32;
                let world_y = coord.y * CHUNK_SIZE as i32 + y as i32;

                // Rocks using a different pattern than edges
                if (world_x + world_y * 3) % 17 == 0 && x > 2 && y > 2 && x < CHUNK_SIZE - 3 && y < CHUNK_SIZE - 3 {
                    chunk.layers[0].tiles[idx] = 4; // Rock tile
                    chunk.collision[idx] = true;
                }
            }
        }

        // Add player spawn point at center of chunk
        chunk.player_spawns.push((CHUNK_SIZE / 2, CHUNK_SIZE / 2));

        // Add some test entity spawns for chunk (0, 0)
        if coord.x == 0 && coord.y == 0 {
            let base_x = coord.x * CHUNK_SIZE as i32;
            let base_y = coord.y * CHUNK_SIZE as i32;

            chunk.entity_spawns.push(EntitySpawn {
                entity_id: "pig".to_string(),
                world_x: base_x + 19,
                world_y: base_y + 9,
                level: 1,
                respawn: true,
                respawn_time_override: None,
                facing: None,
                unique_id: None,
            });
            chunk.entity_spawns.push(EntitySpawn {
                entity_id: "pig".to_string(),
                world_x: base_x + 19,
                world_y: base_y + 8,
                level: 1,
                respawn: true,
                respawn_time_override: None,
                facing: None,
                unique_id: None,
            });
            chunk.entity_spawns.push(EntitySpawn {
                entity_id: "pig".to_string(),
                world_x: base_x + 8,
                world_y: base_y + 12,
                level: 1,
                respawn: true,
                respawn_time_override: None,
                facing: None,
                unique_id: None,
            });
            chunk.entity_spawns.push(EntitySpawn {
                entity_id: "pig".to_string(),
                world_x: base_x + 20,
                world_y: base_y + 15,
                level: 2,
                respawn: true,
                respawn_time_override: None,
                facing: None,
                unique_id: None,
            });
            chunk.entity_spawns.push(EntitySpawn {
                entity_id: "pig".to_string(),
                world_x: base_x + 15,
                world_y: base_y + 20,
                level: 3,
                respawn: true,
                respawn_time_override: Some(300000), // 5 minutes
                facing: None,
                unique_id: Some("piggy_boss".to_string()),
            });
        }

        chunk
    }

    /// Check if a local position is walkable
    pub fn is_walkable_local(&self, local_x: u32, local_y: u32) -> bool {
        if local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return false;
        }
        let idx = (local_y * CHUNK_SIZE + local_x) as usize;
        !self.collision.get(idx).copied().unwrap_or(true)
    }

    /// Set collision at local position
    pub fn set_collision(&mut self, local_x: u32, local_y: u32, blocked: bool) {
        if local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return;
        }
        let idx = (local_y * CHUNK_SIZE + local_x) as usize;
        if idx < self.collision.len() {
            self.collision[idx] = blocked;
        }
    }

    /// Get a safe spawn point in this chunk
    pub fn get_safe_spawn(&self) -> Option<(u32, u32)> {
        // First try defined player spawn points
        for &(x, y) in &self.player_spawns {
            if self.is_walkable_local(x, y) {
                return Some((x, y));
            }
        }

        // Fall back to center, spiral outward
        let center_x = CHUNK_SIZE / 2;
        let center_y = CHUNK_SIZE / 2;

        for radius in 0..16 {
            for dy in -(radius as i32)..=(radius as i32) {
                for dx in -(radius as i32)..=(radius as i32) {
                    if dx.abs() == radius as i32 || dy.abs() == radius as i32 {
                        let x = (center_x as i32 + dx) as u32;
                        let y = (center_y as i32 + dy) as u32;
                        if x < CHUNK_SIZE && y < CHUNK_SIZE && self.is_walkable_local(x, y) {
                            return Some((x, y));
                        }
                    }
                }
            }
        }

        None
    }

    /// Pack collision data into bytes (for network transmission)
    pub fn pack_collision(&self) -> Vec<u8> {
        let mut packed = vec![0u8; (CHUNK_SIZE * CHUNK_SIZE / 8) as usize];
        for (i, &blocked) in self.collision.iter().enumerate() {
            if blocked {
                packed[i / 8] |= 1 << (i % 8);
            }
        }
        packed
    }

    /// Unpack collision data from bytes
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_coord_from_world() {
        assert_eq!(ChunkCoord::from_world(0, 0), ChunkCoord::new(0, 0));
        assert_eq!(ChunkCoord::from_world(31, 31), ChunkCoord::new(0, 0));
        assert_eq!(ChunkCoord::from_world(32, 0), ChunkCoord::new(1, 0));
        assert_eq!(ChunkCoord::from_world(-1, 0), ChunkCoord::new(-1, 0));
        assert_eq!(ChunkCoord::from_world(-32, -32), ChunkCoord::new(-1, -1));
        assert_eq!(ChunkCoord::from_world(-33, -33), ChunkCoord::new(-2, -2));
    }

    #[test]
    fn test_world_to_local() {
        assert_eq!(world_to_local(0, 0), (0, 0));
        assert_eq!(world_to_local(31, 31), (31, 31));
        assert_eq!(world_to_local(32, 32), (0, 0));
        assert_eq!(world_to_local(33, 34), (1, 2));
        assert_eq!(world_to_local(-1, -1), (31, 31));
        assert_eq!(world_to_local(-32, -32), (0, 0));
    }

    #[test]
    fn test_local_to_world() {
        assert_eq!(local_to_world(ChunkCoord::new(0, 0), 5, 10), (5, 10));
        assert_eq!(local_to_world(ChunkCoord::new(1, 0), 5, 10), (37, 10));
        assert_eq!(local_to_world(ChunkCoord::new(-1, -1), 0, 0), (-32, -32));
    }

    #[test]
    fn test_collision_packing() {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0));
        chunk.set_collision(0, 0, true);
        chunk.set_collision(1, 0, true);
        chunk.set_collision(8, 0, true);

        let packed = chunk.pack_collision();
        let unpacked = Chunk::unpack_collision(&packed);

        assert!(unpacked[0]); // (0, 0)
        assert!(unpacked[1]); // (1, 0)
        assert!(!unpacked[2]); // (2, 0)
        assert!(unpacked[8]); // (8, 0)
    }
}
