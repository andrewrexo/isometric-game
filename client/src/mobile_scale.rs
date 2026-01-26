//! Mobile resolution scaling
//!
//! On high-DPI mobile devices, uses camera zoom to scale virtual coordinates.
//! This is more performant than render targets on mobile GPUs.

use macroquad::prelude::*;

/// Target virtual width - height is calculated to match screen aspect ratio
pub const VIRTUAL_WIDTH: f32 = 640.0;
/// This will be recalculated based on screen aspect ratio
pub const VIRTUAL_HEIGHT: f32 = 360.0;

/// Mobile scaling state - uses camera zoom instead of render targets for performance
pub struct MobileScaler {
    virtual_width: f32,
    virtual_height: f32,
    scale: f32,
    enabled: bool,
}

impl MobileScaler {
    pub fn new() -> Self {
        #[cfg(target_os = "android")]
        let enabled = true;
        #[cfg(not(target_os = "android"))]
        let enabled = false;

        // Calculate virtual height to match screen aspect ratio (no letterboxing)
        let screen_w = screen_width();
        let screen_h = screen_height();
        let aspect = screen_h / screen_w;
        let virtual_width = VIRTUAL_WIDTH;
        let virtual_height = (VIRTUAL_WIDTH * aspect).round();

        // Calculate scale factor (how much to multiply virtual coords to get screen coords)
        let scale = screen_w / virtual_width;

        Self {
            virtual_width,
            virtual_height,
            scale,
            enabled,
        }
    }

    /// Call before rendering frame content
    pub fn begin_frame(&self) {
        if self.enabled {
            // Set up camera that maps virtual coordinates to screen coordinates
            // from_display_rect expects Y to go up, but we want Y to go down (screen coords)
            // So we flip by using negative height and offset
            let camera = Camera2D::from_display_rect(Rect::new(
                0.0,
                self.virtual_height,
                self.virtual_width,
                -self.virtual_height,
            ));
            set_camera(&camera);
        }
    }

    /// Call after rendering frame content
    pub fn end_frame(&self) {
        if self.enabled {
            set_default_camera();
        }
    }

    /// Get the virtual screen dimensions (use these for layout on mobile)
    pub fn virtual_size(&self) -> (f32, f32) {
        if self.enabled {
            (self.virtual_width, self.virtual_height)
        } else {
            (screen_width(), screen_height())
        }
    }

    /// Convert screen touch coordinates to virtual coordinates
    pub fn screen_to_virtual(&self, x: f32, y: f32) -> (f32, f32) {
        if !self.enabled {
            return (x, y);
        }

        // Scale down from screen coords to virtual coords
        let vx = x / self.scale;
        let vy = y / self.scale;

        (vx, vy)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }
}
