// Library crate for Android builds

use macroquad::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

pub mod util;
pub use util::asset_path;

pub mod mobile_scale;
pub use mobile_scale::MobileScaler;

pub mod game;
pub mod render;
pub mod network;
pub mod input;
#[cfg(not(target_arch = "wasm32"))]
pub mod auth;
pub mod ui;
pub mod audio;
mod app;

use audio::AudioManager;
use game::GameState;
use network::NetworkClient;
use render::Renderer;
use input::InputHandler;

#[cfg(not(target_arch = "wasm32"))]
use ui::{Screen, ScreenState, LoginScreen, CharacterSelectScreen, CharacterCreateScreen};
#[cfg(not(target_arch = "wasm32"))]
use auth::AuthSession;

#[cfg(not(target_arch = "wasm32"))]
use app::AppState;
use app::{window_conf, SERVER_URL, WS_URL, DEV_MODE, run_game_frame};

// For Android, we need to export quad_main as the entry point
// miniquad's JNI code (in MainActivity.java) spawns a thread that calls quad_main
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn quad_main() {
    macroquad::Window::from_config(window_conf(), async_main());
}

// WASM entry point - miniquad's JS bundle calls the exported `main` function
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn main() {
    macroquad::Window::from_config(window_conf(), async_main());
}

// Desktop entry point (not used when building as library, but needed for binary builds)
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
#[macroquad::main(window_conf)]
async fn main() {
    async_main().await;
}

async fn async_main() {
    // Initialize logging (skip on Android - use logcat instead)
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    env_logger::init();

    // Set panic hook for native builds
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!("  at {}:{}:{}", location.file(), location.line(), location.column());
        }
    }));

    // On WASM, macroquad/miniquad handles panic logging to console

    let renderer = Renderer::new().await;
    let mut audio = AudioManager::new().await;
    let scaler = MobileScaler::new();

    // Native build with auth flow
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Start menu music
        audio.play_music("assets/audio/menu.ogg").await;

        let mut login_screen = LoginScreen::new(SERVER_URL);
        login_screen.load_font().await;
        let mut app_state = AppState::Login(login_screen);
        let mut last_next_frame_ms: f64 = 0.0;

        loop {
            let frame_start = Instant::now();

            // Begin scaled rendering for mobile
            scaler.begin_frame();

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
                            let mut game_state = GameState::new();
                            game_state.ui_state.audio_volume = audio.music_volume();
                            game_state.ui_state.audio_sfx_volume = audio.sfx_volume();
                            game_state.ui_state.audio_muted = audio.is_muted();
                            let network = NetworkClient::new_guest(WS_URL);
                            let input_handler = InputHandler::new();

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
                            let mut game_state = GameState::new();
                            game_state.selected_character_name = Some(character_name);
                            game_state.ui_state.audio_volume = audio.music_volume();
                            game_state.ui_state.audio_sfx_volume = audio.sfx_volume();
                            game_state.ui_state.audio_muted = audio.is_muted();

                            let network = NetworkClient::new_authenticated(
                                WS_URL,
                                &session.token,
                                character_id,
                            );
                            let input_handler = InputHandler::new();

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
                            let mut login_screen = LoginScreen::new(SERVER_URL);
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

                    if game_state.disconnect_requested {
                        audio.play_music("assets/audio/menu.ogg").await;
                        network.disconnect();
                        let mut login_screen = LoginScreen::new(SERVER_URL);
                        login_screen.load_font().await;
                        app_state = AppState::Login(login_screen);
                        continue;
                    }

                    if game_state.reconnection_failed {
                        log::info!("Reconnection failed, returning to login screen");
                        audio.play_music("assets/audio/menu.ogg").await;
                        network.disconnect();
                        let mut login_screen = LoginScreen::new(SERVER_URL);
                        login_screen.load_font().await;
                        app_state = AppState::Login(login_screen);
                        continue;
                    }
                }
            }

            // End scaled rendering for mobile (draws scaled result to screen)
            scaler.end_frame();

            // Apply optional FPS cap
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

            let next_frame_start = Instant::now();
            next_frame().await;
            last_next_frame_ms = next_frame_start.elapsed().as_secs_f64() * 1000.0;
        }
    }

    // WASM build - networked game mode
    #[cfg(target_arch = "wasm32")]
    {
        let mut game_state = GameState::new();
        let mut network = NetworkClient::new_guest(WS_URL);
        let mut input_handler = InputHandler::new();

        loop {
            run_game_frame(&mut game_state, &mut network, &mut input_handler, &renderer, &mut audio);
            next_frame().await;
        }
    }
}
