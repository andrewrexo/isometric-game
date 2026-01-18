use macroquad::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use crate::auth::{AuthClient, AuthSession, CharacterInfo};
use crate::render::BitmapFont;

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
    fn update(&mut self) -> ScreenState;
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
    dev_mode: bool,
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
    pub fn new(server_url: &str, dev_mode: bool) -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            active_field: LoginField::Username,
            mode: LoginMode::Login,
            error_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            auth_client: AuthClient::new(server_url),
            dev_mode,
            font: BitmapFont::default(),
            logo: None,
        }
    }

    /// Load font and logo asynchronously - call this after creating the screen
    pub async fn load_font(&mut self) {
        self.font = BitmapFont::load_or_default("assets/fonts/monogram/ttf/monogram-extended.ttf").await;

        // Load logo texture
        if let Ok(texture) = load_texture("assets/ui/logo.png").await {
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

        // Tab to switch fields
        if is_key_pressed(KeyCode::Tab) {
            self.active_field = match self.active_field {
                LoginField::Username => LoginField::Password,
                LoginField::Password => LoginField::Username,
            };
        }
    }
}

impl Screen for LoginScreen {
    fn update(&mut self) -> ScreenState {
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

        // Guest login (dev mode only)
        if self.dev_mode && is_key_pressed(KeyCode::F2) {
            return ScreenState::StartGuestMode;
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
        let sw = screen_width();
        let sh = screen_height();

        // Background
        clear_background(Color::from_rgba(25, 25, 35, 255));

        // Draw decorative grid lines
        for i in 0..20 {
            let alpha = 0.05 + (i as f32 * 0.01);
            let color = Color::new(0.3, 0.4, 0.5, alpha);
            draw_line(0.0, i as f32 * 40.0, sw, i as f32 * 40.0, 1.0, color);
            draw_line(i as f32 * 70.0, 0.0, i as f32 * 70.0, sh, 1.0, color);
        }

        // Logo
        if let Some(logo) = &self.logo {
            let logo_scale = 0.25; // Scale down the logo
            let logo_w = logo.width() * logo_scale;
            let logo_h = logo.height() * logo_scale;
            let logo_x = (sw - logo_w) / 2.0;
            let logo_y = sh * 0.08;
            draw_texture_ex(
                logo,
                logo_x.floor(),
                logo_y.floor(),
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(logo_w.floor(), logo_h.floor())),
                    ..Default::default()
                },
            );
        } else {
            // Fallback to text if logo not loaded
            let title = "NEW AEVEN";
            let title_size = 32.0;
            let title_width = self.measure_text_sharp(title, title_size).width;
            self.draw_text_sharp(title, (sw - title_width) / 2.0, sh * 0.18, title_size, WHITE);
        }

        // Subtitle
        let subtitle = match self.mode {
            LoginMode::Login => "Login to start playing",
            LoginMode::Register => "Create new account",
        };
        let sub_size = 16.0;
        let sub_width = self.measure_text_sharp(subtitle, sub_size).width;
        self.draw_text_sharp(subtitle, (sw - sub_width) / 2.0, sh * 0.18 + 40.0, sub_size, GRAY);

        // Input box dimensions
        let box_width = 350.0;
        let box_height = 50.0;
        let box_x = (sw - box_width) / 2.0;
        let start_y = sh * 0.38;

        // Username field
        let username_active = self.active_field == LoginField::Username;
        let username_color = if username_active { Color::from_rgba(80, 120, 180, 255) } else { Color::from_rgba(60, 60, 80, 255) };
        draw_rectangle(box_x, start_y, box_width, box_height, username_color);
        draw_rectangle_lines(box_x, start_y, box_width, box_height, 2.0, if username_active { WHITE } else { GRAY });

        self.draw_text_sharp("Username", box_x, start_y - 8.0, 16.0, LIGHTGRAY);
        let username_display = if self.username.is_empty() && !username_active {
            "Enter username...".to_string()
        } else {
            let cursor = if username_active && (get_time() * 2.0) as i32 % 2 == 0 { "|" } else { "" };
            format!("{}{}", self.username, cursor)
        };
        let text_color = if self.username.is_empty() && !username_active { DARKGRAY } else { WHITE };
        self.draw_text_sharp(&username_display, box_x + 12.0, start_y + 30.0, 16.0, text_color);

        // Password field
        let password_y = start_y + 85.0;
        let password_active = self.active_field == LoginField::Password;
        let password_color = if password_active { Color::from_rgba(80, 120, 180, 255) } else { Color::from_rgba(60, 60, 80, 255) };
        draw_rectangle(box_x, password_y, box_width, box_height, password_color);
        draw_rectangle_lines(box_x, password_y, box_width, box_height, 2.0, if password_active { WHITE } else { GRAY });

        self.draw_text_sharp("Password", box_x, password_y - 8.0, 16.0, LIGHTGRAY);
        let password_display = if self.password.is_empty() && !password_active {
            "Enter password...".to_string()
        } else {
            let masked: String = "*".repeat(self.password.len());
            let cursor = if password_active && (get_time() * 2.0) as i32 % 2 == 0 { "|" } else { "" };
            format!("{}{}", masked, cursor)
        };
        let text_color = if self.password.is_empty() && !password_active { DARKGRAY } else { WHITE };
        self.draw_text_sharp(&password_display, box_x + 12.0, password_y + 30.0, 16.0, text_color);

        // Error message
        if let Some(ref error) = self.error_message {
            let error_y = password_y + 70.0;
            let error_width = self.measure_text_sharp(error, 16.0).width;
            self.draw_text_sharp(error, (sw - error_width) / 2.0, error_y, 16.0, RED);
        }

        // Instructions
        let inst_y = sh * 0.72;
        let inst_size = 16.0;

        let enter_text = match self.mode {
            LoginMode::Login => "[Enter] Login",
            LoginMode::Register => "[Enter] Register",
        };
        self.draw_text_sharp(enter_text, box_x, inst_y, inst_size, GREEN);

        let toggle_text = match self.mode {
            LoginMode::Login => "[F1] Switch to Register",
            LoginMode::Register => "[F1] Switch to Login",
        };
        self.draw_text_sharp(toggle_text, box_x, inst_y + 24.0, inst_size, YELLOW);

        self.draw_text_sharp("[Tab] Switch fields", box_x, inst_y + 48.0, inst_size, LIGHTGRAY);

        if self.dev_mode {
            self.draw_text_sharp("[F2] Guest Login (Dev Mode)", box_x, inst_y + 72.0, inst_size, ORANGE);
        }

        // Version
        self.draw_text_sharp("v0.1.0", sw - 60.0, sh - 20.0, 12.0, DARKGRAY);
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
        }
    }

    /// Load font asynchronously - call this after creating the screen
    pub async fn load_font(&mut self) {
        self.font = BitmapFont::load_or_default("assets/fonts/monogram/ttf/monogram-extended.ttf").await;
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
    fn update(&mut self) -> ScreenState {
        // Delete confirmation mode
        if self.confirm_delete {
            if is_key_pressed(KeyCode::Y) {
                // Confirm delete
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
            return ScreenState::Continue;
        }

        // Navigate characters
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

        // Create new character - go to character creation screen
        if is_key_pressed(KeyCode::N) && self.characters.len() < MAX_CHARACTERS {
            return ScreenState::ToCharacterCreate(self.session.clone());
        }

        // Delete character - ask for confirmation
        if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::X) {
            if !self.characters.is_empty() {
                self.confirm_delete = true;
            }
        }

        // Logout
        if is_key_pressed(KeyCode::Escape) {
            let _ = self.auth_client.logout(&self.session.token);
            return ScreenState::ToLogin;
        }

        // Select character and start game
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
        let sw = screen_width();
        let sh = screen_height();

        // Background
        clear_background(Color::from_rgba(25, 25, 35, 255));

        // Draw decorative elements
        for i in 0..15 {
            let alpha = 0.03 + (i as f32 * 0.005);
            let color = Color::new(0.2, 0.3, 0.4, alpha);
            draw_line(0.0, i as f32 * 50.0, sw, i as f32 * 50.0, 1.0, color);
        }

        // Title
        let title = "SELECT CHARACTER";
        let title_width = self.measure_text_sharp(title, 16.0).width;
        self.draw_text_sharp(title, (sw - title_width) / 2.0, 50.0, 16.0, WHITE);

        // Account info
        let account_text = format!("Logged in as: {}", self.session.username);
        self.draw_text_sharp(&account_text, 20.0, 24.0, 16.0, LIGHTGRAY);

        // Character list
        let list_x = (sw - 500.0) / 2.0;
        let list_y = 100.0;
        let item_height = 70.0;

        if self.characters.is_empty() {
            self.draw_text_sharp("No characters yet!", list_x, list_y + 40.0, 16.0, GRAY);
            self.draw_text_sharp("Press [N] to create your first character", list_x, list_y + 70.0, 16.0, LIGHTGRAY);
        } else {
            for (i, character) in self.characters.iter().enumerate() {
                let y = list_y + i as f32 * item_height;
                let is_selected = i == self.selected_index;

                // Background
                let bg_color = if is_selected {
                    Color::from_rgba(60, 80, 120, 255)
                } else {
                    Color::from_rgba(40, 40, 60, 255)
                };
                draw_rectangle(list_x, y, 500.0, item_height - 5.0, bg_color);

                if is_selected {
                    draw_rectangle_lines(list_x, y, 500.0, item_height - 5.0, 2.0, WHITE);
                }

                // Character info
                self.draw_text_sharp(&character.name, list_x + 20.0, y + 26.0, 16.0, WHITE);
                let class_info = format!("Level {} {} {}", character.level, character.gender, character.skin);
                self.draw_text_sharp(&class_info, list_x + 20.0, y + 48.0, 16.0, LIGHTGRAY);

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

            self.draw_text_sharp("[Y] Yes, delete    [N] No, cancel", box_x + 70.0, box_y + 100.0, 16.0, LIGHTGRAY);
            return;
        }

        // Instructions at bottom
        let inst_y = sh - 70.0;
        self.draw_text_sharp("[Enter] Play", list_x, inst_y, 16.0, GREEN);
        if self.characters.len() < MAX_CHARACTERS {
            self.draw_text_sharp("[N] New", list_x + 120.0, inst_y, 16.0, YELLOW);
        }
        self.draw_text_sharp("[X] Delete", list_x + 210.0, inst_y, 16.0, RED);
        self.draw_text_sharp("[Esc] Logout", list_x + 330.0, inst_y, 16.0, LIGHTGRAY);

        self.draw_text_sharp("[W/S] Navigate", list_x, inst_y + 24.0, 16.0, DARKGRAY);
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
    error_message: Option<String>,
    auth_client: AuthClient,
    font: BitmapFont,
    active_field: CreateField,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(PartialEq, Clone, Copy)]
enum CreateField {
    Name,
    Gender,
    Skin,
}

#[cfg(not(target_arch = "wasm32"))]
impl CharacterCreateScreen {
    pub fn new(session: AuthSession, server_url: &str) -> Self {
        Self {
            session,
            name: String::new(),
            gender_index: 0,
            skin_index: 0,
            error_message: None,
            auth_client: AuthClient::new(server_url),
            font: BitmapFont::default(),
            active_field: CreateField::Name,
        }
    }

    pub async fn load_font(&mut self) {
        self.font = BitmapFont::load_or_default("assets/fonts/monogram/ttf/monogram-extended.ttf").await;
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
    fn update(&mut self) -> ScreenState {
        // Handle name input when name field is active
        if self.active_field == CreateField::Name {
            self.handle_name_input();
        }

        // Navigate between fields with Tab or Up/Down
        if is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::Down) {
            self.active_field = match self.active_field {
                CreateField::Name => CreateField::Gender,
                CreateField::Gender => CreateField::Skin,
                CreateField::Skin => CreateField::Name,
            };
        }
        if is_key_pressed(KeyCode::Up) {
            self.active_field = match self.active_field {
                CreateField::Name => CreateField::Skin,
                CreateField::Gender => CreateField::Name,
                CreateField::Skin => CreateField::Gender,
            };
        }

        // Left/Right or A/D to cycle options
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
                _ => {}
            }
        }

        // Cancel
        if is_key_pressed(KeyCode::Escape) {
            return ScreenState::ToCharacterSelect(self.session.clone());
        }

        // Create character
        if is_key_pressed(KeyCode::Enter) {
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

            match self.auth_client.create_character(&self.session.token, name, gender, skin) {
                Ok(_) => {
                    // Return to character select - it will refresh the list
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
        let sw = screen_width();
        let sh = screen_height();

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
        self.draw_text_sharp(title, (sw - title_width) / 2.0, 50.0, 16.0, WHITE);

        // Form area
        let form_x = (sw - 400.0) / 2.0;
        let form_y = 120.0;
        let field_height = 80.0;

        // Name field
        let name_active = self.active_field == CreateField::Name;
        let name_y = form_y;
        self.draw_text_sharp("Name", form_x, name_y, 16.0, if name_active { WHITE } else { GRAY });

        let name_box_color = if name_active {
            Color::from_rgba(80, 120, 180, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(form_x, name_y + 24.0, 400.0, 40.0, name_box_color);
        draw_rectangle_lines(form_x, name_y + 24.0, 400.0, 40.0, 2.0, if name_active { WHITE } else { GRAY });

        let cursor = if name_active && (get_time() * 2.0) as i32 % 2 == 0 { "|" } else { "" };
        let name_display = if self.name.is_empty() && !name_active {
            "Enter name...".to_string()
        } else {
            format!("{}{}", self.name, cursor)
        };
        let text_color = if self.name.is_empty() && !name_active { DARKGRAY } else { WHITE };
        self.draw_text_sharp(&name_display, form_x + 12.0, name_y + 52.0, 16.0, text_color);

        // Gender field
        let gender_active = self.active_field == CreateField::Gender;
        let gender_y = form_y + field_height;
        self.draw_text_sharp("Gender", form_x, gender_y, 16.0, if gender_active { WHITE } else { GRAY });

        let gender_box_color = if gender_active {
            Color::from_rgba(80, 120, 180, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(form_x, gender_y + 24.0, 400.0, 40.0, gender_box_color);
        draw_rectangle_lines(form_x, gender_y + 24.0, 400.0, 40.0, 2.0, if gender_active { WHITE } else { GRAY });

        // Left arrow
        self.draw_text_sharp("<", form_x + 20.0, gender_y + 52.0, 16.0, if gender_active { YELLOW } else { DARKGRAY });
        // Current gender
        let gender_text = GENDERS[self.gender_index];
        let gender_width = self.measure_text_sharp(gender_text, 16.0).width;
        self.draw_text_sharp(gender_text, form_x + 200.0 - gender_width / 2.0, gender_y + 52.0, 16.0, WHITE);
        // Right arrow
        self.draw_text_sharp(">", form_x + 370.0, gender_y + 52.0, 16.0, if gender_active { YELLOW } else { DARKGRAY });

        // Skin field
        let skin_active = self.active_field == CreateField::Skin;
        let skin_y = form_y + field_height * 2.0;
        self.draw_text_sharp("Skin", form_x, skin_y, 16.0, if skin_active { WHITE } else { GRAY });

        let skin_box_color = if skin_active {
            Color::from_rgba(80, 120, 180, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(form_x, skin_y + 24.0, 400.0, 40.0, skin_box_color);
        draw_rectangle_lines(form_x, skin_y + 24.0, 400.0, 40.0, 2.0, if skin_active { WHITE } else { GRAY });

        // Left arrow
        self.draw_text_sharp("<", form_x + 20.0, skin_y + 52.0, 16.0, if skin_active { YELLOW } else { DARKGRAY });
        // Current skin
        let skin_text = SKINS[self.skin_index];
        let skin_width = self.measure_text_sharp(skin_text, 16.0).width;
        self.draw_text_sharp(skin_text, form_x + 200.0 - skin_width / 2.0, skin_y + 52.0, 16.0, WHITE);
        // Right arrow
        self.draw_text_sharp(">", form_x + 370.0, skin_y + 52.0, 16.0, if skin_active { YELLOW } else { DARKGRAY });

        // Preview info
        let preview_y = form_y + field_height * 3.0 + 20.0;
        let preview_text = format!("Preview: {} {}", GENDERS[self.gender_index], SKINS[self.skin_index]);
        self.draw_text_sharp(&preview_text, form_x, preview_y, 16.0, LIGHTGRAY);

        // Error message
        if let Some(ref error) = self.error_message {
            let error_y = sh - 130.0;
            let error_width = self.measure_text_sharp(error, 16.0).width;
            self.draw_text_sharp(error, (sw - error_width) / 2.0, error_y, 16.0, RED);
        }

        // Instructions
        let inst_y = sh - 70.0;
        self.draw_text_sharp("[Enter] Create", form_x, inst_y, 16.0, GREEN);
        self.draw_text_sharp("[Escape] Cancel", form_x + 150.0, inst_y, 16.0, LIGHTGRAY);

        self.draw_text_sharp("[Tab] Switch field    [A/D] Change option", form_x, inst_y + 24.0, 16.0, DARKGRAY);
    }
}
