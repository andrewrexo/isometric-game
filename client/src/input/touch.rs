// Touch input handling for mobile devices
// Provides virtual joystick and touch buttons

use macroquad::prelude::*;
use crate::mobile_scale::VIRTUAL_WIDTH;

/// Convert screen coordinates to virtual coordinates (for Android scaling)
#[cfg(target_os = "android")]
fn to_virtual_coords(x: f32, y: f32) -> (f32, f32) {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let (vw, vh) = virtual_screen_size();

    let vx = x * vw / screen_w;
    let vy = y * vh / screen_h;
    (vx, vy)
}

#[cfg(not(target_os = "android"))]
fn to_virtual_coords(x: f32, y: f32) -> (f32, f32) {
    (x, y)
}

/// Get virtual screen dimensions
fn virtual_screen_size() -> (f32, f32) {
    #[cfg(target_os = "android")]
    {
        // Calculate virtual height to match screen aspect ratio
        let screen_w = screen_width();
        let screen_h = screen_height();
        let aspect = screen_h / screen_w;
        let virtual_height = (VIRTUAL_WIDTH * aspect).round();
        (VIRTUAL_WIDTH, virtual_height)
    }
    #[cfg(not(target_os = "android"))]
    {
        (screen_width(), screen_height())
    }
}

/// Virtual joystick state
pub struct VirtualJoystick {
    /// Center position of the joystick (where finger first touched)
    center: Option<Vec2>,
    /// Current touch position
    current: Option<Vec2>,
    /// Touch ID tracking this joystick
    touch_id: Option<u64>,
    /// Maximum distance the stick can move from center
    max_radius: f32,
    /// Dead zone radius (inputs below this are ignored)
    dead_zone: f32,
    /// Visual radius of the joystick base
    base_radius: f32,
    /// Visual radius of the joystick stick
    stick_radius: f32,
    /// Which side of the screen (for positioning)
    side: JoystickSide,
}

#[derive(Clone, Copy, PartialEq)]
pub enum JoystickSide {
    Left,
    Right,
}

impl VirtualJoystick {
    pub fn new(side: JoystickSide) -> Self {
        Self {
            center: None,
            current: None,
            touch_id: None,
            max_radius: 60.0,
            dead_zone: 10.0,
            base_radius: 70.0,
            stick_radius: 35.0,
            side,
        }
    }

    /// Update joystick state based on touch input
    /// Returns true if this joystick consumed the touch
    pub fn update(&mut self, touches: &[Touch]) -> bool {
        let (screen_w, screen_h) = virtual_screen_size();

        // Define the active zone for this joystick (left or right half of screen)
        let zone_start = match self.side {
            JoystickSide::Left => 0.0,
            JoystickSide::Right => screen_w * 0.5,
        };
        let zone_end = match self.side {
            JoystickSide::Left => screen_w * 0.5,
            JoystickSide::Right => screen_w,
        };

        // If we're tracking a touch, update or release it
        if let Some(tracking_id) = self.touch_id {
            // Find our tracked touch
            let tracked = touches.iter().find(|t| t.id == tracking_id);

            match tracked {
                Some(touch) => {
                    match touch.phase {
                        TouchPhase::Moved | TouchPhase::Stationary => {
                            let (vx, vy) = to_virtual_coords(touch.position.x, touch.position.y);
                            self.current = Some(vec2(vx, vy));
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => {
                            self.release();
                        }
                        _ => {}
                    }
                }
                None => {
                    // Touch disappeared
                    self.release();
                }
            }
            return true;
        }

        // Look for a new touch in our zone
        for touch in touches {
            if touch.phase == TouchPhase::Started {
                let (x, y) = to_virtual_coords(touch.position.x, touch.position.y);

                // Check if touch is in our zone and in the lower portion of screen
                if x >= zone_start && x < zone_end && y > screen_h * 0.3 {
                    self.touch_id = Some(touch.id);
                    self.center = Some(vec2(x, y));
                    self.current = Some(vec2(x, y));
                    return true;
                }
            }
        }

        false
    }

    /// Release the joystick
    fn release(&mut self) {
        self.touch_id = None;
        self.center = None;
        self.current = None;
    }

    /// Get the joystick input as a normalized vector (-1 to 1 on each axis)
    pub fn get_input(&self) -> Vec2 {
        match (self.center, self.current) {
            (Some(center), Some(current)) => {
                let delta = current - center;
                let distance = delta.length();

                if distance < self.dead_zone {
                    return Vec2::ZERO;
                }

                // Normalize and clamp to max radius
                let clamped_distance = distance.min(self.max_radius);
                let normalized = delta.normalize_or_zero();

                // Scale to 0-1 range (accounting for dead zone)
                let effective_distance = (clamped_distance - self.dead_zone) / (self.max_radius - self.dead_zone);

                normalized * effective_distance
            }
            _ => Vec2::ZERO,
        }
    }

    /// Check if the joystick is currently active
    pub fn is_active(&self) -> bool {
        self.touch_id.is_some()
    }

    /// Render the joystick
    pub fn render(&self) {
        if let (Some(center), Some(current)) = (self.center, self.current) {
            // Draw base circle (semi-transparent)
            draw_circle(
                center.x,
                center.y,
                self.base_radius,
                Color::new(1.0, 1.0, 1.0, 0.2),
            );
            draw_circle_lines(
                center.x,
                center.y,
                self.base_radius,
                2.0,
                Color::new(1.0, 1.0, 1.0, 0.4),
            );

            // Calculate stick position (clamped to max radius)
            let delta = current - center;
            let distance = delta.length().min(self.max_radius);
            let stick_pos = if delta.length() > 0.0 {
                center + delta.normalize() * distance
            } else {
                center
            };

            // Draw stick
            draw_circle(
                stick_pos.x,
                stick_pos.y,
                self.stick_radius,
                Color::new(1.0, 1.0, 1.0, 0.5),
            );
            draw_circle_lines(
                stick_pos.x,
                stick_pos.y,
                self.stick_radius,
                2.0,
                Color::new(1.0, 1.0, 1.0, 0.7),
            );
        }
    }
}

/// Touch button for actions like attack
pub struct TouchButton {
    /// Position of the button
    pub position: Vec2,
    /// Radius of the button
    pub radius: f32,
    /// Whether the button is currently pressed
    pressed: bool,
    /// Whether the button was just pressed this frame
    just_pressed: bool,
    /// Touch ID tracking this button
    touch_id: Option<u64>,
    /// Label to display
    pub label: String,
    /// Color when not pressed
    pub color: Color,
    /// Color when pressed
    pub pressed_color: Color,
}

impl TouchButton {
    pub fn new(x: f32, y: f32, radius: f32, label: &str) -> Self {
        Self {
            position: vec2(x, y),
            radius,
            pressed: false,
            just_pressed: false,
            touch_id: None,
            label: label.to_string(),
            color: Color::new(1.0, 0.3, 0.3, 0.5),
            pressed_color: Color::new(1.0, 0.5, 0.5, 0.7),
        }
    }

    /// Update button position (call each frame to handle screen resize)
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.position = vec2(x, y);
    }

    /// Update button state based on touch input
    /// Returns true if this button consumed the touch
    pub fn update(&mut self, touches: &[Touch]) -> bool {
        self.just_pressed = false;

        // If we're tracking a touch, check if it's still valid
        if let Some(tracking_id) = self.touch_id {
            let tracked = touches.iter().find(|t| t.id == tracking_id);

            match tracked {
                Some(touch) => {
                    match touch.phase {
                        TouchPhase::Ended | TouchPhase::Cancelled => {
                            self.pressed = false;
                            self.touch_id = None;
                        }
                        _ => {}
                    }
                }
                None => {
                    self.pressed = false;
                    self.touch_id = None;
                }
            }
            return self.pressed;
        }

        // Look for a new touch on this button
        for touch in touches {
            if touch.phase == TouchPhase::Started {
                let (vx, vy) = to_virtual_coords(touch.position.x, touch.position.y);
                let touch_pos = vec2(vx, vy);
                let distance = (touch_pos - self.position).length();

                if distance <= self.radius {
                    self.touch_id = Some(touch.id);
                    self.pressed = true;
                    self.just_pressed = true;
                    return true;
                }
            }
        }

        false
    }

    /// Check if button is currently pressed
    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    /// Check if button was just pressed this frame
    pub fn just_pressed(&self) -> bool {
        self.just_pressed
    }

    /// Render the button
    pub fn render(&self) {
        let color = if self.pressed { self.pressed_color } else { self.color };

        // Draw button circle
        draw_circle(self.position.x, self.position.y, self.radius, color);
        draw_circle_lines(
            self.position.x,
            self.position.y,
            self.radius,
            3.0,
            Color::new(1.0, 1.0, 1.0, 0.7),
        );

        // Draw label
        let font_size = 24.0;
        let text_dim = measure_text(&self.label, None, font_size as u16, 1.0);
        draw_text(
            &self.label,
            self.position.x - text_dim.width / 2.0,
            self.position.y + text_dim.height / 4.0,
            font_size,
            WHITE,
        );
    }
}

/// Container for all touch controls
pub struct TouchControls {
    pub joystick: VirtualJoystick,
    pub attack_button: TouchButton,
    pub interact_button: TouchButton,
    /// Whether touch controls are enabled (typically only on Android)
    pub enabled: bool,
    /// Whether any touch was consumed by controls this frame
    touch_consumed: bool,
}

impl TouchControls {
    pub fn new() -> Self {
        let (screen_w, screen_h) = virtual_screen_size();

        // Position buttons on the right side of the screen, above menu buttons
        // Smaller buttons to avoid overlap with bottom UI
        let attack_x = screen_w - 55.0;
        let attack_y = screen_h - 130.0;
        let interact_x = screen_w - 115.0;
        let interact_y = screen_h - 85.0;

        Self {
            joystick: VirtualJoystick::new(JoystickSide::Left),
            attack_button: TouchButton::new(attack_x, attack_y, 40.0, "ATK"),
            interact_button: TouchButton::new(interact_x, interact_y, 32.0, "USE"),
            #[cfg(target_os = "android")]
            enabled: true,
            #[cfg(not(target_os = "android"))]
            enabled: false,
            touch_consumed: false,
        }
    }

    /// Update all touch controls
    pub fn update(&mut self) {
        self.touch_consumed = false;

        if !self.enabled {
            return;
        }

        // Update button positions for current virtual screen size (above menu buttons)
        let (screen_w, screen_h) = virtual_screen_size();
        self.attack_button.set_position(screen_w - 55.0, screen_h - 130.0);
        self.interact_button.set_position(screen_w - 115.0, screen_h - 85.0);

        // Get all current touches
        let touches: Vec<Touch> = touches();

        // Update each control (order matters - buttons first to consume their touches)
        // Track if any control consumed a touch
        let attack_consumed = self.attack_button.update(&touches);
        let interact_consumed = self.interact_button.update(&touches);
        let joystick_consumed = self.joystick.update(&touches);

        // Mark touch as consumed if any control is active or just received input
        self.touch_consumed = attack_consumed || interact_consumed || joystick_consumed
            || self.joystick.is_active()
            || self.attack_button.is_pressed()
            || self.interact_button.is_pressed();
    }

    /// Check if touch input was consumed by controls this frame
    /// Use this to prevent touch from triggering map clicks
    pub fn consumed_touch(&self) -> bool {
        self.touch_consumed
    }

    /// Render all touch controls
    pub fn render(&self) {
        if !self.enabled {
            return;
        }

        self.joystick.render();
        self.attack_button.render();
        self.interact_button.render();
    }

    /// Get movement input from joystick
    pub fn get_movement(&self) -> (f32, f32) {
        if !self.enabled {
            return (0.0, 0.0);
        }
        let input = self.joystick.get_input();
        (input.x, input.y)
    }

    /// Check if attack was just pressed
    pub fn attack_pressed(&self) -> bool {
        self.enabled && self.attack_button.just_pressed()
    }

    /// Check if interact was just pressed
    pub fn interact_pressed(&self) -> bool {
        self.enabled && self.interact_button.just_pressed()
    }
}
