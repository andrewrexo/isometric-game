//! Area banner UI - displays location name during map transitions

use macroquad::prelude::*;
use crate::render::Renderer;

/// Banner display phase
#[derive(Debug, Clone, PartialEq)]
pub enum BannerPhase {
    Hidden,
    FadingIn,
    Holding,
    FadingOut,
}

/// Timing constants
const FADE_IN_DURATION: f32 = 0.5;
const HOLD_DURATION: f32 = 2.5;
const FADE_OUT_DURATION: f32 = 0.5;

/// Overworld display name
pub const OVERWORLD_NAME: &str = "Verdant Fields";

/// Area banner state
#[derive(Debug, Clone)]
pub struct AreaBanner {
    pub text: String,
    pub phase: BannerPhase,
    pub timer: f32,
}

impl Default for AreaBanner {
    fn default() -> Self {
        Self {
            text: String::new(),
            phase: BannerPhase::Hidden,
            timer: 0.0,
        }
    }
}

impl AreaBanner {
    /// Trigger the banner with a new area name
    pub fn show(&mut self, name: &str) {
        self.text = name.to_string();
        self.phase = BannerPhase::FadingIn;
        self.timer = FADE_IN_DURATION;
    }

    /// Update the banner timer, transitioning phases as needed
    pub fn update(&mut self, delta: f32) {
        if self.phase == BannerPhase::Hidden {
            return;
        }

        self.timer -= delta;

        if self.timer <= 0.0 {
            match self.phase {
                BannerPhase::FadingIn => {
                    self.phase = BannerPhase::Holding;
                    self.timer = HOLD_DURATION;
                }
                BannerPhase::Holding => {
                    self.phase = BannerPhase::FadingOut;
                    self.timer = FADE_OUT_DURATION;
                }
                BannerPhase::FadingOut => {
                    self.phase = BannerPhase::Hidden;
                    self.timer = 0.0;
                }
                BannerPhase::Hidden => {}
            }
        }
    }

    /// Get current opacity (0.0 to 1.0)
    pub fn opacity(&self) -> f32 {
        match self.phase {
            BannerPhase::Hidden => 0.0,
            BannerPhase::FadingIn => 1.0 - (self.timer / FADE_IN_DURATION),
            BannerPhase::Holding => 1.0,
            BannerPhase::FadingOut => self.timer / FADE_OUT_DURATION,
        }
    }

    /// Check if banner should be rendered
    pub fn is_visible(&self) -> bool {
        self.phase != BannerPhase::Hidden
    }
}

impl Renderer {
    /// Render the area banner (called from main render loop)
    pub fn render_area_banner(&self, text: &str, opacity: f32) {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // Colors
        let text_color = Color::new(0.96, 0.94, 0.88, opacity); // Off-white/cream
        let flourish_color = Color::new(0.96, 0.94, 0.88, opacity * 0.7);
        let shadow_color = Color::new(0.1, 0.08, 0.05, opacity * 0.5);
        let bg_color = Color::new(0.0, 0.0, 0.0, opacity * 0.3);

        // Position: 18% down from top
        let banner_y = screen_h * 0.18;

        // Measure text
        let font_size = 28.0;
        let text_dims = self.measure_text_sharp(text, font_size);

        // Banner dimensions
        let padding_x = 40.0;
        let padding_y = 16.0;
        let banner_width = text_dims.width + padding_x * 2.0;
        let banner_height = text_dims.height + padding_y * 2.0;
        let banner_x = (screen_w - banner_width) / 2.0;

        // Draw semi-transparent background
        draw_rectangle(
            banner_x,
            banner_y - padding_y,
            banner_width,
            banner_height,
            bg_color,
        );

        // Draw flourishes (decorative lines)
        let flourish_width = text_dims.width * 0.8;
        let flourish_x = (screen_w - flourish_width) / 2.0;

        // Top flourish (thicker)
        let top_y = banner_y - 4.0;
        draw_line(flourish_x, top_y, flourish_x + flourish_width, top_y, 2.0, flourish_color);

        // Bottom flourish (thinner, slightly shorter)
        let bottom_flourish_width = flourish_width * 0.9;
        let bottom_flourish_x = (screen_w - bottom_flourish_width) / 2.0;
        let bottom_y = banner_y + text_dims.height + 8.0;
        draw_line(bottom_flourish_x, bottom_y, bottom_flourish_x + bottom_flourish_width, bottom_y, 1.0, flourish_color);

        // Draw text shadow
        let text_x = (screen_w - text_dims.width) / 2.0;
        let text_y = banner_y + text_dims.height * 0.8;
        self.draw_text_sharp(text, text_x + 2.0, text_y + 2.0, font_size, shadow_color);

        // Draw text
        self.draw_text_sharp(text, text_x, text_y, font_size, text_color);
    }
}
