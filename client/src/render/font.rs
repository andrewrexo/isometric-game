//! Multi-size bitmap font for pixel-perfect text rendering
//!
//! Pre-loads a pixel font at multiple native sizes to avoid scaling artifacts.

use macroquad::prelude::*;
use std::collections::HashMap;

/// Available font sizes - these are pre-loaded at native resolution
pub const FONT_SIZES: &[u16] = &[8, 10, 12, 16, 20, 24, 32];

/// Multi-size bitmap font that provides pixel-perfect rendering
pub struct BitmapFont {
    /// Fonts keyed by their native size
    fonts: HashMap<u16, Font>,
}

impl BitmapFont {
    /// Load the font at all predefined sizes
    pub async fn load(path: &str) -> Self {
        let mut fonts = HashMap::new();

        for &size in FONT_SIZES {
            match load_ttf_font_from_bytes(
                &std::fs::read(path).unwrap_or_default()
            ) {
                Ok(mut font) => {
                    font.set_filter(FilterMode::Nearest);
                    fonts.insert(size, font);
                }
                Err(e) => {
                    eprintln!("Failed to load font at size {}: {}", size, e);
                }
            }
        }

        Self { fonts }
    }

    /// Load from a specific path, falling back gracefully
    pub async fn load_or_default(path: &str) -> Self {
        let mut fonts = HashMap::new();

        // Try to read the font file once
        let font_bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("Failed to read font file {}: {}", path, e);
                return Self { fonts };
            }
        };

        for &size in FONT_SIZES {
            match load_ttf_font_from_bytes(&font_bytes) {
                Ok(mut font) => {
                    font.set_filter(FilterMode::Nearest);
                    fonts.insert(size, font);
                }
                Err(e) => {
                    eprintln!("Failed to load font at size {}: {}", size, e);
                }
            }
        }

        Self { fonts }
    }

    /// Get the closest available font size
    fn get_closest_size(&self, requested: f32) -> u16 {
        let requested = requested.round() as u16;

        // Find exact match or closest smaller size
        FONT_SIZES
            .iter()
            .rev()
            .find(|&&s| s <= requested)
            .copied()
            .unwrap_or(FONT_SIZES[0])
    }

    /// Get font for a specific size (returns closest match)
    pub fn get_font(&self, size: f32) -> Option<&Font> {
        let closest = self.get_closest_size(size);
        self.fonts.get(&closest)
    }

    /// Draw text at the specified size (uses closest native size)
    pub fn draw_text(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        let native_size = self.get_closest_size(font_size);

        // Only apply scaling if not an exact native size match
        let scale = if (font_size - native_size as f32).abs() < 0.01 {
            1.0
        } else {
            font_size / native_size as f32
        };

        if let Some(font) = self.fonts.get(&native_size) {
            draw_text_ex(
                text,
                x.floor(),  // Pixel-perfect positioning
                y.floor(),
                TextParams {
                    font: Some(font),
                    font_size: native_size,
                    font_scale: scale,
                    color,
                    ..Default::default()
                },
            );
        } else {
            // Fallback to default font
            draw_text(text, x.floor(), y.floor(), font_size, color);
        }
    }

    /// Draw text at exact native size (no scaling - crispest)
    pub fn draw_text_native(&self, text: &str, x: f32, y: f32, size: u16, color: Color) {
        if let Some(font) = self.fonts.get(&size) {
            draw_text_ex(
                text,
                x.floor(),  // Pixel-perfect positioning
                y.floor(),
                TextParams {
                    font: Some(font),
                    font_size: size,
                    font_scale: 1.0,
                    color,
                    ..Default::default()
                },
            );
        } else if let Some((&fallback_size, font)) = self.fonts.iter().next() {
            // Use any available size as fallback
            let scale = size as f32 / fallback_size as f32;
            draw_text_ex(
                text,
                x.floor(),
                y.floor(),
                TextParams {
                    font: Some(font),
                    font_size: fallback_size,
                    font_scale: scale,
                    color,
                    ..Default::default()
                },
            );
        }
    }

    /// Measure text at the specified size
    pub fn measure_text(&self, text: &str, font_size: f32) -> TextDimensions {
        let native_size = self.get_closest_size(font_size);
        let scale = font_size / native_size as f32;

        if let Some(font) = self.fonts.get(&native_size) {
            measure_text(text, Some(font), native_size, scale)
        } else {
            measure_text(text, None, font_size as u16, 1.0)
        }
    }

    /// Measure text at exact native size
    pub fn measure_text_native(&self, text: &str, size: u16) -> TextDimensions {
        if let Some(font) = self.fonts.get(&size) {
            measure_text(text, Some(font), size, 1.0)
        } else {
            measure_text(text, None, size, 1.0)
        }
    }

    /// Check if font is loaded
    pub fn is_loaded(&self) -> bool {
        !self.fonts.is_empty()
    }
}

impl Default for BitmapFont {
    fn default() -> Self {
        Self {
            fonts: HashMap::new(),
        }
    }
}
