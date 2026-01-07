use macroquad::prelude::*;

mod game;
mod render;
#[cfg(not(target_arch = "wasm32"))]
mod network;
mod input;

use game::GameState;
#[cfg(not(target_arch = "wasm32"))]
use network::NetworkClient;
use render::Renderer;
use input::{InputHandler, InputCommand};

fn window_conf() -> Conf {
    Conf {
        window_title: "Isometric MMORPG".to_string(),
        window_width: 1280,
        window_height: 720,
        fullscreen: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Initialize logging
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    // Initialize game systems
    let mut game_state = GameState::new();
    #[cfg(not(target_arch = "wasm32"))]
    let mut network = NetworkClient::new("ws://localhost:2567");
    let renderer = Renderer::new().await;
    let mut input_handler = InputHandler::new();

    // For WASM demo, create a local player
    #[cfg(target_arch = "wasm32")]
    {
        use game::Player;
        let player = Player::new("local".to_string(), "WebPlayer".to_string(), 5.0, 5.0);
        game_state.players.insert("local".to_string(), player);
        game_state.local_player_id = Some("local".to_string());
    }

    // Main game loop
    loop {
        let delta = get_frame_time();

        // 1. Poll network messages (non-blocking) - native only
        #[cfg(not(target_arch = "wasm32"))]
        network.poll(&mut game_state);

        // 2. Handle input and send commands to server
        let commands = input_handler.process(&mut game_state);
        #[cfg(not(target_arch = "wasm32"))]
        for cmd in &commands {
            use network::messages::ClientMessage;
            let msg = match cmd {
                InputCommand::Move { dx, dy } => ClientMessage::Move { dx: *dx, dy: *dy },
                InputCommand::Attack => ClientMessage::Attack,
                InputCommand::Target { entity_id } => ClientMessage::Target { entity_id: entity_id.clone() },
                InputCommand::ClearTarget => ClientMessage::Target { entity_id: String::new() },
                InputCommand::Chat { text } => ClientMessage::Chat { text: text.clone() },
                InputCommand::Pickup { item_id } => ClientMessage::Pickup { item_id: item_id.clone() },
                InputCommand::UseItem { slot_index } => ClientMessage::UseItem { slot_index: *slot_index as u32 },
            };
            network.send(&msg);
        }
        let _ = commands; // Suppress unused warning on WASM

        // 3. Update game state with current input (smooth local movement)
        let (input_dx, input_dy) = input_handler.get_movement();
        game_state.update(delta, input_dx, input_dy);

        // 4. Render
        clear_background(Color::from_rgba(30, 30, 40, 255));
        renderer.render(&game_state);

        // 5. Debug info
        if game_state.debug_mode {
            draw_text(
                &format!("FPS: {}", get_fps()),
                10.0,
                20.0,
                20.0,
                WHITE,
            );
            draw_text(
                &format!("Players: {}", game_state.players.len()),
                10.0,
                40.0,
                20.0,
                WHITE,
            );
            #[cfg(not(target_arch = "wasm32"))]
            draw_text(
                &format!("Connected: {}", network.is_connected()),
                10.0,
                60.0,
                20.0,
                WHITE,
            );
            #[cfg(target_arch = "wasm32")]
            draw_text(
                "WASM Demo (no network)",
                10.0,
                60.0,
                20.0,
                WHITE,
            );
        }

        next_frame().await
    }
}
