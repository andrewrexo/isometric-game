/// Server-side tilemap for collision detection
/// Must generate the same collision data as the client

/// Tilemap for collision detection
pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    collision: Vec<bool>,
}

impl Tilemap {
    /// Create a test tilemap (must match client's new_test_map)
    pub fn new_test_map(width: u32, height: u32) -> Self {
        let mut collision = vec![false; (width * height) as usize];

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;

                // Water edges (collision)
                if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    collision[idx] = true;
                }

                // Some rocks/obstacles (must match client algorithm exactly)
                if (x + y * 3) % 17 == 0 && x > 2 && y > 2 && x < width - 3 && y < height - 3 {
                    collision[idx] = true;
                }
            }
        }

        Self {
            width,
            height,
            collision,
        }
    }

    /// Check if a grid tile is walkable
    pub fn is_tile_walkable(&self, x: i32, y: i32) -> bool {
        // Check bounds
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }

        let idx = (y as u32 * self.width + x as u32) as usize;
        !self.collision.get(idx).copied().unwrap_or(true)
    }

    /// Check if a position is walkable (float version for NPCs)
    pub fn is_walkable(&self, x: f32, y: f32) -> bool {
        self.is_tile_walkable(x as i32, y as i32)
    }

    /// Check collision for an NPC with a radius (float positions)
    pub fn check_collision(&self, x: f32, y: f32, radius: f32) -> bool {
        // Check the tile at rounded position
        let tile_x = x.round() as i32;
        let tile_y = y.round() as i32;
        !self.is_tile_walkable(tile_x, tile_y)
    }

    /// Try to move from current position to new position (for NPCs with float positions)
    pub fn resolve_movement(&self, from_x: f32, from_y: f32, to_x: f32, to_y: f32, _radius: f32) -> (f32, f32) {
        // Check if target tile is walkable
        let target_tile_x = to_x.round() as i32;
        let target_tile_y = to_y.round() as i32;

        if self.is_tile_walkable(target_tile_x, target_tile_y) {
            return (to_x, to_y);
        }

        // Try moving only on X axis
        let from_tile_y = from_y.round() as i32;
        if self.is_tile_walkable(target_tile_x, from_tile_y) {
            return (to_x, from_y);
        }

        // Try moving only on Y axis
        let from_tile_x = from_x.round() as i32;
        if self.is_tile_walkable(from_tile_x, target_tile_y) {
            return (from_x, to_y);
        }

        // Can't move at all
        (from_x, from_y)
    }

    /// Get spawn position that's not colliding (returns grid coordinates)
    pub fn get_safe_spawn(&self) -> (i32, i32) {
        let center_x = self.width as i32 / 2;
        let center_y = self.height as i32 / 2;

        // Try center first
        if self.is_tile_walkable(center_x, center_y) {
            return (center_x, center_y);
        }

        // Search outward in a spiral pattern
        for radius in 1..10i32 {
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    if dx.abs() == radius || dy.abs() == radius {
                        let x = center_x + dx;
                        let y = center_y + dy;
                        if self.is_tile_walkable(x, y) {
                            return (x, y);
                        }
                    }
                }
            }
        }

        // Fallback to center
        (center_x, center_y)
    }

    /// Check if there's a clear line of sight between two points (Bresenham's line)
    /// Returns true if no solid tiles block the path
    pub fn has_line_of_sight(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            // Don't check start position (attacker's tile)
            if (x != x0 || y != y0) && !self.is_tile_walkable(x, y) {
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
}
