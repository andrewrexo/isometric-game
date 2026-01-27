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

/// D-pad direction
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DPadDirection {
    None,
    Up,
    Down,
    Left,
    Right,
}

impl DPadDirection {
    pub fn to_direction_u8(self) -> u8 {
        match self {
            DPadDirection::Up => 0,
            DPadDirection::Down => 1,
            DPadDirection::Left => 2,
            DPadDirection::Right => 3,
            DPadDirection::None => 0,
        }
    }

    pub fn to_velocity(self) -> (f32, f32) {
        match self {
            DPadDirection::Up => (0.0, -1.0),
            DPadDirection::Down => (0.0, 1.0),
            DPadDirection::Left => (-1.0, 0.0),
            DPadDirection::Right => (1.0, 0.0),
            DPadDirection::None => (0.0, 0.0),
        }
    }
}

/// Virtual D-pad for mobile controls
/// Supports tap-to-face (quick tap) vs hold-to-move (like keyboard)
pub struct VirtualDPad {
    /// Center position of the D-pad
    center: Vec2,
    /// Size of each direction button
    button_size: f32,
    /// Gap between buttons
    gap: f32,
    /// Currently pressed direction
    current_dir: DPadDirection,
    /// Touch ID tracking input
    touch_id: Option<u64>,
    /// Time when direction was first pressed
    press_time: f64,
    /// Whether a move command has been sent (held past threshold)
    move_sent: bool,
    /// Direction that was just released (for face command)
    just_released_dir: DPadDirection,
}

impl VirtualDPad {
    pub fn new() -> Self {
        let (screen_w, screen_h) = virtual_screen_size();
        // Position in bottom-left area
        let center_x = 90.0;
        let center_y = screen_h - 100.0;

        Self {
            center: vec2(center_x, center_y),
            button_size: 52.0,
            gap: 2.0,
            current_dir: DPadDirection::None,
            touch_id: None,
            press_time: 0.0,
            move_sent: false,
            just_released_dir: DPadDirection::None,
        }
    }

    /// Update D-pad position for current screen size
    pub fn update_position(&mut self) {
        let (_, screen_h) = virtual_screen_size();
        self.center = vec2(90.0, screen_h - 100.0);
    }

    /// Get the button rect for a direction
    fn get_button_rect(&self, dir: DPadDirection) -> (f32, f32, f32, f32) {
        let half = self.button_size / 2.0;
        let offset = self.button_size + self.gap;

        let (ox, oy) = match dir {
            DPadDirection::Up => (0.0, -offset),
            DPadDirection::Down => (0.0, offset),
            DPadDirection::Left => (-offset, 0.0),
            DPadDirection::Right => (offset, 0.0),
            DPadDirection::None => (0.0, 0.0),
        };

        (
            self.center.x + ox - half,
            self.center.y + oy - half,
            self.button_size,
            self.button_size,
        )
    }

    /// Check which direction a point is in
    fn hit_test(&self, x: f32, y: f32) -> DPadDirection {
        for dir in [DPadDirection::Up, DPadDirection::Down, DPadDirection::Left, DPadDirection::Right] {
            let (rx, ry, rw, rh) = self.get_button_rect(dir);
            if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                return dir;
            }
        }
        DPadDirection::None
    }

    /// Update D-pad state based on touch input
    /// Returns true if this D-pad consumed the touch
    pub fn update(&mut self, touches: &[Touch], current_time: f64) -> bool {
        let (screen_w, _) = virtual_screen_size();
        self.just_released_dir = DPadDirection::None;

        // If we're tracking a touch, update or release it
        if let Some(tracking_id) = self.touch_id {
            let tracked = touches.iter().find(|t| t.id == tracking_id);

            match tracked {
                Some(touch) => {
                    match touch.phase {
                        TouchPhase::Moved | TouchPhase::Stationary => {
                            let (vx, vy) = to_virtual_coords(touch.position.x, touch.position.y);
                            let new_dir = self.hit_test(vx, vy);

                            // Direction changed while touching
                            if new_dir != self.current_dir {
                                if new_dir != DPadDirection::None {
                                    // Moved to new direction
                                    self.current_dir = new_dir;
                                    if !self.move_sent {
                                        // Restart timer if we haven't started moving yet
                                        self.press_time = current_time;
                                    }
                                } else {
                                    // Moved off all buttons but still touching
                                    // Keep current direction for now
                                }
                            }
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => {
                            self.just_released_dir = self.current_dir;
                            self.release();
                        }
                        _ => {}
                    }
                }
                None => {
                    self.just_released_dir = self.current_dir;
                    self.release();
                }
            }
            return true;
        }

        // Look for a new touch on the D-pad (left side of screen)
        for touch in touches {
            if touch.phase == TouchPhase::Started {
                let (x, y) = to_virtual_coords(touch.position.x, touch.position.y);

                // Only respond to touches on left half of screen
                if x < screen_w * 0.5 {
                    let dir = self.hit_test(x, y);
                    if dir != DPadDirection::None {
                        self.touch_id = Some(touch.id);
                        self.current_dir = dir;
                        self.press_time = current_time;
                        self.move_sent = false;

                        // Check if this same touch also ended in this frame (very quick tap)
                        // This happens when Started and Ended occur in the same frame
                        let also_ended = touches.iter().any(|t| {
                            t.id == touch.id && matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled)
                        });
                        if also_ended {
                            self.just_released_dir = dir;
                            self.release();
                        }

                        return true;
                    }
                }
            }
        }

        false
    }

    /// Release the D-pad
    fn release(&mut self) {
        self.touch_id = None;
        self.current_dir = DPadDirection::None;
        self.move_sent = false;
    }

    /// Get current direction (None if not pressed)
    pub fn get_direction(&self) -> DPadDirection {
        self.current_dir
    }

    /// Get the direction that was just released (for face command detection)
    pub fn get_just_released(&self) -> DPadDirection {
        self.just_released_dir
    }

    /// Get press time for threshold checking
    pub fn get_press_time(&self) -> f64 {
        self.press_time
    }

    /// Mark that a move command was sent (held past threshold)
    pub fn set_move_sent(&mut self, sent: bool) {
        self.move_sent = sent;
    }

    /// Check if move was sent (held past threshold)
    pub fn was_move_sent(&self) -> bool {
        self.move_sent
    }

    /// Check if D-pad is active
    pub fn is_active(&self) -> bool {
        self.touch_id.is_some()
    }

    /// Render the D-pad
    pub fn render(&self) {
        let directions = [DPadDirection::Up, DPadDirection::Down, DPadDirection::Left, DPadDirection::Right];
        let arrows = ["^", "v", "<", ">"];

        for (dir, arrow) in directions.iter().zip(arrows.iter()) {
            let (x, y, w, h) = self.get_button_rect(*dir);
            let is_pressed = self.current_dir == *dir;

            // Button background
            let bg_color = if is_pressed {
                Color::new(1.0, 1.0, 1.0, 0.4)
            } else {
                Color::new(1.0, 1.0, 1.0, 0.15)
            };
            draw_rectangle(x, y, w, h, bg_color);

            // Button border
            let border_color = if is_pressed {
                Color::new(1.0, 1.0, 1.0, 0.8)
            } else {
                Color::new(1.0, 1.0, 1.0, 0.3)
            };
            draw_rectangle_lines(x, y, w, h, 2.0, border_color);

            // Arrow
            let font_size = 16.0;
            let text_dim = measure_text(arrow, None, font_size as u16, 1.0);
            let text_color = if is_pressed {
                WHITE
            } else {
                Color::new(0.95, 0.95, 0.95, 0.9)
            };
            draw_text(
                arrow,
                x + (w - text_dim.width) / 2.0,
                y + (h + text_dim.height) / 2.0,
                font_size,
                text_color,
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
        let font_size = 16.0;
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
    pub dpad: VirtualDPad,
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
            dpad: VirtualDPad::new(),
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
    pub fn update(&mut self, current_time: f64) {
        self.touch_consumed = false;

        if !self.enabled {
            return;
        }

        // Update positions for current virtual screen size
        let (screen_w, screen_h) = virtual_screen_size();
        self.attack_button.set_position(screen_w - 55.0, screen_h - 130.0);
        self.interact_button.set_position(screen_w - 115.0, screen_h - 85.0);
        self.dpad.update_position();

        // Get all current touches
        let touches: Vec<Touch> = touches();

        // Update each control (order matters - buttons first to consume their touches)
        let attack_consumed = self.attack_button.update(&touches);
        let interact_consumed = self.interact_button.update(&touches);
        let dpad_consumed = self.dpad.update(&touches, current_time);

        // Mark touch as consumed if any control is active or just received input
        self.touch_consumed = attack_consumed || interact_consumed || dpad_consumed
            || self.dpad.is_active()
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

        self.dpad.render();
        self.attack_button.render();
        self.interact_button.render();
    }

    /// Get current D-pad direction
    pub fn get_direction(&self) -> DPadDirection {
        if !self.enabled {
            return DPadDirection::None;
        }
        self.dpad.get_direction()
    }

    /// Get direction that was just released (for tap-to-face)
    pub fn get_just_released_direction(&self) -> DPadDirection {
        if !self.enabled {
            return DPadDirection::None;
        }
        self.dpad.get_just_released()
    }

    /// Get D-pad press time for threshold checking
    pub fn get_dpad_press_time(&self) -> f64 {
        self.dpad.get_press_time()
    }

    /// Mark that a move command was sent
    pub fn set_dpad_move_sent(&mut self, sent: bool) {
        self.dpad.set_move_sent(sent);
    }

    /// Check if D-pad move was sent (held past threshold)
    pub fn was_dpad_move_sent(&self) -> bool {
        self.dpad.was_move_sent()
    }

    /// Check if attack button is held (for continuous attacking like space bar)
    pub fn attack_pressed(&self) -> bool {
        self.enabled && self.attack_button.is_pressed()
    }

    /// Check if interact was just pressed
    pub fn interact_pressed(&self) -> bool {
        self.enabled && self.interact_button.just_pressed()
    }
}
