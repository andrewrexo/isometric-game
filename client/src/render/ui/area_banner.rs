//! Area banner UI - displays location name during map transitions

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
