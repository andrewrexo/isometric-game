use macroquad::prelude::*;

mod game;
mod render;
#[cfg(not(target_arch = "wasm32"))]
mod network;
mod input;
#[cfg(not(target_arch = "wasm32"))]
mod auth;
#[cfg(not(target_arch = "wasm32"))]
mod ui;

use game::GameState;
#[cfg(not(target_arch = "wasm32"))]
use network::NetworkClient;
use render::Renderer;
use input::{InputHandler, InputCommand};

#[cfg(not(target_arch = "wasm32"))]
use ui::{Screen, ScreenState, LoginScreen, CharacterSelectScreen};
#[cfg(not(target_arch = "wasm32"))]
use auth::AuthSession;

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
        ..Default::default()
    }
}

/// Application state for native builds
#[cfg(not(target_arch = "wasm32"))]
enum AppState {
    Login(LoginScreen),
    CharacterSelect(CharacterSelectScreen),
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

    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let renderer = Renderer::new().await;

    // Native build with auth flow
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app_state = AppState::Login(LoginScreen::new(SERVER_URL, DEV_MODE));

        loop {
            match &mut app_state {
                AppState::Login(screen) => {
                    let result = screen.update();
                    screen.render();

                    match result {
                        ScreenState::ToCharacterSelect(session) => {
                            app_state = AppState::CharacterSelect(
                                CharacterSelectScreen::new(session, SERVER_URL)
                            );
                        }
                        ScreenState::StartGameDirect { session } => {
                            // Simple model: account = character, go directly to game
                            let mut game_state = GameState::new();
                            game_state.selected_character_name = Some(session.username.clone());

                            // Authenticated matchmaking with token
                            let network = NetworkClient::new_with_token(WS_URL, &session.token, &session.username);
                            let input_handler = InputHandler::new();

                            app_state = AppState::Playing {
                                game_state,
                                network,
                                input_handler,
                                _session: session,
                            };
                        }
                        ScreenState::StartGuestMode => {
                            // Guest mode - connect without auth
                            let game_state = GameState::new();
                            let network = NetworkClient::new_guest(WS_URL);
                            let input_handler = InputHandler::new();
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
                    let result = screen.update();
                    screen.render();

                    match result {
                        ScreenState::StartGame { session, character_id, character_name } => {
                            // Start game with selected character
                            let mut game_state = GameState::new();
                            game_state.selected_character_name = Some(character_name);

                            let network = NetworkClient::new_authenticated(
                                WS_URL,
                                &session.token,
                                character_id,
                            );
                            let input_handler = InputHandler::new();

                            app_state = AppState::Playing {
                                game_state,
                                network,
                                input_handler,
                                _session: session,
                            };
                        }
                        ScreenState::ToLogin => {
                            app_state = AppState::Login(LoginScreen::new(SERVER_URL, DEV_MODE));
                        }
                        _ => {}
                    }
                }

                AppState::Playing { game_state, network, input_handler, .. } |
                AppState::GuestMode { game_state, network, input_handler } => {
                    run_game_frame(game_state, network, input_handler, &renderer);
                }
            }

            next_frame().await;
        }
    }

    // WASM build - offline demo mode
    #[cfg(target_arch = "wasm32")]
    {
        let mut game_state = GameState::new();

        // Create a local player for demo
        use game::Player;
        let player = Player::new("local".to_string(), "WebPlayer".to_string(), 5.0, 5.0);
        game_state.players.insert("local".to_string(), player);
        game_state.local_player_id = Some("local".to_string());

        let mut input_handler = InputHandler::new();

        loop {
            let delta = get_frame_time();

            // Handle input (local only in WASM)
            let _ = input_handler.process(&mut game_state);

            // Update game state
            let (input_dx, input_dy) = input_handler.get_movement();
            game_state.update(delta, input_dx, input_dy);

            // Render
            clear_background(Color::from_rgba(30, 30, 40, 255));
            renderer.render(&game_state);

            // Debug info
            if game_state.debug_mode {
                draw_text(&format!("FPS: {}", get_fps()), 10.0, 20.0, 20.0, WHITE);
                draw_text("WASM Demo (no network)", 10.0, 40.0, 20.0, YELLOW);
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
) {
    let delta = get_frame_time();

    // 1. Poll network messages
    network.poll(game_state);

    // 2. Handle input and send commands
    let commands = input_handler.process(game_state);
    for cmd in &commands {
        use network::messages::ClientMessage;
        let msg = match cmd {
            InputCommand::Move { dx, dy } => ClientMessage::Move { dx: *dx, dy: *dy },
            InputCommand::Attack => {
                // Trigger attack animation on local player
                if let Some(local_id) = &game_state.local_player_id {
                    if let Some(player) = game_state.players.get_mut(local_id) {
                        player.play_attack();
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
        };
        network.send(&msg);
    }

    // 3. Update game state
    let (input_dx, input_dy) = input_handler.get_movement();
    game_state.update(delta, input_dx, input_dy);

    // 4. Render
    clear_background(Color::from_rgba(30, 30, 40, 255));
    renderer.render(game_state);

    // 5. Debug info
    if game_state.debug_mode {
        draw_text(&format!("FPS: {}", get_fps()), 10.0, 20.0, 20.0, WHITE);
        draw_text(&format!("Players: {}", game_state.players.len()), 10.0, 40.0, 20.0, WHITE);
        draw_text(&format!("Connected: {}", network.is_connected()), 10.0, 60.0, 20.0, WHITE);

        // Show position and chunk info
        if let Some(player) = game_state.get_local_player() {
            let chunk_x = (player.x / 32.0).floor() as i32;
            let chunk_y = (player.y / 32.0).floor() as i32;
            draw_text(&format!("Pos: ({:.1}, {:.1})", player.x, player.y), 10.0, 80.0, 20.0, YELLOW);
            draw_text(&format!("Chunk: ({}, {})", chunk_x, chunk_y), 10.0, 100.0, 20.0, YELLOW);
            draw_text(&format!("NPCs: {}", game_state.npcs.len()), 10.0, 120.0, 20.0, WHITE);
        }
    }
}
