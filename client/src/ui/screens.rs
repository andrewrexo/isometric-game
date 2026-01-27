use macroquad::prelude::*;
use macroquad::miniquad::window::show_keyboard;
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use crate::auth::{AuthClient, AuthSession, CharacterInfo};
use crate::audio::AudioManager;
use crate::render::BitmapFont;
use crate::util::{asset_path, virtual_screen_size};

// Sprite sheet constants for character preview
const SPRITE_WIDTH: f32 = 34.0;
const SPRITE_HEIGHT: f32 = 78.0;

/// Convert screen coordinates to virtual coordinates (for Android scaling)
fn screen_to_virtual(x: f32, y: f32) -> (f32, f32) {
    let (vw, vh) = virtual_screen_size();
    let screen_w = screen_width();
    let screen_h = screen_height();

    // On desktop, virtual == screen, so this is a no-op
    let vx = x * vw / screen_w;
    let vy = y * vh / screen_h;
    (vx, vy)
}

/// Get input position and click state from either mouse or touch
/// Returns (position, just_clicked, is_touching)
fn get_input_state() -> (Vec2, bool, bool) {
    let touches: Vec<Touch> = touches();

    // Check for touch input first (mobile)
    for touch in &touches {
        if touch.phase == TouchPhase::Started {
            let (vx, vy) = screen_to_virtual(touch.position.x, touch.position.y);
            return (vec2(vx, vy), true, true);
        }
    }

    // Check for any active touch (for position tracking)
    if let Some(touch) = touches.first() {
        let (vx, vy) = screen_to_virtual(touch.position.x, touch.position.y);
        return (vec2(vx, vy), false, true);
    }

    // Fall back to mouse input (desktop)
    let (mx, my) = mouse_position();
    let (vx, vy) = screen_to_virtual(mx, my);
    let clicked = is_mouse_button_pressed(MouseButton::Left);
    (vec2(vx, vy), clicked, false)
}

/// Load all player sprite textures (gender x skin combinations)
async fn load_player_sprites() -> HashMap<String, Texture2D> {
    let mut sprites = HashMap::new();
    let genders = ["male", "female"];
    let skins = ["tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"];

    for gender in &genders {
        for skin in &skins {
            let path = asset_path(&format!("assets/sprites/players/player_{}_{}.png", gender, skin));
            if let Ok(texture) = load_texture(&path).await {
                texture.set_filter(FilterMode::Nearest);
                let key = format!("{}_{}", gender, skin);
                sprites.insert(key, texture);
            }
        }
    }

    sprites
}

/// Check if a point is inside a rectangle
fn point_in_rect(px: f32, py: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    px >= rx && px < rx + rw && py >= ry && py < ry + rh
}

// Hair sprite dimensions (different from player sprites)
const HAIR_SPRITE_WIDTH: f32 = 28.0;
const HAIR_SPRITE_HEIGHT: f32 = 54.0;

/// Draw a character preview sprite at the given position
/// Uses the idle frame (row 0, column 0) facing down
fn draw_character_preview(
    sprites: &HashMap<String, Texture2D>,
    hair_sprites: &HashMap<i32, Texture2D>,
    gender: &str,
    skin: &str,
    hair_style: Option<i32>,
    hair_color: i32,
    x: f32,
    y: f32,
    scale: f32,
) {
    let key = format!("{}_{}", gender, skin);
    if let Some(texture) = sprites.get(&key) {
        // Idle frame is at row 0, column 0
        let src_x = 0.0;
        let src_y = 0.0;

        let dest_w = SPRITE_WIDTH * scale;
        let dest_h = SPRITE_HEIGHT * scale;

        // Draw base character sprite
        draw_texture_ex(
            texture,
            x,
            y,
            WHITE,
            DrawTextureParams {
                source: Some(Rect::new(src_x, src_y, SPRITE_WIDTH, SPRITE_HEIGHT)),
                dest_size: Some(Vec2::new(dest_w, dest_h)),
                ..Default::default()
            },
        );

        // Draw hair if selected
        if let Some(style) = hair_style {
            if let Some(hair_tex) = hair_sprites.get(&style) {
                // Front frame for preview (facing down)
                let hair_frame_index = hair_color * 2; // front frame
                let hair_src_x = hair_frame_index as f32 * HAIR_SPRITE_WIDTH;

                // Scale hair sprite
                let hair_dest_w = HAIR_SPRITE_WIDTH * scale;
                let hair_dest_h = HAIR_SPRITE_HEIGHT * scale;

                // Hair offset: 3 pixels up, 1 pixel left (facing down, not flipped)
                let hair_offset_x = -1.0 * scale;
                let hair_offset_y = -3.0 * scale;

                // Center hair horizontally on player, apply offsets
                let hair_x = x + (dest_w - hair_dest_w) / 2.0 + hair_offset_x;
                let hair_y = y + hair_offset_y;

                draw_texture_ex(
                    hair_tex,
                    hair_x,
                    hair_y,
                    WHITE,
                    DrawTextureParams {
                        source: Some(Rect::new(hair_src_x, 0.0, HAIR_SPRITE_WIDTH, HAIR_SPRITE_HEIGHT)),
                        dest_size: Some(Vec2::new(hair_dest_w, hair_dest_h)),
                        ..Default::default()
                    },
                );
            }
        }
    } else {
        // Fallback: draw a colored rectangle if sprite not found
        let dest_w = SPRITE_WIDTH * scale;
        let dest_h = SPRITE_HEIGHT * scale;
        draw_rectangle(x, y, dest_w, dest_h, Color::from_rgba(100, 100, 100, 255));
    }
}

/// Result of screen update - tells main loop what to do next
pub enum ScreenState {
    /// Stay on current screen
    Continue,
    /// Move to character select with auth session
    #[cfg(not(target_arch = "wasm32"))]
    ToCharacterSelect(AuthSession),
    /// Move to character creation screen
    #[cfg(not(target_arch = "wasm32"))]
    ToCharacterCreate(AuthSession),
    /// Start the game with the selected character
    #[cfg(not(target_arch = "wasm32"))]
    StartGame {
        session: AuthSession,
        character_id: i64,
        character_name: String,
    },
    /// Guest mode (dev only)
    StartGuestMode,
    /// Go back to login
    ToLogin,
}

pub trait Screen {
    fn update(&mut self, audio: &AudioManager) -> ScreenState;
    fn render(&self);
}

// ============================================================================
// Login Screen
// ============================================================================

pub struct LoginScreen {
    username: String,
    password: String,
    active_field: LoginField,
    mode: LoginMode,
    error_message: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    auth_client: AuthClient,
    font: BitmapFont,
    logo: Option<Texture2D>,
}

#[derive(PartialEq, Clone, Copy)]
enum LoginField {
    Username,
    Password,
}

#[derive(PartialEq, Clone, Copy)]
enum LoginMode {
    Login,
    Register,
}

impl LoginScreen {
    pub fn new(server_url: &str) -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            active_field: LoginField::Username,
            mode: LoginMode::Login,
            error_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            auth_client: AuthClient::new(server_url),
            font: BitmapFont::default(),
            logo: None,
        }
    }

    /// Load font and logo asynchronously - call this after creating the screen
    pub async fn load_font(&mut self) {
        self.font = BitmapFont::load_or_default("assets/fonts/monogram/ttf/monogram-extended.ttf").await;

        // Load logo texture
        if let Ok(texture) = load_texture(&asset_path("assets/ui/logo.png")).await {
            texture.set_filter(FilterMode::Nearest);
            self.logo = Some(texture);
        }
    }

    /// Draw text with pixel font for sharp rendering
    fn draw_text_sharp(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        self.font.draw_text(text, x, y, font_size, color);
    }

    fn measure_text_sharp(&self, text: &str, font_size: f32) -> TextDimensions {
        self.font.measure_text(text, font_size)
    }

    fn handle_text_input(&mut self) {
        // Handle character input
        while let Some(c) = get_char_pressed() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                let field = match self.active_field {
                    LoginField::Username => &mut self.username,
                    LoginField::Password => &mut self.password,
                };
                if field.len() < 20 {
                    field.push(c);
                }
            }
        }

        // Handle backspace
        if is_key_pressed(KeyCode::Backspace) {
            let field = match self.active_field {
                LoginField::Username => &mut self.username,
                LoginField::Password => &mut self.password,
            };
            field.pop();
        }
    }
}

impl Screen for LoginScreen {
    fn update(&mut self, audio: &AudioManager) -> ScreenState {
        let (sw, sh) = virtual_screen_size();
        let (input_pos, clicked, _is_touch) = get_input_state();
        let mx = input_pos.x;
        let my = input_pos.y;

        // Layout constants (must match render)
        let box_width = sw.min(340.0);  // Wider inputs
        let box_height = 40.0;          // Taller inputs
        let box_x = (sw - box_width) / 2.0;
        let btn_height = 36.0;          // Taller buttons
        let spacing = 10.0;             // More spacing

        // Calculate positions flowing from top (must match render)
        let logo_h = 60.0;
        let start_y = logo_h + 30.0;
        let username_y = start_y;
        let username_field_y = username_y + 12.0;  // Box position (after label) - reduced from 16.0
        let password_y = username_field_y + box_height + spacing + 10.0; // Added 10.0 extra margin
        let password_field_y = password_y + 12.0;  // Box position (after label) - reduced from 16.0
        let buttons_y = password_field_y + box_height + spacing + 24.0;

        // Handle touch/click on input fields and buttons
        if clicked {
            // Username field (clickable box area)
            if point_in_rect(mx, my, box_x, username_field_y, box_width, box_height) {
                self.active_field = LoginField::Username;
                show_keyboard(true);
            }

            // Password field (clickable box area)
            else if point_in_rect(mx, my, box_x, password_field_y, box_width, box_height) {
                self.active_field = LoginField::Password;
                show_keyboard(true);
            }

            // Tapped elsewhere - hide keyboard
            else {
                show_keyboard(false);
            }

            // Login/Register button (left side)
            let login_btn_w = (box_width - spacing) / 2.0;
            if point_in_rect(mx, my, box_x, buttons_y, login_btn_w, btn_height) {
                show_keyboard(false);
                if self.username.len() >= 3 && self.password.len() >= 6 {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let result = match self.mode {
                            LoginMode::Login => self.auth_client.login(&self.username, &self.password),
                            LoginMode::Register => self.auth_client.register(&self.username, &self.password),
                        };

                        match result {
                            Ok(session) => {
                                return ScreenState::ToCharacterSelect(session);
                            }
                            Err(e) => {
                                self.error_message = Some(e.to_string());
                            }
                        }
                    }
                } else if self.username.len() < 3 {
                    self.error_message = Some("Username min 3 chars".to_string());
                } else {
                    self.error_message = Some("Password min 6 chars".to_string());
                }
            }

            // Toggle mode button (right side, same row)
            let toggle_x = box_x + login_btn_w + spacing;
            if point_in_rect(mx, my, toggle_x, buttons_y, login_btn_w, btn_height) {
                self.mode = match self.mode {
                    LoginMode::Login => LoginMode::Register,
                    LoginMode::Register => LoginMode::Login,
                };
                self.error_message = None;
            }
        }

        // Tab to switch fields (check before handle_text_input consumes it)
        if is_key_pressed(KeyCode::Tab) {
            audio.play_sfx("enter");
            self.active_field = match self.active_field {
                LoginField::Username => LoginField::Password,
                LoginField::Password => LoginField::Username,
            };
        }

        self.handle_text_input();

        // Clear error on any input
        if is_key_pressed(KeyCode::Backspace) || get_char_pressed().is_some() {
            self.error_message = None;
        }

        // Toggle between login/register
        if is_key_pressed(KeyCode::F1) {
            self.mode = match self.mode {
                LoginMode::Login => LoginMode::Register,
                LoginMode::Register => LoginMode::Login,
            };
            self.error_message = None;
        }

        // Submit on Enter
        if is_key_pressed(KeyCode::Enter) {
            if self.username.len() < 3 {
                self.error_message = Some("Username must be at least 3 characters".to_string());
                return ScreenState::Continue;
            }
            if self.password.len() < 6 {
                self.error_message = Some("Password must be at least 6 characters".to_string());
                return ScreenState::Continue;
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let result = match self.mode {
                    LoginMode::Login => self.auth_client.login(&self.username, &self.password),
                    LoginMode::Register => self.auth_client.register(&self.username, &self.password),
                };

                match result {
                    Ok(session) => {
                        // Go to character select screen
                        return ScreenState::ToCharacterSelect(session);
                    }
                    Err(e) => {
                        self.error_message = Some(e.to_string());
                    }
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                self.error_message = Some("Network not available in WASM".to_string());
            }
        }

        ScreenState::Continue
    }

    fn render(&self) {
        let (sw, sh) = virtual_screen_size();
        let (input_pos, _, _) = get_input_state();
        let mx = input_pos.x;
        let my = input_pos.y;

        // Background
        clear_background(Color::from_rgba(25, 25, 35, 255));

        // Draw decorative grid lines (fewer for performance)
        for i in 0..10 {
            let alpha = 0.05 + (i as f32 * 0.01);
            let color = Color::new(0.3, 0.4, 0.5, alpha);
            draw_line(0.0, i as f32 * 30.0, sw, i as f32 * 30.0, 1.0, color);
            draw_line(i as f32 * 70.0, 0.0, i as f32 * 70.0, sh, 1.0, color);
        }

        // Layout constants (must match update)
        let box_width = sw.min(340.0);  // Wider inputs
        let box_height = 40.0;          // Taller inputs
        let box_x = (sw - box_width) / 2.0;
        let btn_height = 36.0;          // Taller buttons
        let spacing = 10.0;             // More spacing
        let font_size = 16.0;           // Native font size for crisp rendering

        // Calculate positions flowing from top
        let logo_h = 60.0;
        let start_y = logo_h + 30.0;

        // Logo (smaller for compact layout)
        if let Some(logo) = &self.logo {
            let logo_scale = 0.15;
            let logo_w = logo.width() * logo_scale;
            let logo_actual_h = logo.height() * logo_scale;
            let logo_x = (sw - logo_w) / 2.0;
            let logo_y: f32 = 8.0;
            draw_texture_ex(
                logo,
                logo_x.floor(),
                logo_y.floor(),
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(logo_w.floor(), logo_actual_h.floor())),
                    ..Default::default()
                },
            );
        } else {
            // Fallback title (native 16pt for crisp rendering)
            let title = "NEW AEVEN";
            let title_width = self.measure_text_sharp(title, 16.0).width;
            self.draw_text_sharp(title, (sw - title_width) / 2.0, 30.0, 16.0, WHITE);
        }

        // Subtitle (use native 16pt for crisp rendering)
        let subtitle = match self.mode {
            LoginMode::Login => "Login",
            LoginMode::Register => "Register",
        };
        let sub_width = self.measure_text_sharp(subtitle, 16.0).width;
        self.draw_text_sharp(subtitle, (sw - sub_width) / 2.0, logo_h + 12.0, 16.0, GRAY);

        // Username field
        let username_y = start_y;
        let username_active = self.active_field == LoginField::Username;
        let username_color = if username_active { Color::from_rgba(80, 120, 180, 255) } else { Color::from_rgba(60, 60, 80, 255) };

        self.draw_text_sharp("Username", box_x, username_y, font_size, LIGHTGRAY);
        let field_y = username_y + 12.0; // Reduced from 16.0
        draw_rectangle(box_x, field_y, box_width, box_height, username_color);
        draw_rectangle_lines(box_x, field_y, box_width, box_height, 2.0, if username_active { WHITE } else { GRAY });

        let username_display = if self.username.is_empty() && !username_active {
            "Enter username...".to_string()
        } else {
            let cursor = if username_active && (get_time() * 2.0) as i32 % 2 == 0 { "|" } else { "" };
            format!("{}{}", self.username, cursor)
        };
        let text_color = if self.username.is_empty() && !username_active { DARKGRAY } else { WHITE };
        self.draw_text_sharp(&username_display, box_x + 10.0, field_y + 27.0, font_size, text_color);

        // Password field
        let password_y = field_y + box_height + spacing + 10.0; // Added 10.0 extra margin
        let password_active = self.active_field == LoginField::Password;
        let password_color = if password_active { Color::from_rgba(80, 120, 180, 255) } else { Color::from_rgba(60, 60, 80, 255) };

        self.draw_text_sharp("Password", box_x, password_y, font_size, LIGHTGRAY);
        let pass_field_y = password_y + 12.0; // Reduced from 16.0
        draw_rectangle(box_x, pass_field_y, box_width, box_height, password_color);
        draw_rectangle_lines(box_x, pass_field_y, box_width, box_height, 2.0, if password_active { WHITE } else { GRAY });

        let password_display = if self.password.is_empty() && !password_active {
            "Enter password...".to_string()
        } else {
            let masked: String = "*".repeat(self.password.len());
            let cursor = if password_active && (get_time() * 2.0) as i32 % 2 == 0 { "|" } else { "" };
            format!("{}{}", masked, cursor)
        };
        let text_color = if self.password.is_empty() && !password_active { DARKGRAY } else { WHITE };
        self.draw_text_sharp(&password_display, box_x + 10.0, pass_field_y + 27.0, font_size, text_color);

        // Error message (use native 16pt for crisp rendering)
        if let Some(ref error) = self.error_message {
            let error_y = pass_field_y + box_height + 4.0;
            self.draw_text_sharp(error, box_x, error_y + 14.0, 16.0, RED);
        }

        // Buttons row - Login and Toggle side by side
        let buttons_y = pass_field_y + box_height + spacing + 24.0;
        let btn_w = (box_width - spacing) / 2.0;

        // Login/Register button (left)
        let enter_text = match self.mode {
            LoginMode::Login => "Login",
            LoginMode::Register => "Register",
        };
        let login_hovered = point_in_rect(mx, my, box_x, buttons_y, btn_w, btn_height);
        let login_bg = if login_hovered {
            Color::from_rgba(60, 140, 90, 255)
        } else {
            Color::from_rgba(40, 100, 60, 255)
        };
        let login_border = if login_hovered {
            Color::from_rgba(100, 255, 150, 255)
        } else {
            GREEN
        };
        draw_rectangle(box_x, buttons_y, btn_w, btn_height, login_bg);
        draw_rectangle_lines(box_x, buttons_y, btn_w, btn_height, 2.0, login_border);
        let enter_w = self.measure_text_sharp(enter_text, font_size).width;
        self.draw_text_sharp(enter_text, box_x + (btn_w - enter_w) / 2.0, buttons_y + 24.0, font_size, WHITE);

        // Toggle mode button (right)
        let toggle_text = match self.mode {
            LoginMode::Login => "Register",
            LoginMode::Register => "Login",
        };
        let toggle_x = box_x + btn_w + spacing;
        let toggle_hovered = point_in_rect(mx, my, toggle_x, buttons_y, btn_w, btn_height);
        let toggle_bg = if toggle_hovered {
            Color::from_rgba(120, 120, 60, 255)
        } else {
            Color::from_rgba(80, 80, 40, 255)
        };
        let toggle_border = if toggle_hovered {
            Color::from_rgba(255, 255, 100, 255)
        } else {
            YELLOW
        };
        draw_rectangle(toggle_x, buttons_y, btn_w, btn_height, toggle_bg);
        draw_rectangle_lines(toggle_x, buttons_y, btn_w, btn_height, 2.0, toggle_border);
        let toggle_w = self.measure_text_sharp(toggle_text, font_size).width;
        self.draw_text_sharp(toggle_text, toggle_x + (btn_w - toggle_w) / 2.0, buttons_y + 24.0, font_size, WHITE);

        // Version (bottom right, native 16pt for crisp rendering)
        self.draw_text_sharp("v0.1.0", sw - 60.0, sh - 10.0, 16.0, DARKGRAY);
    }
}

// ============================================================================
// Character Select Screen
// ============================================================================

/// Maximum characters per account
#[cfg(not(target_arch = "wasm32"))]
const MAX_CHARACTERS: usize = 3;

#[cfg(not(target_arch = "wasm32"))]
pub struct CharacterSelectScreen {
    session: AuthSession,
    characters: Vec<CharacterInfo>,
    selected_index: usize,
    error_message: Option<String>,
    auth_client: AuthClient,
    font: BitmapFont,
    confirm_delete: bool,
    player_sprites: HashMap<String, Texture2D>,
    hair_sprites: HashMap<i32, Texture2D>,
}

#[cfg(not(target_arch = "wasm32"))]
impl CharacterSelectScreen {
    pub fn new(session: AuthSession, server_url: &str) -> Self {
        let auth_client = AuthClient::new(server_url);

        // Load characters
        let characters = auth_client.get_characters(&session.token).unwrap_or_default();

        Self {
            session,
            characters,
            selected_index: 0,
            error_message: None,
            auth_client,
            font: BitmapFont::default(),
            confirm_delete: false,
            player_sprites: HashMap::new(),
            hair_sprites: HashMap::new(),
        }
    }

    /// Load font and sprites asynchronously - call this after creating the screen
    pub async fn load_font(&mut self) {
        self.font = BitmapFont::load_or_default("assets/fonts/monogram/ttf/monogram-extended.ttf").await;
        self.player_sprites = load_player_sprites().await;
        // Load hair sprites
        for style in 0..3i32 {
            let path = asset_path(&format!("assets/sprites/hair/hair_{}.png", style));
            if let Ok(tex) = load_texture(&path).await {
                tex.set_filter(FilterMode::Nearest);
                self.hair_sprites.insert(style, tex);
            }
        }
    }

    /// Refresh the character list from the server
    pub fn refresh_characters(&mut self) {
        if let Ok(chars) = self.auth_client.get_characters(&self.session.token) {
            self.characters = chars;
            if self.selected_index >= self.characters.len() && !self.characters.is_empty() {
                self.selected_index = self.characters.len() - 1;
            }
        }
    }

    /// Draw text with pixel font for sharp rendering
    fn draw_text_sharp(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        self.font.draw_text(text, x, y, font_size, color);
    }

    fn measure_text_sharp(&self, text: &str, font_size: f32) -> TextDimensions {
        self.font.measure_text(text, font_size)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Screen for CharacterSelectScreen {
    fn update(&mut self, _audio: &AudioManager) -> ScreenState {
        let (sw, sh) = virtual_screen_size();
        let (input_pos, clicked, _is_touch) = get_input_state();
        let mx = input_pos.x;
        let my = input_pos.y;

        // Layout constants (must match render)
        let list_x = (sw - 500.0) / 2.0;
        let list_y = 44.0;
        let item_height = 70.0;
        let inst_y = sh - 70.0;

        // Delete confirmation mode
        if self.confirm_delete {
            // Keyboard shortcuts
            if is_key_pressed(KeyCode::Y) {
                if !self.characters.is_empty() {
                    let char_id = self.characters[self.selected_index].id;
                    if self.auth_client.delete_character(&self.session.token, char_id).is_ok() {
                        self.refresh_characters();
                    }
                }
                self.confirm_delete = false;
                return ScreenState::Continue;
            }
            if is_key_pressed(KeyCode::N) || is_key_pressed(KeyCode::Escape) {
                self.confirm_delete = false;
                return ScreenState::Continue;
            }

            // Mouse clicks on Yes/No buttons
            if clicked {
                let box_w = 450.0;
                let box_h = 150.0;
                let box_x = (sw - box_w) / 2.0;
                let box_y = (sh - box_h) / 2.0;

                // Yes button area (left side of button text)
                let yes_x = box_x + 70.0;
                let yes_y = box_y + 85.0;
                if point_in_rect(mx, my, yes_x, yes_y, 100.0, 30.0) {
                    if !self.characters.is_empty() {
                        let char_id = self.characters[self.selected_index].id;
                        if self.auth_client.delete_character(&self.session.token, char_id).is_ok() {
                            self.refresh_characters();
                        }
                    }
                    self.confirm_delete = false;
                    return ScreenState::Continue;
                }

                // No button area (right side of button text)
                let no_x = box_x + 250.0;
                if point_in_rect(mx, my, no_x, yes_y, 100.0, 30.0) {
                    self.confirm_delete = false;
                    return ScreenState::Continue;
                }
            }

            return ScreenState::Continue;
        }

        // Mouse: Click on character rows
        if clicked && !self.characters.is_empty() {
            for i in 0..self.characters.len() {
                let y = list_y + i as f32 * item_height;
                if point_in_rect(mx, my, list_x, y, 500.0, item_height - 5.0) {
                    if self.selected_index == i {
                        // Double-click effect: if already selected, start game
                        let character = &self.characters[self.selected_index];
                        return ScreenState::StartGame {
                            session: self.session.clone(),
                            character_id: character.id,
                            character_name: character.name.clone(),
                        };
                    } else {
                        self.selected_index = i;
                    }
                    break;
                }
            }
        }

        // Mouse: Click on buttons at bottom
        if clicked {
            // Play button
            if point_in_rect(mx, my, list_x, inst_y - 10.0, 100.0, 30.0) {
                if !self.characters.is_empty() {
                    let character = &self.characters[self.selected_index];
                    return ScreenState::StartGame {
                        session: self.session.clone(),
                        character_id: character.id,
                        character_name: character.name.clone(),
                    };
                }
            }

            // New button
            if self.characters.len() < MAX_CHARACTERS {
                if point_in_rect(mx, my, list_x + 120.0, inst_y - 10.0, 70.0, 30.0) {
                    return ScreenState::ToCharacterCreate(self.session.clone());
                }
            }

            // Delete button
            if point_in_rect(mx, my, list_x + 210.0, inst_y - 10.0, 90.0, 30.0) {
                if !self.characters.is_empty() {
                    self.confirm_delete = true;
                }
            }

            // Logout button
            if point_in_rect(mx, my, list_x + 330.0, inst_y - 10.0, 100.0, 30.0) {
                let _ = self.auth_client.logout(&self.session.token);
                return ScreenState::ToLogin;
            }
        }

        // Keyboard: Navigate characters
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            }
        }
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
            if self.selected_index < self.characters.len().saturating_sub(1) {
                self.selected_index += 1;
            }
        }

        // Keyboard: Create new character
        if is_key_pressed(KeyCode::N) && self.characters.len() < MAX_CHARACTERS {
            return ScreenState::ToCharacterCreate(self.session.clone());
        }

        // Keyboard: Delete character
        if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::X) {
            if !self.characters.is_empty() {
                self.confirm_delete = true;
            }
        }

        // Keyboard: Logout
        if is_key_pressed(KeyCode::Escape) {
            let _ = self.auth_client.logout(&self.session.token);
            return ScreenState::ToLogin;
        }

        // Keyboard: Select character and start game
        if is_key_pressed(KeyCode::Enter) {
            if !self.characters.is_empty() {
                let character = &self.characters[self.selected_index];
                return ScreenState::StartGame {
                    session: self.session.clone(),
                    character_id: character.id,
                    character_name: character.name.clone(),
                };
            }
        }

        ScreenState::Continue
    }

    fn render(&self) {
        let (sw, sh) = virtual_screen_size();
        let (input_pos, _, _) = get_input_state();
        let mx = input_pos.x;
        let my = input_pos.y;

        // Background
        clear_background(Color::from_rgba(25, 25, 35, 255));

        // Draw decorative elements
        for i in 0..15 {
            let alpha = 0.03 + (i as f32 * 0.005);
            let color = Color::new(0.2, 0.3, 0.4, alpha);
            draw_line(0.0, i as f32 * 50.0, sw, i as f32 * 50.0, 1.0, color);
        }

        // Title (aligned vertically with account info, horizontally centered)
        let title = "SELECT CHARACTER";
        let title_width = self.measure_text_sharp(title, 16.0).width;
        self.draw_text_sharp(title, (sw - title_width) / 2.0, 24.0, 16.0, WHITE);

        // Account info
        let account_text = format!("Logged in as: {}", self.session.username);
        self.draw_text_sharp(&account_text, 20.0, 24.0, 16.0, LIGHTGRAY);

        // Character list (directly below title with slight padding)
        let list_x = (sw - 500.0) / 2.0;
        let list_y = 44.0;
        let item_height = 70.0;

        if self.characters.is_empty() {
            self.draw_text_sharp("No characters yet!", list_x, list_y + 30.0, 16.0, GRAY);
            self.draw_text_sharp("Press [N] to create your first character", list_x, list_y + 55.0, 16.0, LIGHTGRAY);
        } else {
            for (i, character) in self.characters.iter().enumerate() {
                let y = list_y + i as f32 * item_height;
                let is_selected = i == self.selected_index;
                let is_hovered = point_in_rect(mx, my, list_x, y, 500.0, item_height - 5.0);

                // Background
                let bg_color = if is_selected {
                    Color::from_rgba(60, 80, 120, 255)
                } else if is_hovered {
                    Color::from_rgba(50, 55, 80, 255)
                } else {
                    Color::from_rgba(40, 40, 60, 255)
                };
                draw_rectangle(list_x, y, 500.0, item_height - 5.0, bg_color);

                if is_selected {
                    draw_rectangle_lines(list_x, y, 500.0, item_height - 5.0, 2.0, WHITE);
                } else if is_hovered {
                    draw_rectangle_lines(list_x, y, 500.0, item_height - 5.0, 1.0, GRAY);
                }

                // Character preview sprite (scale to fit in the row)
                let preview_scale = 1.0;
                let preview_h = SPRITE_HEIGHT * preview_scale;
                let preview_y = y + (item_height - 5.0 - preview_h) / 2.0;
                draw_character_preview(
                    &self.player_sprites,
                    &self.hair_sprites,
                    &character.gender,
                    &character.skin,
                    character.hair_style,
                    character.hair_color.unwrap_or(0),
                    list_x + 10.0,
                    preview_y,
                    preview_scale,
                );

                // Character info (shifted right to make room for preview)
                let text_x = list_x + 50.0;
                self.draw_text_sharp(&character.name, text_x, y + 26.0, 16.0, WHITE);
                let class_info = format!("Level {} {} {}", character.level, character.gender, character.skin);
                self.draw_text_sharp(&class_info, text_x, y + 48.0, 16.0, LIGHTGRAY);

                // Played time
                let hours = character.played_time / 3600;
                let minutes = (character.played_time % 3600) / 60;
                let time_str = if hours > 0 {
                    format!("{}h {}m played", hours, minutes)
                } else {
                    format!("{}m played", minutes)
                };
                self.draw_text_sharp(&time_str, list_x + 340.0, y + 36.0, 16.0, DARKGRAY);
            }
        }

        // Error message
        if let Some(ref error) = self.error_message {
            let error_y = sh - 130.0;
            self.draw_text_sharp(error, list_x, error_y, 16.0, RED);
        }

        // Delete confirmation overlay
        if self.confirm_delete {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.7));

            let box_w = 450.0;
            let box_h = 150.0;
            let box_x = (sw - box_w) / 2.0;
            let box_y = (sh - box_h) / 2.0;

            draw_rectangle(box_x, box_y, box_w, box_h, Color::from_rgba(60, 40, 40, 255));
            draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, RED);

            if !self.characters.is_empty() {
                let char_name = &self.characters[self.selected_index].name;
                let delete_text = format!("Delete '{}'?", char_name);
                let delete_width = self.measure_text_sharp(&delete_text, 16.0).width;
                self.draw_text_sharp(&delete_text, box_x + (box_w - delete_width) / 2.0, box_y + 50.0, 16.0, WHITE);
            }

            // Touch-friendly Yes/No buttons
            let button_w = 100.0;
            let button_h = 30.0;
            let yes_x = box_x + 70.0;
            let yes_y = box_y + 85.0;
            let no_x = box_x + 250.0;

            // Yes button
            let yes_hovered = point_in_rect(mx, my, yes_x, yes_y, button_w, button_h);
            let yes_bg = if yes_hovered {
                Color::from_rgba(140, 60, 60, 255)
            } else {
                Color::from_rgba(100, 40, 40, 255)
            };
            let yes_border = if yes_hovered {
                Color::from_rgba(255, 100, 100, 255)
            } else {
                RED
            };
            draw_rectangle(yes_x, yes_y, button_w, button_h, yes_bg);
            draw_rectangle_lines(yes_x, yes_y, button_w, button_h, 2.0, yes_border);
            self.draw_text_sharp("Yes, delete", yes_x + 8.0, yes_y + 20.0, 16.0, WHITE);

            // No button
            let no_hovered = point_in_rect(mx, my, no_x, yes_y, button_w, button_h);
            let no_bg = if no_hovered {
                Color::from_rgba(80, 80, 110, 255)
            } else {
                Color::from_rgba(60, 60, 80, 255)
            };
            let no_border = if no_hovered {
                WHITE
            } else {
                LIGHTGRAY
            };
            draw_rectangle(no_x, yes_y, button_w, button_h, no_bg);
            draw_rectangle_lines(no_x, yes_y, button_w, button_h, 2.0, no_border);
            self.draw_text_sharp("No, cancel", no_x + 8.0, yes_y + 20.0, 16.0, WHITE);
            return;
        }

        // Buttons at bottom
        let inst_y = sh - 70.0;
        let button_height = 30.0;

        // Play button
        let play_hovered = point_in_rect(mx, my, list_x, inst_y - 10.0, 100.0, button_height);
        let play_bg = if play_hovered {
            Color::from_rgba(60, 140, 90, 255)
        } else {
            Color::from_rgba(40, 100, 60, 255)
        };
        let play_border = if play_hovered {
            Color::from_rgba(100, 255, 150, 255)
        } else {
            GREEN
        };
        draw_rectangle(list_x, inst_y - 10.0, 100.0, button_height, play_bg);
        draw_rectangle_lines(list_x, inst_y - 10.0, 100.0, button_height, 2.0, play_border);
        self.draw_text_sharp("Play", list_x + 10.0, inst_y + 10.0, 16.0, WHITE);

        // New button
        if self.characters.len() < MAX_CHARACTERS {
            let new_hovered = point_in_rect(mx, my, list_x + 120.0, inst_y - 10.0, 70.0, button_height);
            let new_bg = if new_hovered {
                Color::from_rgba(120, 120, 60, 255)
            } else {
                Color::from_rgba(80, 80, 40, 255)
            };
            let new_border = if new_hovered {
                Color::from_rgba(255, 255, 100, 255)
            } else {
                YELLOW
            };
            draw_rectangle(list_x + 120.0, inst_y - 10.0, 70.0, button_height, new_bg);
            draw_rectangle_lines(list_x + 120.0, inst_y - 10.0, 70.0, button_height, 2.0, new_border);
            self.draw_text_sharp("New", list_x + 130.0, inst_y + 10.0, 16.0, WHITE);
        }

        // Delete button
        let delete_hovered = point_in_rect(mx, my, list_x + 210.0, inst_y - 10.0, 90.0, button_height);
        let delete_bg = if delete_hovered {
            Color::from_rgba(140, 60, 60, 255)
        } else {
            Color::from_rgba(100, 40, 40, 255)
        };
        let delete_border = if delete_hovered {
            Color::from_rgba(255, 100, 100, 255)
        } else {
            RED
        };
        draw_rectangle(list_x + 210.0, inst_y - 10.0, 90.0, button_height, delete_bg);
        draw_rectangle_lines(list_x + 210.0, inst_y - 10.0, 90.0, button_height, 2.0, delete_border);
        self.draw_text_sharp("Delete", list_x + 220.0, inst_y + 10.0, 16.0, WHITE);

        // Logout button
        let logout_hovered = point_in_rect(mx, my, list_x + 330.0, inst_y - 10.0, 100.0, button_height);
        let logout_bg = if logout_hovered {
            Color::from_rgba(80, 80, 110, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        let logout_border = if logout_hovered {
            WHITE
        } else {
            LIGHTGRAY
        };
        draw_rectangle(list_x + 330.0, inst_y - 10.0, 100.0, button_height, logout_bg);
        draw_rectangle_lines(list_x + 330.0, inst_y - 10.0, 100.0, button_height, 2.0, logout_border);
        self.draw_text_sharp("Logout", list_x + 340.0, inst_y + 10.0, 16.0, WHITE);

        self.draw_text_sharp("[W/S] Navigate", list_x, inst_y + 28.0, 16.0, DARKGRAY);
    }
}

// ============================================================================
// Character Create Screen
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
const GENDERS: [&str; 2] = ["male", "female"];

#[cfg(not(target_arch = "wasm32"))]
const SKINS: [&str; 7] = ["tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"];

#[cfg(not(target_arch = "wasm32"))]
pub struct CharacterCreateScreen {
    session: AuthSession,
    name: String,
    gender_index: usize,
    skin_index: usize,
    hair_style_index: Option<usize>,  // None = bald, Some(0-2) = style
    hair_color_index: usize,          // 0-6
    error_message: Option<String>,
    auth_client: AuthClient,
    font: BitmapFont,
    active_field: CreateField,
    player_sprites: HashMap<String, Texture2D>,
    hair_sprites: HashMap<i32, Texture2D>,
}

const HAIR_STYLES: usize = 3;  // 0, 1, 2
const HAIR_COLORS: usize = 10; // 0-9 (20 frames / 2 front-back pairs)

#[cfg(not(target_arch = "wasm32"))]
#[derive(PartialEq, Clone, Copy)]
enum CreateField {
    Name,
    Gender,
    Skin,
    HairStyle,
    HairColor,
}

#[cfg(not(target_arch = "wasm32"))]
impl CharacterCreateScreen {
    pub fn new(session: AuthSession, server_url: &str) -> Self {
        Self {
            session,
            name: String::new(),
            gender_index: 0,
            skin_index: 0,
            hair_style_index: None,
            hair_color_index: 0,
            error_message: None,
            auth_client: AuthClient::new(server_url),
            font: BitmapFont::default(),
            active_field: CreateField::Name,
            player_sprites: HashMap::new(),
            hair_sprites: HashMap::new(),
        }
    }

    pub async fn load_font(&mut self) {
        self.font = BitmapFont::load_or_default("assets/fonts/monogram/ttf/monogram-extended.ttf").await;
        self.player_sprites = load_player_sprites().await;
        // Load hair sprites
        for style in 0..HAIR_STYLES as i32 {
            let path = asset_path(&format!("assets/sprites/hair/hair_{}.png", style));
            match load_texture(&path).await {
                Ok(tex) => {
                    tex.set_filter(FilterMode::Nearest);
                    self.hair_sprites.insert(style, tex);
                }
                Err(e) => {
                    log::warn!("Failed to load hair sprite {}: {}", path, e);
                }
            }
        }
    }

    fn draw_text_sharp(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        self.font.draw_text(text, x, y, font_size, color);
    }

    fn measure_text_sharp(&self, text: &str, font_size: f32) -> TextDimensions {
        self.font.measure_text(text, font_size)
    }

    fn handle_name_input(&mut self) {
        while let Some(c) = get_char_pressed() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                if self.name.len() < 16 {
                    self.name.push(c);
                    self.error_message = None;
                }
            }
        }

        if is_key_pressed(KeyCode::Backspace) {
            self.name.pop();
            self.error_message = None;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Screen for CharacterCreateScreen {
    fn update(&mut self, audio: &AudioManager) -> ScreenState {
        let (sw, _sh) = virtual_screen_size();
        let (input_pos, clicked, _is_touch) = get_input_state();
        let mx = input_pos.x;
        let my = input_pos.y;

        // Layout constants (must match render)
        let total_width = 460.0;
        let content_x = (sw - total_width) / 2.0;
        let content_y = 70.0;
        let preview_w = 140.0;
        let form_x = content_x + preview_w + 20.0;
        let form_w = 300.0;
        let field_height = 70.0;
        let half_width = (form_w - 10.0) / 2.0;

        // Handle name input when name field is active
        if self.active_field == CreateField::Name {
            self.handle_name_input();
        }

        // Mouse: Click on fields to focus
        if clicked {
            // Name field box
            let name_box_y = content_y + 20.0;
            if point_in_rect(mx, my, form_x, name_box_y, form_w, 36.0) {
                self.active_field = CreateField::Name;
                show_keyboard(true);
            }

            // Gender field box
            let gender_box_y = content_y + field_height + 20.0;
            if point_in_rect(mx, my, form_x, gender_box_y, form_w, 36.0) {
                self.active_field = CreateField::Gender;
                show_keyboard(false);

                // Check if clicked on left arrow area
                if point_in_rect(mx, my, form_x, gender_box_y, 50.0, 36.0) {
                    self.gender_index = if self.gender_index == 0 {
                        GENDERS.len() - 1
                    } else {
                        self.gender_index - 1
                    };
                }
                // Check if clicked on right arrow area
                if point_in_rect(mx, my, form_x + form_w - 50.0, gender_box_y, 50.0, 36.0) {
                    self.gender_index = (self.gender_index + 1) % GENDERS.len();
                }
            }

            // Skin field box
            let skin_box_y = content_y + field_height * 2.0 + 20.0;
            if point_in_rect(mx, my, form_x, skin_box_y, form_w, 36.0) {
                self.active_field = CreateField::Skin;
                show_keyboard(false);

                // Check if clicked on left arrow area
                if point_in_rect(mx, my, form_x, skin_box_y, 50.0, 36.0) {
                    self.skin_index = if self.skin_index == 0 {
                        SKINS.len() - 1
                    } else {
                        self.skin_index - 1
                    };
                }
                // Check if clicked on right arrow area
                if point_in_rect(mx, my, form_x + form_w - 50.0, skin_box_y, 50.0, 36.0) {
                    self.skin_index = (self.skin_index + 1) % SKINS.len();
                }
            }

            // Hair style field box (left half of hair row)
            let hair_box_y = content_y + field_height * 3.0 + 20.0;
            if point_in_rect(mx, my, form_x, hair_box_y, half_width, 36.0) {
                self.active_field = CreateField::HairStyle;
                show_keyboard(false);

                // Check if clicked on left arrow area
                if point_in_rect(mx, my, form_x, hair_box_y, 35.0, 36.0) {
                    self.hair_style_index = match self.hair_style_index {
                        None => Some(HAIR_STYLES - 1),
                        Some(0) => None,
                        Some(i) => Some(i - 1),
                    };
                }
                // Check if clicked on right arrow area
                if point_in_rect(mx, my, form_x + half_width - 35.0, hair_box_y, 35.0, 36.0) {
                    self.hair_style_index = match self.hair_style_index {
                        None => Some(0),
                        Some(i) if i >= HAIR_STYLES - 1 => None,
                        Some(i) => Some(i + 1),
                    };
                }
            }

            // Hair color field box (right half of hair row, only if hair style selected)
            let hair_color_x = form_x + half_width + 10.0;
            if self.hair_style_index.is_some() && point_in_rect(mx, my, hair_color_x, hair_box_y, half_width, 36.0) {
                self.active_field = CreateField::HairColor;
                show_keyboard(false);

                // Check if clicked on left arrow area
                if point_in_rect(mx, my, hair_color_x, hair_box_y, 35.0, 36.0) {
                    self.hair_color_index = if self.hair_color_index == 0 {
                        HAIR_COLORS - 1
                    } else {
                        self.hair_color_index - 1
                    };
                }
                // Check if clicked on right arrow area
                if point_in_rect(mx, my, hair_color_x + half_width - 35.0, hair_box_y, 35.0, 36.0) {
                    self.hair_color_index = (self.hair_color_index + 1) % HAIR_COLORS;
                }
            }

            // Buttons row
            let buttons_y = content_y + field_height * 4.0 + 10.0;
            let button_w = (form_w - 10.0) / 2.0;

            // Create button
            if point_in_rect(mx, my, form_x, buttons_y, button_w, 36.0) {
                show_keyboard(false);
                let name = self.name.trim();
                if name.len() < 2 {
                    self.error_message = Some("Name must be at least 2 characters".to_string());
                    return ScreenState::Continue;
                }
                if name.len() > 16 {
                    self.error_message = Some("Name must be at most 16 characters".to_string());
                    return ScreenState::Continue;
                }

                let gender = GENDERS[self.gender_index];
                let skin = SKINS[self.skin_index];
                let hair_style = self.hair_style_index.map(|i| i as i32);
                let hair_color = if self.hair_style_index.is_some() { Some(self.hair_color_index as i32) } else { None };

                match self.auth_client.create_character(&self.session.token, name, gender, skin, hair_style, hair_color) {
                    Ok(_) => {
                        return ScreenState::ToCharacterSelect(self.session.clone());
                    }
                    Err(e) => {
                        self.error_message = Some(e.to_string());
                    }
                }
            }

            // Cancel button
            let cancel_x = form_x + button_w + 10.0;
            if point_in_rect(mx, my, cancel_x, buttons_y, button_w, 36.0) {
                show_keyboard(false);
                return ScreenState::ToCharacterSelect(self.session.clone());
            }
        }

        // Keyboard: Navigate between fields with Tab or Up/Down
        if is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::Down) {
            audio.play_sfx("enter");
            self.active_field = match self.active_field {
                CreateField::Name => CreateField::Gender,
                CreateField::Gender => CreateField::Skin,
                CreateField::Skin => CreateField::HairStyle,
                CreateField::HairStyle => {
                    if self.hair_style_index.is_some() {
                        CreateField::HairColor
                    } else {
                        CreateField::Name
                    }
                }
                CreateField::HairColor => CreateField::Name,
            };
        }
        if is_key_pressed(KeyCode::Up) {
            audio.play_sfx("enter");
            self.active_field = match self.active_field {
                CreateField::Name => {
                    if self.hair_style_index.is_some() {
                        CreateField::HairColor
                    } else {
                        CreateField::HairStyle
                    }
                }
                CreateField::Gender => CreateField::Name,
                CreateField::Skin => CreateField::Gender,
                CreateField::HairStyle => CreateField::Skin,
                CreateField::HairColor => CreateField::HairStyle,
            };
        }

        // Keyboard: Left/Right or A/D to cycle options
        if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
            match self.active_field {
                CreateField::Gender => {
                    self.gender_index = if self.gender_index == 0 {
                        GENDERS.len() - 1
                    } else {
                        self.gender_index - 1
                    };
                }
                CreateField::Skin => {
                    self.skin_index = if self.skin_index == 0 {
                        SKINS.len() - 1
                    } else {
                        self.skin_index - 1
                    };
                }
                CreateField::HairStyle => {
                    self.hair_style_index = match self.hair_style_index {
                        None => Some(HAIR_STYLES - 1),
                        Some(0) => None,
                        Some(i) => Some(i - 1),
                    };
                }
                CreateField::HairColor => {
                    self.hair_color_index = if self.hair_color_index == 0 {
                        HAIR_COLORS - 1
                    } else {
                        self.hair_color_index - 1
                    };
                }
                _ => {}
            }
        }
        if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
            match self.active_field {
                CreateField::Gender => {
                    self.gender_index = (self.gender_index + 1) % GENDERS.len();
                }
                CreateField::Skin => {
                    self.skin_index = (self.skin_index + 1) % SKINS.len();
                }
                CreateField::HairStyle => {
                    self.hair_style_index = match self.hair_style_index {
                        None => Some(0),
                        Some(i) if i >= HAIR_STYLES - 1 => None,
                        Some(i) => Some(i + 1),
                    };
                }
                CreateField::HairColor => {
                    self.hair_color_index = (self.hair_color_index + 1) % HAIR_COLORS;
                }
                _ => {}
            }
        }

        // Keyboard: Cancel
        if is_key_pressed(KeyCode::Escape) {
            show_keyboard(false);
            return ScreenState::ToCharacterSelect(self.session.clone());
        }

        // Keyboard: Create character
        if is_key_pressed(KeyCode::Enter) {
            audio.play_sfx("enter");
            let name = self.name.trim();
            if name.len() < 2 {
                self.error_message = Some("Name must be at least 2 characters".to_string());
                return ScreenState::Continue;
            }
            if name.len() > 16 {
                self.error_message = Some("Name must be at most 16 characters".to_string());
                return ScreenState::Continue;
            }

            let gender = GENDERS[self.gender_index];
            let skin = SKINS[self.skin_index];
            let hair_style = self.hair_style_index.map(|i| i as i32);
            let hair_color = if self.hair_style_index.is_some() { Some(self.hair_color_index as i32) } else { None };

            match self.auth_client.create_character(&self.session.token, name, gender, skin, hair_style, hair_color) {
                Ok(_) => {
                    return ScreenState::ToCharacterSelect(self.session.clone());
                }
                Err(e) => {
                    self.error_message = Some(e.to_string());
                }
            }
        }

        ScreenState::Continue
    }

    fn render(&self) {
        let (sw, sh) = virtual_screen_size();
        let (input_pos, _, _) = get_input_state();
        let mx = input_pos.x;
        let my = input_pos.y;

        // Background
        clear_background(Color::from_rgba(25, 25, 35, 255));

        // Draw decorative elements
        for i in 0..15 {
            let alpha = 0.03 + (i as f32 * 0.005);
            let color = Color::new(0.2, 0.3, 0.4, alpha);
            draw_line(0.0, i as f32 * 50.0, sw, i as f32 * 50.0, 1.0, color);
        }

        // Title
        let title = "CREATE CHARACTER";
        let title_width = self.measure_text_sharp(title, 16.0).width;
        self.draw_text_sharp(title, (sw - title_width) / 2.0, 30.0, 16.0, WHITE);

        // Layout: Preview on left, form on right
        let total_width = 460.0;  // Preview (140) + gap (20) + form (300)
        let content_x = (sw - total_width) / 2.0;
        let content_y = 70.0;

        // === LEFT SIDE: Character Preview ===
        let preview_w = 140.0;
        let preview_h = 200.0;

        // Decorative frame around preview
        let frame_padding = 8.0;
        let frame_x = content_x - frame_padding;
        let frame_y = content_y - frame_padding;
        let frame_w = preview_w + frame_padding * 2.0;
        let frame_h = preview_h + frame_padding * 2.0;

        // Outer glow
        draw_rectangle(frame_x - 2.0, frame_y - 2.0, frame_w + 4.0, frame_h + 4.0, Color::from_rgba(60, 80, 120, 100));
        // Frame background
        draw_rectangle(frame_x, frame_y, frame_w, frame_h, Color::from_rgba(40, 45, 60, 255));
        // Inner preview area
        draw_rectangle(content_x, content_y, preview_w, preview_h, Color::from_rgba(20, 22, 30, 255));
        // Frame border
        draw_rectangle_lines(frame_x, frame_y, frame_w, frame_h, 2.0, Color::from_rgba(80, 100, 140, 255));
        // Corner accents
        let accent_size = 8.0;
        let accent_color = Color::from_rgba(100, 140, 200, 255);
        draw_line(frame_x, frame_y + accent_size, frame_x + accent_size, frame_y, 2.0, accent_color);
        draw_line(frame_x + frame_w - accent_size, frame_y, frame_x + frame_w, frame_y + accent_size, 2.0, accent_color);
        draw_line(frame_x, frame_y + frame_h - accent_size, frame_x + accent_size, frame_y + frame_h, 2.0, accent_color);
        draw_line(frame_x + frame_w - accent_size, frame_y + frame_h, frame_x + frame_w, frame_y + frame_h - accent_size, 2.0, accent_color);

        // Draw character sprite preview (scaled up for visibility)
        let preview_scale = 1.5;
        let sprite_w = SPRITE_WIDTH * preview_scale;
        let sprite_h = SPRITE_HEIGHT * preview_scale;
        let sprite_x = content_x + (preview_w - sprite_w) / 2.0;
        let sprite_y = content_y + (preview_h - sprite_h) / 2.0 - 10.0;
        draw_character_preview(
            &self.player_sprites,
            &self.hair_sprites,
            GENDERS[self.gender_index],
            SKINS[self.skin_index],
            self.hair_style_index.map(|i| i as i32),
            self.hair_color_index as i32,
            sprite_x,
            sprite_y,
            preview_scale,
        );

        // Preview label below character
        let preview_label = format!("{} {}", GENDERS[self.gender_index], SKINS[self.skin_index]);
        let label_width = self.measure_text_sharp(&preview_label, 16.0).width;
        self.draw_text_sharp(&preview_label, content_x + (preview_w - label_width) / 2.0, content_y + preview_h - 30.0, 16.0, LIGHTGRAY);

        // === RIGHT SIDE: Form Fields ===
        let form_x = content_x + preview_w + 20.0;
        let form_w = 300.0;
        let field_height = 70.0;

        // Name field
        let name_active = self.active_field == CreateField::Name;
        let name_y = content_y;
        self.draw_text_sharp("Name", form_x, name_y, 16.0, if name_active { WHITE } else { GRAY });

        let name_box_color = if name_active {
            Color::from_rgba(80, 120, 180, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(form_x, name_y + 20.0, form_w, 36.0, name_box_color);
        draw_rectangle_lines(form_x, name_y + 20.0, form_w, 36.0, 2.0, if name_active { WHITE } else { GRAY });

        let cursor = if name_active && (get_time() * 2.0) as i32 % 2 == 0 { "|" } else { "" };
        let name_display = if self.name.is_empty() && !name_active {
            "Enter name...".to_string()
        } else {
            format!("{}{}", self.name, cursor)
        };
        let text_color = if self.name.is_empty() && !name_active { DARKGRAY } else { WHITE };
        self.draw_text_sharp(&name_display, form_x + 10.0, name_y + 44.0, 16.0, text_color);

        // Gender field
        let gender_active = self.active_field == CreateField::Gender;
        let gender_y = content_y + field_height;
        self.draw_text_sharp("Gender", form_x, gender_y, 16.0, if gender_active { WHITE } else { GRAY });

        let gender_box_color = if gender_active {
            Color::from_rgba(80, 120, 180, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(form_x, gender_y + 20.0, form_w, 36.0, gender_box_color);
        draw_rectangle_lines(form_x, gender_y + 20.0, form_w, 36.0, 2.0, if gender_active { WHITE } else { GRAY });

        self.draw_text_sharp("<", form_x + 15.0, gender_y + 44.0, 16.0, if gender_active { YELLOW } else { DARKGRAY });
        let gender_text = GENDERS[self.gender_index];
        let gender_width = self.measure_text_sharp(gender_text, 16.0).width;
        self.draw_text_sharp(gender_text, form_x + form_w / 2.0 - gender_width / 2.0, gender_y + 44.0, 16.0, WHITE);
        self.draw_text_sharp(">", form_x + form_w - 25.0, gender_y + 44.0, 16.0, if gender_active { YELLOW } else { DARKGRAY });

        // Skin field
        let skin_active = self.active_field == CreateField::Skin;
        let skin_y = content_y + field_height * 2.0;
        self.draw_text_sharp("Skin", form_x, skin_y, 16.0, if skin_active { WHITE } else { GRAY });

        let skin_box_color = if skin_active {
            Color::from_rgba(80, 120, 180, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(form_x, skin_y + 20.0, form_w, 36.0, skin_box_color);
        draw_rectangle_lines(form_x, skin_y + 20.0, form_w, 36.0, 2.0, if skin_active { WHITE } else { GRAY });

        self.draw_text_sharp("<", form_x + 15.0, skin_y + 44.0, 16.0, if skin_active { YELLOW } else { DARKGRAY });
        let skin_text = SKINS[self.skin_index];
        let skin_width = self.measure_text_sharp(skin_text, 16.0).width;
        self.draw_text_sharp(skin_text, form_x + form_w / 2.0 - skin_width / 2.0, skin_y + 44.0, 16.0, WHITE);
        self.draw_text_sharp(">", form_x + form_w - 25.0, skin_y + 44.0, 16.0, if skin_active { YELLOW } else { DARKGRAY });

        // Hair row: Style and Color side by side
        let hair_y = content_y + field_height * 3.0;
        let half_width = (form_w - 10.0) / 2.0;  // 10px gap between

        // Hair Style (left half)
        let hair_style_active = self.active_field == CreateField::HairStyle;
        self.draw_text_sharp("Style", form_x, hair_y, 16.0, if hair_style_active { WHITE } else { GRAY });

        let hair_style_box_color = if hair_style_active {
            Color::from_rgba(80, 120, 180, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(form_x, hair_y + 20.0, half_width, 36.0, hair_style_box_color);
        draw_rectangle_lines(form_x, hair_y + 20.0, half_width, 36.0, 2.0, if hair_style_active { WHITE } else { GRAY });

        self.draw_text_sharp("<", form_x + 10.0, hair_y + 44.0, 16.0, if hair_style_active { YELLOW } else { DARKGRAY });
        let hair_style_text = match self.hair_style_index {
            None => "Bald",
            Some(0) => "Style 1",
            Some(1) => "Style 2",
            Some(2) => "Style 3",
            _ => "?",
        };
        let hair_style_width = self.measure_text_sharp(hair_style_text, 16.0).width;
        self.draw_text_sharp(hair_style_text, form_x + half_width / 2.0 - hair_style_width / 2.0, hair_y + 44.0, 16.0, WHITE);
        self.draw_text_sharp(">", form_x + half_width - 20.0, hair_y + 44.0, 16.0, if hair_style_active { YELLOW } else { DARKGRAY });

        // Hair Color (right half) - only enabled if hair style selected
        let hair_color_x = form_x + half_width + 10.0;
        let hair_color_active = self.active_field == CreateField::HairColor;
        let has_hair = self.hair_style_index.is_some();

        self.draw_text_sharp("Color", hair_color_x, hair_y, 16.0, if has_hair { if hair_color_active { WHITE } else { GRAY } } else { DARKGRAY });

        let hair_color_box_color = if !has_hair {
            Color::from_rgba(40, 40, 50, 255)  // Disabled look
        } else if hair_color_active {
            Color::from_rgba(80, 120, 180, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(hair_color_x, hair_y + 20.0, half_width, 36.0, hair_color_box_color);
        draw_rectangle_lines(hair_color_x, hair_y + 20.0, half_width, 36.0, 2.0, if has_hair { if hair_color_active { WHITE } else { GRAY } } else { DARKGRAY });

        if has_hair {
            self.draw_text_sharp("<", hair_color_x + 10.0, hair_y + 44.0, 16.0, if hair_color_active { YELLOW } else { DARKGRAY });
            let hair_color_text = format!("{}", self.hair_color_index + 1);
            let hair_color_width = self.measure_text_sharp(&hair_color_text, 16.0).width;
            self.draw_text_sharp(&hair_color_text, hair_color_x + half_width / 2.0 - hair_color_width / 2.0, hair_y + 44.0, 16.0, WHITE);
            self.draw_text_sharp(">", hair_color_x + half_width - 20.0, hair_y + 44.0, 16.0, if hair_color_active { YELLOW } else { DARKGRAY });
        } else {
            let dash_width = self.measure_text_sharp("-", 16.0).width;
            self.draw_text_sharp("-", hair_color_x + half_width / 2.0 - dash_width / 2.0, hair_y + 44.0, 16.0, DARKGRAY);
        }

        // Buttons row
        let buttons_y = content_y + field_height * 4.0 + 10.0;
        let button_w = (form_w - 10.0) / 2.0;

        // Create button
        let create_hovered = point_in_rect(mx, my, form_x, buttons_y, button_w, 36.0);
        let create_bg = if create_hovered {
            Color::from_rgba(60, 140, 90, 255)
        } else {
            Color::from_rgba(40, 100, 60, 255)
        };
        let create_border = if create_hovered {
            Color::from_rgba(100, 200, 120, 255)
        } else {
            Color::from_rgba(60, 140, 80, 255)
        };
        draw_rectangle(form_x, buttons_y, button_w, 36.0, create_bg);
        draw_rectangle_lines(form_x, buttons_y, button_w, 36.0, 2.0, create_border);
        let create_text = "Create";
        let create_width = self.measure_text_sharp(create_text, 16.0).width;
        self.draw_text_sharp(create_text, form_x + button_w / 2.0 - create_width / 2.0, buttons_y + 24.0, 16.0, WHITE);

        // Cancel button
        let cancel_x = form_x + button_w + 10.0;
        let cancel_hovered = point_in_rect(mx, my, cancel_x, buttons_y, button_w, 36.0);
        let cancel_bg = if cancel_hovered {
            Color::from_rgba(120, 80, 80, 255)
        } else {
            Color::from_rgba(80, 60, 60, 255)
        };
        let cancel_border = if cancel_hovered {
            Color::from_rgba(180, 120, 120, 255)
        } else {
            Color::from_rgba(120, 80, 80, 255)
        };
        draw_rectangle(cancel_x, buttons_y, button_w, 36.0, cancel_bg);
        draw_rectangle_lines(cancel_x, buttons_y, button_w, 36.0, 2.0, cancel_border);
        let cancel_text = "Cancel";
        let cancel_width = self.measure_text_sharp(cancel_text, 16.0).width;
        self.draw_text_sharp(cancel_text, cancel_x + button_w / 2.0 - cancel_width / 2.0, buttons_y + 24.0, 16.0, WHITE);

        // Error message
        if let Some(ref error) = self.error_message {
            let error_y = buttons_y + 50.0;
            let error_width = self.measure_text_sharp(error, 16.0).width;
            self.draw_text_sharp(error, form_x + (form_w - error_width) / 2.0, error_y, 16.0, RED);
        }

        // Keyboard hints at bottom
        let hints_y = sh - 30.0;
        self.draw_text_sharp("[Tab] Next field    [A/D] Change    [Enter] Create    [Esc] Cancel",
            (sw - 450.0) / 2.0, hints_y, 16.0, DARKGRAY);
    }
}
