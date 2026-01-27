// Shared application code between desktop and Android builds

use macroquad::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use crate::game::GameState;
#[cfg(not(target_arch = "wasm32"))]
use crate::network::NetworkClient;
use crate::render::Renderer;
use crate::input::{InputHandler, InputCommand};
use crate::render::animation::AnimationState;
use crate::audio::AudioManager;

#[cfg(not(target_arch = "wasm32"))]
use crate::ui::{Screen, ScreenState, LoginScreen, CharacterSelectScreen, CharacterCreateScreen};
#[cfg(not(target_arch = "wasm32"))]
use crate::auth::AuthSession;

// Production mode - use the production server.
pub const SERVER_URL: &str = "http://5.161.177.38:2567";
pub const WS_URL: &str = "ws://5.161.177.38:2567";

// Development mode - use the development server
// pub const SERVER_URL: &str = "http://localhost:2567";
// pub const WS_URL: &str = "ws://localhost:2567";

// Development mode - enables guest login
// Set to false for production builds
pub const DEV_MODE: bool = true;

pub fn window_conf() -> Conf {
    Conf {
        window_title: "New Aeven".to_string(),
        window_width: 1280,
        window_height: 720,
        fullscreen: false,
        platform: miniquad::conf::Platform {
            swap_interval: Some(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Application state for native builds
#[cfg(not(target_arch = "wasm32"))]
pub enum AppState {
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

/// Run a single frame of gameplay
#[cfg(not(target_arch = "wasm32"))]
pub fn run_game_frame(
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
        use crate::network::messages::ClientMessage;
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
                        let new_dir = crate::game::Direction::from_u8(*direction);
                        player.direction = new_dir;
                        player.animation.direction = new_dir;
                        log::info!("[MAIN] Updated local player direction: {:?} -> {:?}", old_dir, player.direction);
                    }
                }
                ClientMessage::Face { direction: *direction }
            },
            InputCommand::Attack => {
                // Trigger attack animation and sound on local player
                if let Some(local_id) = &game_state.local_player_id {
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
            InputCommand::Interact { npc_id } => ClientMessage::Interact { npc_id: npc_id.clone() },
            InputCommand::DialogueChoice { quest_id, choice_id } => ClientMessage::DialogueChoice {
                quest_id: quest_id.clone(),
                choice_id: choice_id.clone(),
            },
            InputCommand::CloseDialogue => {
                continue;
            },
            InputCommand::Craft { recipe_id } => ClientMessage::Craft { recipe_id: recipe_id.clone() },
            InputCommand::Equip { slot_index } => ClientMessage::Equip { slot_index: *slot_index },
            InputCommand::Unequip { slot_type, target_slot } => ClientMessage::Unequip { slot_type: slot_type.clone(), target_slot: *target_slot },
            InputCommand::DropItem { slot_index, quantity, target_x, target_y } => ClientMessage::DropItem { slot_index: *slot_index, quantity: *quantity, target_x: *target_x, target_y: *target_y },
            InputCommand::DropGold { amount } => ClientMessage::DropGold { amount: *amount },
            InputCommand::SwapSlots { from_slot, to_slot } => ClientMessage::SwapSlots { from_slot: *from_slot, to_slot: *to_slot },
            InputCommand::ShopBuy { npc_id, item_id, quantity } => ClientMessage::ShopBuy { npc_id: npc_id.clone(), item_id: item_id.clone(), quantity: *quantity },
            InputCommand::ShopSell { npc_id, item_id, quantity } => ClientMessage::ShopSell { npc_id: npc_id.clone(), item_id: item_id.clone(), quantity: *quantity },
            InputCommand::EnterPortal { portal_id } => ClientMessage::EnterPortal { portal_id: portal_id.clone() },
        };
        network.send(&msg);
    }

    // Process pending portal trigger
    if let Some(portal_id) = game_state.pending_portal_id.take() {
        network.send(&crate::network::messages::ClientMessage::EnterPortal { portal_id });
    }

    // Record delta for diagnostics
    game_state.frame_timings.record_delta(delta as f64 * 1000.0);

    // 4. Update game state
    let update_start = get_time();
    let (input_dx, input_dy) = input_handler.get_movement();
    game_state.update(delta, input_dx, input_dy);
    game_state.update_transition(delta);
    let update_ms = (get_time() - update_start) * 1000.0;

    // 4b. Request chunks around player position
    if let Some(player) = game_state.get_local_player() {
        let chunks_to_request = game_state.chunk_manager.update_player_position(player.x, player.y);
        for coord in chunks_to_request {
            network.send(&crate::network::messages::ClientMessage::RequestChunk {
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

    // 5. Debug info
    if game_state.debug_mode {
        let fps_cap_str = match game_state.frame_timings.fps_cap {
            Some(cap) => format!(" (cap: {})", cap),
            None => " (uncapped)".to_string(),
        };
        renderer.draw_text_sharp(&format!("FPS: {}{} [F4]", get_fps(), fps_cap_str), 10.0, 20.0, 16.0, WHITE);
        renderer.draw_text_sharp(&format!("Players: {}", game_state.players.len()), 10.0, 40.0, 16.0, WHITE);
        renderer.draw_text_sharp(&format!("Connected: {}", network.is_connected()), 10.0, 60.0, 16.0, WHITE);

        if let Some(player) = game_state.get_local_player() {
            let chunk_x = (player.x / 32.0).floor() as i32;
            let chunk_y = (player.y / 32.0).floor() as i32;
            renderer.draw_text_sharp(&format!("Pos: ({:.1}, {:.1})", player.x, player.y), 10.0, 80.0, 16.0, YELLOW);
            renderer.draw_text_sharp(&format!("Chunk: ({}, {})", chunk_x, chunk_y), 10.0, 100.0, 16.0, YELLOW);
            renderer.draw_text_sharp(&format!("NPCs: {}", game_state.npcs.len()), 10.0, 120.0, 16.0, WHITE);
            renderer.draw_text_sharp(&format!("Appearance: {} {} (F5/F6 to cycle)", player.gender, player.skin), 10.0, 140.0, 16.0, Color::from_rgba(150, 200, 255, 255));
        }

        let t = &game_state.frame_timings;
        let timing_color = Color::from_rgba(100, 255, 150, 255);
        let spike_color = Color::from_rgba(255, 100, 100, 255);
        renderer.draw_text_sharp("--- Frame Timing (ms) ---", 10.0, 170.0, 16.0, timing_color);
        renderer.draw_text_sharp(&format!("Network:  {:.2}", t.network_ms), 10.0, 190.0, 16.0, timing_color);

        let ground_color = if t.render_ground_ms > 0.5 { spike_color } else { timing_color };
        let entities_color = if t.render_entities_ms > 0.5 { spike_color } else { timing_color };
        let overhead_color = if t.render_overhead_ms > 0.5 { spike_color } else { timing_color };
        let effects_color = if t.render_effects_ms > 0.5 { spike_color } else { timing_color };
        let ui_color = if t.render_ui_ms > 0.5 { spike_color } else { timing_color };

        renderer.draw_text_sharp(&format!("Render:   {:.2} (total)", t.render_total_ms), 10.0, 210.0, 16.0, timing_color);
        renderer.draw_text_sharp(&format!("  Ground:   {:.2}", t.render_ground_ms), 10.0, 230.0, 16.0, ground_color);
        renderer.draw_text_sharp(&format!("  Entities: {:.2}", t.render_entities_ms), 10.0, 250.0, 16.0, entities_color);
        renderer.draw_text_sharp(&format!("  Overhead: {:.2}", t.render_overhead_ms), 10.0, 270.0, 16.0, overhead_color);
        renderer.draw_text_sharp(&format!("  Effects:  {:.2}", t.render_effects_ms), 10.0, 290.0, 16.0, effects_color);
        renderer.draw_text_sharp(&format!("  UI:       {:.2}", t.render_ui_ms), 10.0, 310.0, 16.0, ui_color);

        renderer.draw_text_sharp(&format!("Update:   {:.2}", t.update_ms), 10.0, 330.0, 16.0, timing_color);
        renderer.draw_text_sharp(&format!("Total:    {:.2}", t.total_ms), 10.0, 350.0, 16.0, timing_color);
        renderer.draw_text_sharp(&format!("Entities: {} | Chunks: {}", t.entity_count, t.chunk_count), 10.0, 370.0, 16.0, timing_color);

        let delta_variance = t.delta_max_ms - t.delta_min_ms;
        let variance_color = if delta_variance > 5.0 { spike_color } else { timing_color };
        renderer.draw_text_sharp(&format!("Delta: {:.1}ms (range: {:.1}-{:.1}, var: {:.1})",
            t.delta_ms, t.delta_min_ms, t.delta_max_ms, delta_variance), 10.0, 390.0, 16.0, variance_color);

        let nf_variance = t.next_frame_max_ms - t.next_frame_min_ms;
        let nf_color = if nf_variance > 5.0 { spike_color } else { timing_color };
        renderer.draw_text_sharp(&format!("next_frame(): {:.1}ms (range: {:.1}-{:.1}, var: {:.1})",
            t.next_frame_ms, t.next_frame_min_ms, t.next_frame_max_ms, nf_variance), 10.0, 410.0, 16.0, nf_color);

        let smooth_str = if t.delta_smoothing > 0.0 {
            format!("{:.1}", t.delta_smoothing)
        } else {
            "off".to_string()
        };
        renderer.draw_text_sharp(&format!("Smoothing: {} [F7] (smoothed: {:.1}ms)",
            smooth_str, t.smoothed_delta * 1000.0), 10.0, 430.0, 16.0, timing_color);
    }

    // 6. Render transition overlay
    renderer.render_transition_overlay(game_state);

    // 7. Render touch controls (mobile only)
    // Hide action buttons when panels are open
    let hide_action_buttons = game_state.ui_state.inventory_open
        || game_state.ui_state.character_panel_open
        || game_state.ui_state.skills_open;
    input_handler.render_touch_controls(hide_action_buttons);
}
