use macroquad::prelude::*;
use crate::game::GameState;
use crate::render::isometric::screen_to_world;

/// Input commands that can be sent to the server
#[derive(Debug, Clone)]
pub enum InputCommand {
    Move { dx: f32, dy: f32 },
    Attack,
    Target { entity_id: String },
    ClearTarget,
    Chat { text: String },
    Pickup { item_id: String },
    UseItem { slot_index: u8 },
}

/// Cardinal directions for isometric movement (no diagonals)
#[derive(Clone, Copy, PartialEq, Eq)]
enum CardinalDir {
    None,
    Up,
    Down,
    Left,
    Right,
}

pub struct InputHandler {
    // Track last sent velocity to detect changes
    last_dx: f32,
    last_dy: f32,
    // Track which direction was pressed first (for priority)
    current_dir: CardinalDir,
    // Send commands at server tick rate
    last_send_time: f64,
    send_interval: f64,
    // Attack cooldown tracking (matches server cooldown)
    last_attack_time: f64,
    attack_cooldown: f64,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            last_dx: 0.0,
            last_dy: 0.0,
            current_dir: CardinalDir::None,
            last_send_time: 0.0,
            send_interval: 0.05, // 50ms = 20Hz (matches server tick rate)
            last_attack_time: 0.0,
            attack_cooldown: 1.0, // 1 second (matches server ATTACK_COOLDOWN_MS)
        }
    }

    pub fn process(&mut self, state: &mut GameState) -> Vec<InputCommand> {
        let mut commands = Vec::new();
        let current_time = get_time();

        // Toggle debug mode
        if is_key_pressed(KeyCode::F3) {
            // Debug toggle handled in main loop
        }

        // Handle chat input mode
        if state.ui_state.chat_open {
            // Escape cancels chat
            if is_key_pressed(KeyCode::Escape) {
                state.ui_state.chat_open = false;
                state.ui_state.chat_input.clear();
                return commands;
            }

            // Enter sends message
            if is_key_pressed(KeyCode::Enter) {
                let text = state.ui_state.chat_input.trim().to_string();
                if !text.is_empty() {
                    commands.push(InputCommand::Chat { text });
                }
                state.ui_state.chat_open = false;
                state.ui_state.chat_input.clear();
                return commands;
            }

            // Backspace removes last character
            if is_key_pressed(KeyCode::Backspace) {
                state.ui_state.chat_input.pop();
            }

            // Capture typed characters
            while let Some(c) = get_char_pressed() {
                // Filter control characters
                if c.is_control() {
                    continue;
                }
                // Limit chat message length
                if state.ui_state.chat_input.len() < 200 {
                    state.ui_state.chat_input.push(c);
                }
            }

            return commands;
        }

        // Enter key opens chat
        if is_key_pressed(KeyCode::Enter) {
            state.ui_state.chat_open = true;
            state.ui_state.chat_input.clear();
            return commands;
        }

        // Read which keys are held
        let up = is_key_down(KeyCode::W) || is_key_down(KeyCode::Up);
        let down = is_key_down(KeyCode::S) || is_key_down(KeyCode::Down);
        let left = is_key_down(KeyCode::A) || is_key_down(KeyCode::Left);
        let right = is_key_down(KeyCode::D) || is_key_down(KeyCode::Right);

        // Determine new direction - only one direction at a time
        // Priority: keep current direction if still held, otherwise pick new one
        let new_dir = match self.current_dir {
            CardinalDir::Up if up => CardinalDir::Up,
            CardinalDir::Down if down => CardinalDir::Down,
            CardinalDir::Left if left => CardinalDir::Left,
            CardinalDir::Right if right => CardinalDir::Right,
            _ => {
                // Current direction released, pick a new one (priority order)
                if up { CardinalDir::Up }
                else if down { CardinalDir::Down }
                else if left { CardinalDir::Left }
                else if right { CardinalDir::Right }
                else { CardinalDir::None }
            }
        };

        self.current_dir = new_dir;

        // Convert direction to velocity
        let (dx, dy): (f32, f32) = match new_dir {
            CardinalDir::Up => (0.0, -1.0),
            CardinalDir::Down => (0.0, 1.0),
            CardinalDir::Left => (-1.0, 0.0),
            CardinalDir::Right => (1.0, 0.0),
            CardinalDir::None => (0.0, 0.0),
        };

        // Check if movement direction changed
        let direction_changed = (dx - self.last_dx).abs() > 0.01 || (dy - self.last_dy).abs() > 0.01;
        let time_elapsed = current_time - self.last_send_time >= self.send_interval;

        // Send command if: direction changed OR (moving AND enough time passed)
        let should_send = direction_changed || (time_elapsed && (dx != 0.0 || dy != 0.0));

        if should_send {
            commands.push(InputCommand::Move { dx, dy });
            self.last_dx = dx;
            self.last_dy = dy;
            self.last_send_time = current_time;
        }

        // Attack (Space key) - holding space continues attacking with cooldown
        if is_key_down(KeyCode::Space) {
            if current_time - self.last_attack_time >= self.attack_cooldown {
                log::info!("Space held - sending Attack command");
                commands.push(InputCommand::Attack);
                self.last_attack_time = current_time;
            }
        }

        // Target selection (left click)
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            let (world_x, world_y) = screen_to_world(mouse_x, mouse_y, &state.camera);

            // Find nearest entity within click radius (1.5 tiles)
            const CLICK_RADIUS: f32 = 1.5;
            let mut nearest_entity: Option<(String, f32)> = None;

            // Check players
            for (id, player) in &state.players {
                // Don't allow targeting self
                if state.local_player_id.as_ref() == Some(id) {
                    continue;
                }

                let dx: f32 = player.x - world_x;
                let dy: f32 = player.y - world_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < CLICK_RADIUS {
                    if nearest_entity.is_none() || dist < nearest_entity.as_ref().unwrap().1 {
                        nearest_entity = Some((id.clone(), dist));
                    }
                }
            }

            // Check NPCs (prioritize over players for targeting)
            for (id, npc) in &state.npcs {
                // Only allow targeting alive NPCs
                if !npc.is_alive() {
                    continue;
                }

                let dx: f32 = npc.x - world_x;
                let dy: f32 = npc.y - world_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < CLICK_RADIUS {
                    if nearest_entity.is_none() || dist < nearest_entity.as_ref().unwrap().1 {
                        nearest_entity = Some((id.clone(), dist));
                    }
                }
            }

            if let Some((entity_id, _)) = nearest_entity {
                commands.push(InputCommand::Target { entity_id });
            } else {
                // Clicked on empty space - clear target
                if state.selected_entity_id.is_some() {
                    commands.push(InputCommand::ClearTarget);
                }
            }
        }

        // Clear target with Escape
        if is_key_pressed(KeyCode::Escape) {
            if state.selected_entity_id.is_some() {
                commands.push(InputCommand::ClearTarget);
            }
        }

        // Toggle inventory (I key)
        if is_key_pressed(KeyCode::I) {
            state.ui_state.inventory_open = !state.ui_state.inventory_open;
        }

        // Use items (1-5 keys for quick slots)
        if is_key_pressed(KeyCode::Key1) {
            commands.push(InputCommand::UseItem { slot_index: 0 });
        }
        if is_key_pressed(KeyCode::Key2) {
            commands.push(InputCommand::UseItem { slot_index: 1 });
        }
        if is_key_pressed(KeyCode::Key3) {
            commands.push(InputCommand::UseItem { slot_index: 2 });
        }
        if is_key_pressed(KeyCode::Key4) {
            commands.push(InputCommand::UseItem { slot_index: 3 });
        }
        if is_key_pressed(KeyCode::Key5) {
            commands.push(InputCommand::UseItem { slot_index: 4 });
        }

        // Pickup nearest item (F key)
        if is_key_pressed(KeyCode::F) {
            // Get local player position
            if let Some(local_id) = &state.local_player_id {
                if let Some(player) = state.players.get(local_id) {
                    // Find nearest item within pickup range (2 tiles)
                    const PICKUP_RANGE: f32 = 2.0;
                    let mut nearest_item: Option<(String, f32)> = None;

                    for (id, item) in &state.ground_items {
                        let dx = item.x - player.x;
                        let dy = item.y - player.y;
                        let dist = (dx * dx + dy * dy).sqrt();

                        if dist < PICKUP_RANGE {
                            if nearest_item.is_none() || dist < nearest_item.as_ref().unwrap().1 {
                                nearest_item = Some((id.clone(), dist));
                            }
                        }
                    }

                    if let Some((item_id, _)) = nearest_item {
                        commands.push(InputCommand::Pickup { item_id });
                    }
                }
            }
        }

        commands
    }

    /// Get current movement direction (for client-side prediction)
    pub fn get_movement(&self) -> (f32, f32) {
        (self.last_dx, self.last_dy)
    }
}
