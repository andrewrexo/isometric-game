use macroquad::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

mod game;
mod render;
#[cfg(not(target_arch = "wasm32"))]
mod network;
mod input;
#[cfg(not(target_arch = "wasm32"))]
mod auth;
#[cfg(not(target_arch = "wasm32"))]
mod ui;
mod audio;

use audio::AudioManager;

use game::GameState;
#[cfg(not(target_arch = "wasm32"))]
use network::NetworkClient;
use render::Renderer;
use input::{InputHandler, InputCommand};
use render::animation::AnimationState;

#[cfg(not(target_arch = "wasm32"))]
use ui::{Screen, ScreenState, LoginScreen, CharacterSelectScreen, CharacterCreateScreen};
#[cfg(not(target_arch = "wasm32"))]
use auth::AuthSession;

// Production mode - use the production server.
// const SERVER_URL: &str = "http://5.161.177.38:2567";
// const WS_URL: &str = "ws://5.161.177.38:2567";

// Development mode - use the development server
const SERVER_URL: &str = "http://localhost:2567";
const WS_URL: &str = "ws://localhost:2567";

// Development mode - enables guest login
// Set to false for production builds
const DEV_MODE: bool = true;

fn window_conf() -> Conf {
    Conf {
        window_title: "New Aeven".to_string(),
        window_width: 1280,
        window_height: 720,
        fullscreen: false,
        platform: miniquad::conf::Platform {
            // Disable VSync for uncapped FPS - manual frame timing handles pacing
            // VSync on macOS causes unreliable frame pacing (12-14ms variance)
            swap_interval: Some(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Application state for native builds
#[cfg(not(target_arch = "wasm32"))]
enum AppState {
    Login(LoginScreen),
    CharacterSelect(CharacterSelectScreen),
    CharacterCreate(CharacterCreateScreen),
    Playing {
        game_state: GameState,
        network: NetworkClient,
        input_handler: InputHandler,
        _session: AuthSession,
    },
    GuestMode {
        game_state: GameState,
        network: NetworkClient,
        input_handler: InputHandler,
    },
}

#[macroquad::main(window_conf)]
async fn main() {
    // Initialize logging
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    // Set panic hook for native builds to capture crash info
    #[cfg(not(target_arch = "wasm32"))]
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!("  at {}:{}:{}", location.file(), location.line(), location.column());
        }
    }));

    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let renderer = Renderer::new().await;
    let mut audio = AudioManager::new().await;

    // Native build with auth flow
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Start menu music
        audio.play_music("assets/audio/menu.ogg").await;

        let mut login_screen = LoginScreen::new(SERVER_URL, DEV_MODE);
        login_screen.load_font().await;
        let mut app_state = AppState::Login(login_screen);
        let mut last_next_frame_ms: f64 = 0.0;

        loop {
            let frame_start = Instant::now();

            // Record last frame's next_frame() time into game state
            if let AppState::Playing { game_state, .. } | AppState::GuestMode { game_state, .. } = &mut app_state {
                game_state.frame_timings.record_next_frame(last_next_frame_ms);
            }

            match &mut app_state {
                AppState::Login(screen) => {
                    let result = screen.update(&audio);
                    screen.render();

                    match result {
                        ScreenState::ToCharacterSelect(session) => {
                            audio.play_sfx("login_success");
                            let mut char_screen = CharacterSelectScreen::new(session, SERVER_URL);
                            char_screen.load_font().await;
                            app_state = AppState::CharacterSelect(char_screen);
                        }
                        ScreenState::StartGuestMode => {
                            // Guest mode - connect without auth
                            let mut game_state = GameState::new();
                            // Sync audio settings to UI state
                            game_state.ui_state.audio_volume = audio.music_volume();
                            game_state.ui_state.audio_sfx_volume = audio.sfx_volume();
                            game_state.ui_state.audio_muted = audio.is_muted();
                            let network = NetworkClient::new_guest(WS_URL);
                            let input_handler = InputHandler::new();

                            // Start background music
                            audio.play_music("assets/audio/start.ogg").await;

                            app_state = AppState::GuestMode {
                                game_state,
                                network,
                                input_handler,
                            };
                        }
                        _ => {}
                    }
                }

                AppState::CharacterSelect(screen) => {
                    let result = screen.update(&audio);
                    screen.render();

                    match result {
                        ScreenState::StartGame { session, character_id, character_name } => {
                            // Start game with selected character
                            let mut game_state = GameState::new();
                            game_state.selected_character_name = Some(character_name);
                            // Sync audio settings to UI state
                            game_state.ui_state.audio_volume = audio.music_volume();
                            game_state.ui_state.audio_sfx_volume = audio.sfx_volume();
                            game_state.ui_state.audio_muted = audio.is_muted();

                            let network = NetworkClient::new_authenticated(
                                WS_URL,
                                &session.token,
                                character_id,
                            );
                            let input_handler = InputHandler::new();

                            // Start background music
                            audio.play_music("assets/audio/start.ogg").await;

                            app_state = AppState::Playing {
                                game_state,
                                network,
                                input_handler,
                                _session: session,
                            };
                        }
                        ScreenState::ToCharacterCreate(session) => {
                            let mut create_screen = CharacterCreateScreen::new(session, SERVER_URL);
                            create_screen.load_font().await;
                            app_state = AppState::CharacterCreate(create_screen);
                        }
                        ScreenState::ToLogin => {
                            let mut login_screen = LoginScreen::new(SERVER_URL, DEV_MODE);
                            login_screen.load_font().await;
                            app_state = AppState::Login(login_screen);
                        }
                        _ => {}
                    }
                }

                AppState::CharacterCreate(screen) => {
                    let result = screen.update(&audio);
                    screen.render();

                    match result {
                        ScreenState::ToCharacterSelect(session) => {
                            let mut char_screen = CharacterSelectScreen::new(session, SERVER_URL);
                            char_screen.load_font().await;
                            app_state = AppState::CharacterSelect(char_screen);
                        }
                        _ => {}
                    }
                }

                AppState::Playing { game_state, network, input_handler, .. } |
                AppState::GuestMode { game_state, network, input_handler } => {
                    run_game_frame(game_state, network, input_handler, &renderer, &mut audio);

                    // Check for disconnect request
                    if game_state.disconnect_requested {
                        // Switch to menu music and disconnect from server
                        audio.play_music("assets/audio/menu.ogg").await;
                        network.disconnect();
                        let mut login_screen = LoginScreen::new(SERVER_URL, DEV_MODE);
                        login_screen.load_font().await;
                        app_state = AppState::Login(login_screen);
                        continue;
                    }

                    // Check for reconnection failure (server disconnected and retries exhausted)
                    if game_state.reconnection_failed {
                        log::info!("Reconnection failed, returning to login screen");
                        audio.play_music("assets/audio/menu.ogg").await;
                        network.disconnect();
                        let mut login_screen = LoginScreen::new(SERVER_URL, DEV_MODE);
                        login_screen.load_font().await;
                        app_state = AppState::Login(login_screen);
                        continue;
                    }
                }
            }

            // Apply optional FPS cap (only when in game)
            let fps_cap = match &app_state {
                AppState::Playing { game_state, .. } | AppState::GuestMode { game_state, .. } => {
                    game_state.frame_timings.fps_cap
                }
                _ => None,
            };

            if let Some(cap) = fps_cap {
                let target_frame_time = Duration::from_secs_f64(1.0 / cap as f64);
                let elapsed = frame_start.elapsed();
                if elapsed < target_frame_time {
                    std::thread::sleep(target_frame_time - elapsed);
                }
            }

            // Measure time spent in next_frame() to diagnose variance
            let next_frame_start = Instant::now();
            next_frame().await;
            last_next_frame_ms = next_frame_start.elapsed().as_secs_f64() * 1000.0;
        }
    }

    // WASM build - offline demo mode
    #[cfg(target_arch = "wasm32")]
    {
        let mut game_state = GameState::new();

        // Create a local player for demo with default appearance
        use game::Player;
        let player = Player::new("local".to_string(), "WebPlayer".to_string(), 5.0, 5.0, "male".to_string(), "tan".to_string());
        game_state.players.insert("local".to_string(), player);
        game_state.local_player_id = Some("local".to_string());

        let mut input_handler = InputHandler::new();

        loop {
            let delta = get_frame_time();

            // Toggle debug mode with F3
            if is_key_pressed(KeyCode::F3) {
                game_state.debug_mode = !game_state.debug_mode;
            }

            // Debug controls for appearance cycling (only in debug mode)
            if game_state.debug_mode {
                if let Some(local_id) = &game_state.local_player_id.clone() {
                    if let Some(player) = game_state.players.get_mut(local_id) {
                        if is_key_pressed(KeyCode::F5) {
                            player.gender = match player.gender.as_str() {
                                "male" => "female".to_string(),
                                _ => "male".to_string(),
                            };
                        }
                        if is_key_pressed(KeyCode::F6) {
                            let skins = ["tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"];
                            let current_idx = skins.iter().position(|&s| s == player.skin).unwrap_or(0);
                            let next_idx = (current_idx + 1) % skins.len();
                            player.skin = skins[next_idx].to_string();
                        }
                    }
                }
            }

            // Render and get UI layout
            clear_background(Color::from_rgba(30, 30, 40, 255));
            let (layout, _render_timings) = renderer.render(&game_state);

            // Handle input with UI layout (local only in WASM)
            let _ = input_handler.process(&mut game_state, &layout, &mut audio);

            // Update game state
            let (input_dx, input_dy) = input_handler.get_movement();
            game_state.update(delta, input_dx, input_dy);

            // Debug info
            if game_state.debug_mode {
                renderer.draw_text_sharp(&format!("FPS: {}", get_fps()), 10.0, 20.0, 16.0, WHITE);
                renderer.draw_text_sharp("WASM Demo (no network)", 10.0, 40.0, 16.0, YELLOW);
                if let Some(player) = game_state.get_local_player() {
                    renderer.draw_text_sharp(&format!("Appearance: {} {} (F5/F6 to cycle)", player.gender, player.skin), 10.0, 60.0, 16.0, Color::from_rgba(150, 200, 255, 255));
                }
            }

            next_frame().await;
        }
    }
}

/// Run a single frame of gameplay
#[cfg(not(target_arch = "wasm32"))]
fn run_game_frame(
    game_state: &mut GameState,
    network: &mut NetworkClient,
    input_handler: &mut InputHandler,
    renderer: &Renderer,
    audio: &mut AudioManager,
) {
    let frame_start = get_time();
    let delta = get_frame_time();

    // Toggle debug mode with F3
    if is_key_pressed(KeyCode::F3) {
        game_state.debug_mode = !game_state.debug_mode;
    }

    // Cycle FPS cap with F4: Uncapped -> 60 -> 144 -> 240 -> Uncapped
    if is_key_pressed(KeyCode::F4) {
        game_state.frame_timings.fps_cap = match game_state.frame_timings.fps_cap {
            None => Some(60),
            Some(60) => Some(144),
            Some(144) => Some(240),
            _ => None,
        };
        log::info!("FPS cap: {:?}", game_state.frame_timings.fps_cap);
    }

    // Cycle delta smoothing with F7: 0 -> 0.5 -> 0.8 -> 0.9 -> 0
    if is_key_pressed(KeyCode::F7) {
        game_state.frame_timings.delta_smoothing = match game_state.frame_timings.delta_smoothing {
            x if x < 0.1 => 0.5,
            x if x < 0.6 => 0.8,
            x if x < 0.85 => 0.9,
            _ => 0.0,
        };
        log::info!("Delta smoothing: {}", game_state.frame_timings.delta_smoothing);
    }

    // Debug controls for appearance cycling (only in debug mode)
    if game_state.debug_mode {
        if let Some(local_id) = &game_state.local_player_id.clone() {
            if let Some(player) = game_state.players.get_mut(local_id) {
                // F5 to cycle gender
                if is_key_pressed(KeyCode::F5) {
                    player.gender = match player.gender.as_str() {
                        "male" => "female".to_string(),
                        _ => "male".to_string(),
                    };
                    log::info!("Debug: Changed gender to {}", player.gender);
                }

                // F6 to cycle skin
                if is_key_pressed(KeyCode::F6) {
                    let skins = ["tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"];
                    let current_idx = skins.iter().position(|&s| s == player.skin).unwrap_or(0);
                    let next_idx = (current_idx + 1) % skins.len();
                    player.skin = skins[next_idx].to_string();
                    log::info!("Debug: Changed skin to {}", player.skin);
                }
            }
        }
    }

    // 1. Poll network messages
    let network_start = get_time();
    network.poll(game_state);
    let network_ms = (get_time() - network_start) * 1000.0;

    // 2. Render and get UI layout for hit detection
    clear_background(Color::from_rgba(30, 30, 40, 255));
    let (layout, render_timings) = renderer.render(game_state);

    // 3. Handle input with UI layout and send commands
    let commands = input_handler.process(game_state, &layout, audio);
    for cmd in &commands {
        use network::messages::ClientMessage;
        let msg = match cmd {
            InputCommand::Move { dx, dy } => ClientMessage::Move { dx: *dx, dy: *dy },
            InputCommand::Face { direction } => {
                log::info!("[MAIN] Processing Face command: direction={}", direction);
                // Skip direction update if attacking - player must finish attack first
                if let Some(local_id) = &game_state.local_player_id {
                    if let Some(player) = game_state.players.get(local_id) {
                        let is_attacking = matches!(
                            player.animation.state,
                            AnimationState::Attacking | AnimationState::Casting | AnimationState::ShootingBow
                        );
                        if is_attacking {
                            continue;
                        }
                    }
                }
                // Record when we sent Face to ignore stale server updates
                game_state.last_face_command_time = get_time();
                // Immediately update local player direction for responsiveness
                if let Some(local_id) = &game_state.local_player_id {
                    if let Some(player) = game_state.players.get_mut(local_id) {
                        let old_dir = player.direction;
                        let new_dir = game::Direction::from_u8(*direction);
                        player.direction = new_dir;
                        player.animation.direction = new_dir; // Also update animation direction for rendering
                        log::info!("[MAIN] Updated local player direction: {:?} -> {:?}", old_dir, player.direction);
                    }
                }
                ClientMessage::Face { direction: *direction }
            },
            InputCommand::Attack => {
                // Trigger attack animation and sound on local player
                if let Some(local_id) = &game_state.local_player_id {
                    // Check weapon type to determine animation
                    let is_ranged = game_state.players.get(local_id)
                        .and_then(|p| p.equipped_weapon.as_ref())
                        .and_then(|weapon_id| game_state.item_registry.get(weapon_id))
                        .map(|item_def| item_def.weapon_type.as_deref() == Some("ranged"))
                        .unwrap_or(false);

                    if let Some(player) = game_state.players.get_mut(local_id) {
                        if is_ranged {
                            player.play_shoot_bow();
                        } else {
                            player.play_attack();
                        }
                        let has_weapon = player.equipped_weapon.is_some();
                        audio.play_attack_sound(has_weapon);
                    }
                }
                ClientMessage::Attack
            },
            InputCommand::Target { entity_id } => ClientMessage::Target { entity_id: entity_id.clone() },
            InputCommand::ClearTarget => ClientMessage::Target { entity_id: String::new() },
            InputCommand::Chat { text } => ClientMessage::Chat { text: text.clone() },
            InputCommand::Pickup { item_id } => ClientMessage::Pickup { item_id: item_id.clone() },
            InputCommand::UseItem { slot_index } => ClientMessage::UseItem { slot_index: *slot_index as u32 },
            // Quest-related commands
            InputCommand::Interact { npc_id } => ClientMessage::Interact { npc_id: npc_id.clone() },
            InputCommand::DialogueChoice { quest_id, choice_id } => ClientMessage::DialogueChoice {
                quest_id: quest_id.clone(),
                choice_id: choice_id.clone(),
            },
            InputCommand::CloseDialogue => {
                // Just close locally, no server message needed
                continue;
            },
            // Crafting command
            InputCommand::Craft { recipe_id } => ClientMessage::Craft { recipe_id: recipe_id.clone() },
            // Equipment commands
            InputCommand::Equip { slot_index } => ClientMessage::Equip { slot_index: *slot_index },
            InputCommand::Unequip { slot_type, target_slot } => ClientMessage::Unequip { slot_type: slot_type.clone(), target_slot: *target_slot },
            // Inventory commands
            InputCommand::DropItem { slot_index, quantity } => ClientMessage::DropItem { slot_index: *slot_index, quantity: *quantity },
            InputCommand::DropGold { amount } => ClientMessage::DropGold { amount: *amount },
            InputCommand::SwapSlots { from_slot, to_slot } => ClientMessage::SwapSlots { from_slot: *from_slot, to_slot: *to_slot },
            // Shop commands
            InputCommand::ShopBuy { npc_id, item_id, quantity } => ClientMessage::ShopBuy { npc_id: npc_id.clone(), item_id: item_id.clone(), quantity: *quantity },
            InputCommand::ShopSell { npc_id, item_id, quantity } => ClientMessage::ShopSell { npc_id: npc_id.clone(), item_id: item_id.clone(), quantity: *quantity },
        };
        network.send(&msg);
    }

    // Record delta for diagnostics
    game_state.frame_timings.record_delta(delta as f64 * 1000.0);

    // 4. Update game state using raw delta (lerp-based interpolation handles smoothing)
    let update_start = get_time();
    let (input_dx, input_dy) = input_handler.get_movement();
    game_state.update(delta, input_dx, input_dy);
    let update_ms = (get_time() - update_start) * 1000.0;

    // 4b. Request chunks around player position
    if let Some(player) = game_state.get_local_player() {
        let chunks_to_request = game_state.chunk_manager.update_player_position(player.x, player.y);
        for coord in chunks_to_request {
            network.send(&network::messages::ClientMessage::RequestChunk {
                chunk_x: coord.x,
                chunk_y: coord.y,
            });
        }
    }

    // Store frame timings
    let total_ms = (get_time() - frame_start) * 1000.0;
    game_state.frame_timings.network_ms = network_ms;
    game_state.frame_timings.render_total_ms = render_timings.total_ms;
    game_state.frame_timings.render_ground_ms = render_timings.ground_ms;
    game_state.frame_timings.render_entities_ms = render_timings.entities_ms;
    game_state.frame_timings.render_overhead_ms = render_timings.overhead_ms;
    game_state.frame_timings.render_effects_ms = render_timings.effects_ms;
    game_state.frame_timings.render_ui_ms = render_timings.ui_ms;
    game_state.frame_timings.update_ms = update_ms;
    game_state.frame_timings.total_ms = total_ms;
    game_state.frame_timings.entity_count = game_state.players.len() + game_state.npcs.len() + game_state.ground_items.len();
    game_state.frame_timings.chunk_count = game_state.chunk_manager.chunks().len();

    // 5. Debug info (render after game state update to show current frame data)
    // Positioned below player stats panel (which ends around y=69)
    if game_state.debug_mode {
        let debug_y = 80.0; // Start below player stats
        let fps_cap_str = match game_state.frame_timings.fps_cap {
            Some(cap) => format!(" (cap: {})", cap),
            None => " (uncapped)".to_string(),
        };
        renderer.draw_text_sharp(&format!("FPS: {}{} [F4]", get_fps(), fps_cap_str), 10.0, debug_y, 16.0, WHITE);
        renderer.draw_text_sharp(&format!("Players: {}", game_state.players.len()), 10.0, debug_y + 20.0, 16.0, WHITE);
        renderer.draw_text_sharp(&format!("Connected: {}", network.is_connected()), 10.0, debug_y + 40.0, 16.0, WHITE);

        // Show position and chunk info
        if let Some(player) = game_state.get_local_player() {
            let chunk_x = (player.x / 32.0).floor() as i32;
            let chunk_y = (player.y / 32.0).floor() as i32;
            renderer.draw_text_sharp(&format!("Pos: ({:.1}, {:.1})", player.x, player.y), 10.0, debug_y + 60.0, 16.0, YELLOW);
            renderer.draw_text_sharp(&format!("Chunk: ({}, {})", chunk_x, chunk_y), 10.0, debug_y + 80.0, 16.0, YELLOW);
            renderer.draw_text_sharp(&format!("NPCs: {}", game_state.npcs.len()), 10.0, debug_y + 100.0, 16.0, WHITE);
            // Appearance debug info
            renderer.draw_text_sharp(&format!("Appearance: {} {} (F5/F6 to cycle)", player.gender, player.skin), 10.0, debug_y + 120.0, 16.0, Color::from_rgba(150, 200, 255, 255));
        }

        // Frame timing breakdown
        let t = &game_state.frame_timings;
        let timing_color = Color::from_rgba(100, 255, 150, 255);
        let spike_color = Color::from_rgba(255, 100, 100, 255);
        renderer.draw_text_sharp("--- Frame Timing (ms) ---", 10.0, debug_y + 150.0, 16.0, timing_color);
        renderer.draw_text_sharp(&format!("Network:  {:.2}", t.network_ms), 10.0, debug_y + 170.0, 16.0, timing_color);

        // Render breakdown with spike highlighting (>0.5ms highlighted)
        let ground_color = if t.render_ground_ms > 0.5 { spike_color } else { timing_color };
        let entities_color = if t.render_entities_ms > 0.5 { spike_color } else { timing_color };
        let overhead_color = if t.render_overhead_ms > 0.5 { spike_color } else { timing_color };
        let effects_color = if t.render_effects_ms > 0.5 { spike_color } else { timing_color };
        let ui_color = if t.render_ui_ms > 0.5 { spike_color } else { timing_color };

        renderer.draw_text_sharp(&format!("Render:   {:.2} (total)", t.render_total_ms), 10.0, debug_y + 190.0, 16.0, timing_color);
        renderer.draw_text_sharp(&format!("  Ground:   {:.2}", t.render_ground_ms), 10.0, debug_y + 210.0, 16.0, ground_color);
        renderer.draw_text_sharp(&format!("  Entities: {:.2}", t.render_entities_ms), 10.0, debug_y + 230.0, 16.0, entities_color);
        renderer.draw_text_sharp(&format!("  Overhead: {:.2}", t.render_overhead_ms), 10.0, debug_y + 250.0, 16.0, overhead_color);
        renderer.draw_text_sharp(&format!("  Effects:  {:.2}", t.render_effects_ms), 10.0, debug_y + 270.0, 16.0, effects_color);
        renderer.draw_text_sharp(&format!("  UI:       {:.2}", t.render_ui_ms), 10.0, debug_y + 290.0, 16.0, ui_color);

        renderer.draw_text_sharp(&format!("Update:   {:.2}", t.update_ms), 10.0, debug_y + 310.0, 16.0, timing_color);
        renderer.draw_text_sharp(&format!("Total:    {:.2}", t.total_ms), 10.0, debug_y + 330.0, 16.0, timing_color);
        renderer.draw_text_sharp(&format!("Entities: {} | Chunks: {}", t.entity_count, t.chunk_count), 10.0, debug_y + 350.0, 16.0, timing_color);

        // Delta variance (key indicator of frame pacing issues)
        let delta_variance = t.delta_max_ms - t.delta_min_ms;
        let variance_color = if delta_variance > 5.0 { spike_color } else { timing_color };
        renderer.draw_text_sharp(&format!("Delta: {:.1}ms (range: {:.1}-{:.1}, var: {:.1})",
            t.delta_ms, t.delta_min_ms, t.delta_max_ms, delta_variance), 10.0, debug_y + 370.0, 16.0, variance_color);

        // next_frame() timing (helps diagnose where variance comes from)
        let nf_variance = t.next_frame_max_ms - t.next_frame_min_ms;
        let nf_color = if nf_variance > 5.0 { spike_color } else { timing_color };
        renderer.draw_text_sharp(&format!("next_frame(): {:.1}ms (range: {:.1}-{:.1}, var: {:.1})",
            t.next_frame_ms, t.next_frame_min_ms, t.next_frame_max_ms, nf_variance), 10.0, debug_y + 390.0, 16.0, nf_color);

        // Delta smoothing setting
        let smooth_str = if t.delta_smoothing > 0.0 {
            format!("{:.1}", t.delta_smoothing)
        } else {
            "off".to_string()
        };
        renderer.draw_text_sharp(&format!("Smoothing: {} [F7] (smoothed: {:.1}ms)",
            smooth_str, t.smoothed_delta * 1000.0), 10.0, debug_y + 410.0, 16.0, timing_color);
    }
}
