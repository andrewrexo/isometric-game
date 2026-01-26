// Touch input handling for mobile devices
// Provides virtual joystick and touch buttons

use macroquad::prelude::*;

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
    pub fn update(&mut self, touches: &[TouchPoint]) -> bool {
        let screen_w = screen_width();
        let screen_h = screen_height();

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
                            self.current = Some(vec2(touch.position.x, touch.position.y));
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
                let x = touch.position.x;
                let y = touch.position.y;

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
    pub fn update(&mut self, touches: &[TouchPoint]) -> bool {
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
                let touch_pos = vec2(touch.position.x, touch.position.y);
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
}

impl TouchControls {
    pub fn new() -> Self {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // Position buttons on the right side of the screen
        let attack_x = screen_w - 100.0;
        let attack_y = screen_h - 150.0;
        let interact_x = screen_w - 180.0;
        let interact_y = screen_h - 80.0;

        Self {
            joystick: VirtualJoystick::new(JoystickSide::Left),
            attack_button: TouchButton::new(attack_x, attack_y, 50.0, "ATK"),
            interact_button: TouchButton::new(interact_x, interact_y, 40.0, "USE"),
            #[cfg(target_os = "android")]
            enabled: true,
            #[cfg(not(target_os = "android"))]
            enabled: false,
        }
    }

    /// Update all touch controls
    pub fn update(&mut self) {
        if !self.enabled {
            return;
        }

        // Update button positions for current screen size
        let screen_w = screen_width();
        let screen_h = screen_height();
        self.attack_button.set_position(screen_w - 100.0, screen_h - 150.0);
        self.interact_button.set_position(screen_w - 180.0, screen_h - 80.0);

        // Get all current touches
        let touches: Vec<TouchPoint> = touches().collect();

        // Update each control (order matters - buttons first to consume their touches)
        self.attack_button.update(&touches);
        self.interact_button.update(&touches);
        self.joystick.update(&touches);
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
