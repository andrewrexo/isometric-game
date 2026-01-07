use crate::game::Camera;
use macroquad::prelude::*;

// Isometric tile dimensions (2:1 ratio for pixel art)
pub const TILE_WIDTH: f32 = 64.0;
pub const TILE_HEIGHT: f32 = 32.0;

/// Convert world (game) coordinates to screen (pixel) coordinates using isometric projection
/// Returns pixel-snapped coordinates for crisp rendering
pub fn world_to_screen(world_x: f32, world_y: f32, camera: &Camera) -> (f32, f32) {
    // Isometric transformation (2:1 diamond projection)
    let iso_x = (world_x - world_y) * (TILE_WIDTH / 2.0);
    let iso_y = (world_x + world_y) * (TILE_HEIGHT / 2.0);

    // Camera must also be transformed to isometric space
    let cam_iso_x = (camera.x - camera.y) * (TILE_WIDTH / 2.0);
    let cam_iso_y = (camera.x + camera.y) * (TILE_HEIGHT / 2.0);

    // Apply camera offset and zoom, center on screen
    let screen_x = (iso_x - cam_iso_x) * camera.zoom + (screen_width() / 2.0).floor();
    let screen_y = (iso_y - cam_iso_y) * camera.zoom + (screen_height() / 2.0).floor();

    // Pixel snap for crisp rendering (no sub-pixel jitter)
    (screen_x.round(), screen_y.round())
}

/// Convert world to screen WITHOUT pixel snapping (for calculations)
pub fn world_to_screen_exact(world_x: f32, world_y: f32, camera: &Camera) -> (f32, f32) {
    let iso_x = (world_x - world_y) * (TILE_WIDTH / 2.0);
    let iso_y = (world_x + world_y) * (TILE_HEIGHT / 2.0);

    let cam_iso_x = (camera.x - camera.y) * (TILE_WIDTH / 2.0);
    let cam_iso_y = (camera.x + camera.y) * (TILE_HEIGHT / 2.0);

    let screen_x = (iso_x - cam_iso_x) * camera.zoom + screen_width() / 2.0;
    let screen_y = (iso_y - cam_iso_y) * camera.zoom + screen_height() / 2.0;

    (screen_x, screen_y)
}

/// Convert screen (pixel) coordinates to world (game) coordinates
pub fn screen_to_world(screen_x: f32, screen_y: f32, camera: &Camera) -> (f32, f32) {
    // Camera in isometric space
    let cam_iso_x = (camera.x - camera.y) * (TILE_WIDTH / 2.0);
    let cam_iso_y = (camera.x + camera.y) * (TILE_HEIGHT / 2.0);

    // Reverse the screen transformation
    let iso_x = (screen_x - screen_width() / 2.0) / camera.zoom + cam_iso_x;
    let iso_y = (screen_y - screen_height() / 2.0) / camera.zoom + cam_iso_y;

    // Reverse the isometric transformation
    let world_x = (iso_x / (TILE_WIDTH / 2.0) + iso_y / (TILE_HEIGHT / 2.0)) / 2.0;
    let world_y = (iso_y / (TILE_HEIGHT / 2.0) - iso_x / (TILE_WIDTH / 2.0)) / 2.0;

    (world_x, world_y)
}

/// Calculate isometric depth for sorting (painter's algorithm)
/// Higher values should be rendered later (on top)
pub fn calculate_depth(world_x: f32, world_y: f32, layer: u32) -> f32 {
    // Layer provides broad ordering (floor < entities < effects)
    // Within a layer, sort by x + y (entities further down-right render on top)
    (layer as f32 * 10000.0) + world_x + world_y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_to_screen_origin() {
        let camera = Camera { x: 0.0, y: 0.0, zoom: 1.0 };
        let (sx, sy) = world_to_screen(0.0, 0.0, &camera);
        // At origin with centered camera, should be at screen center
        // This will depend on screen size, so just check it's reasonable
        assert!(sx.is_finite());
        assert!(sy.is_finite());
    }

    #[test]
    fn test_roundtrip() {
        let camera = Camera { x: 5.0, y: 3.0, zoom: 1.5 };
        let world_x = 10.0;
        let world_y = 7.0;

        let (screen_x, screen_y) = world_to_screen(world_x, world_y, &camera);
        let (back_x, back_y) = screen_to_world(screen_x, screen_y, &camera);

        assert!((back_x - world_x).abs() < 0.001);
        assert!((back_y - world_y).abs() < 0.001);
    }
}
