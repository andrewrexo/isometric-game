use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use serde::{Deserialize, Serialize};
use crate::game::{GameState, ConnectionStatus, Player, Direction, ChatChannel, ChatMessage, ChatBubble, DamageEvent, LevelUpEvent, SkillXpEvent, GroundItem, InventorySlot, ActiveDialogue, DialogueChoice, ActiveQuest, QuestObjective, QuestCompletedEvent, RecipeDefinition, RecipeIngredient, RecipeResult, ItemDefinition, EquipmentStats, MapObject, ShopData, ShopStockItem, SkillType, Wall, WallEdge, Portal, TransitionState};
use crate::game::npc::{Npc, NpcState};
use crate::render::OVERWORLD_NAME;
use super::messages::ClientMessage;
use super::protocol::{self, DecodedMessage, extract_string, extract_f32, extract_i32, extract_u32, extract_u64, extract_array, extract_u8, extract_bool};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchmakeResponse {
    room: RoomInfo,
    /// Signed session token for secure WebSocket upgrade
    session_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoomInfo {
    room_id: String,
}

#[derive(Debug, Serialize)]
struct JoinOptions {
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticatedJoinOptions {
    character_id: i64,
}

#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    Disconnected,
    Matchmaking,
    Connecting,
    Connected,
}

const MAX_RECONNECT_ATTEMPTS: u32 = 3;

pub struct NetworkClient {
    sender: Option<WsSender>,
    receiver: Option<WsReceiver>,
    base_url: String,
    player_name: String,
    connection_state: ConnectionState,
    reconnect_timer: f32,
    room_id: Option<String>,
    /// Signed session token for secure WebSocket upgrade
    session_token: Option<String>,
    // Auth fields
    auth_token: Option<String>,
    character_id: Option<i64>,
    // Reconnection tracking
    reconnect_attempts: u32,
    was_connected: bool,
}

impl NetworkClient {
    /// Legacy constructor - for backwards compatibility
    pub fn new(base_url: &str) -> Self {
        Self::new_guest(base_url)
    }

    /// Create a guest mode client (dev mode only)
    pub fn new_guest(base_url: &str) -> Self {
        let mut client = Self {
            sender: None,
            receiver: None,
            base_url: base_url.to_string(),
            player_name: format!("Guest{}", rand::random::<u16>() % 10000),
            connection_state: ConnectionState::Disconnected,
            reconnect_timer: 0.0,
            room_id: None,
            session_token: None,
            auth_token: None,
            character_id: None,
            reconnect_attempts: 0,
            was_connected: false,
        };
        client.start_matchmaking();
        client
    }

    /// Create an authenticated client with a specific character
    pub fn new_authenticated(base_url: &str, auth_token: &str, character_id: i64) -> Self {
        let mut client = Self {
            sender: None,
            receiver: None,
            base_url: base_url.to_string(),
            player_name: String::new(), // Will be set by server from character
            connection_state: ConnectionState::Disconnected,
            reconnect_timer: 0.0,
            room_id: None,
            session_token: None,
            auth_token: Some(auth_token.to_string()),
            character_id: Some(character_id),
            reconnect_attempts: 0,
            was_connected: false,
        };
        client.start_matchmaking();
        client
    }

    /// Create a client with auth token (simple account=character model)
    pub fn new_with_token(base_url: &str, auth_token: &str, player_name: &str) -> Self {
        let mut client = Self {
            sender: None,
            receiver: None,
            base_url: base_url.to_string(),
            player_name: player_name.to_string(),
            connection_state: ConnectionState::Disconnected,
            reconnect_timer: 0.0,
            room_id: None,
            session_token: None,
            auth_token: Some(auth_token.to_string()),
            character_id: None, // Not used in simple model
            reconnect_attempts: 0,
            was_connected: false,
        };
        client.start_matchmaking();
        client
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_matchmaking(&mut self) {
        self.connection_state = ConnectionState::Matchmaking;

        let http_url = self.base_url
            .replace("ws://", "http://")
            .replace("wss://", "https://");

        let matchmake_url = format!("{}/matchmake/joinOrCreate/game_room", http_url);

        // Build request - auth is required
        let request = ureq::post(&matchmake_url)
            .set("Content-Type", "application/json");

        let result = if let Some(token) = &self.auth_token {
            // Authenticated matchmaking - must have character_id
            if let Some(char_id) = self.character_id {
                log::info!("Matchmaking (authenticated): POST {} with character_id={}", matchmake_url, char_id);
                let options = AuthenticatedJoinOptions {
                    character_id: char_id,
                };
                request
                    .set("Authorization", &format!("Bearer {}", token))
                    .send_json(&options)
            } else {
                log::error!("Matchmaking failed: No character_id. Select a character first.");
                self.connection_state = ConnectionState::Disconnected;
                return;
            }
        } else {
            // No auth token - this will fail with 401
            log::error!("Matchmaking failed: No auth token. Login required.");
            self.connection_state = ConnectionState::Disconnected;
            return;
        };

        match result {
            Ok(response) => {
                match response.into_json::<MatchmakeResponse>() {
                    Ok(data) => {
                        log::info!("Matchmaking success: room={}", data.room.room_id);
                        self.room_id = Some(data.room.room_id);
                        self.session_token = Some(data.session_token);
                        self.connect_websocket();
                    }
                    Err(e) => {
                        log::error!("Failed to parse matchmake response: {}", e);
                        self.connection_state = ConnectionState::Disconnected;
                    }
                }
            }
            Err(e) => {
                log::error!("Matchmaking failed: {}", e);
                self.connection_state = ConnectionState::Disconnected;
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn start_matchmaking(&mut self) {
        // In WASM, JavaScript does matchmaking before loading WASM
        // and stores the result in localStorage
        let room_id = quad_storage::STORAGE.lock().unwrap().get("roomId");
        let session_token = quad_storage::STORAGE.lock().unwrap().get("sessionToken");

        if let (Some(rid), Some(token)) = (room_id, session_token) {
            log::info!("WASM: Connecting with roomId={}", rid);
            self.room_id = Some(rid);
            self.session_token = Some(token);
            self.connect_websocket();
        } else {
            log::error!("WASM: Missing roomId or sessionToken in localStorage. JavaScript should matchmake first.");
            self.connection_state = ConnectionState::Disconnected;
        }
    }

    fn connect_websocket(&mut self) {
        let room_id = match &self.room_id {
            Some(id) => id,
            None => return,
        };
        let token = match &self.session_token {
            Some(t) => t,
            None => {
                log::error!("No session token available");
                return;
            }
        };

        let ws_url = format!("{}/{}?sessionToken={}", self.base_url, room_id, token);
        log::info!("Connecting WebSocket: {}...", &ws_url[..ws_url.len().min(80)]);

        self.connection_state = ConnectionState::Connecting;

        match ewebsock::connect(&ws_url, ewebsock::Options::default()) {
            Ok((sender, receiver)) => {
                // Colyseus recognizes the sessionId from the URL and auto-consumes the seat
                // No need to send Protocol.JOIN_ROOM for fresh joins
                log::debug!("WebSocket connection initiated");
                self.sender = Some(sender);
                self.receiver = Some(receiver);
            }
            Err(e) => {
                log::error!("WebSocket connection failed: {}", e);
                self.connection_state = ConnectionState::Disconnected;
            }
        }
    }

    pub fn poll(&mut self, state: &mut GameState) {
        match self.connection_state {
            ConnectionState::Disconnected => {
                // Only try to reconnect if we were previously connected
                if self.was_connected {
                    // Check if we've exhausted reconnection attempts
                    if self.reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                        log::error!("Failed to reconnect after {} attempts", MAX_RECONNECT_ATTEMPTS);
                        state.reconnection_failed = true;
                        return;
                    }

                    self.reconnect_timer += 1.0 / 60.0;
                    if self.reconnect_timer > 2.0 {
                        self.reconnect_attempts += 1;
                        log::info!("Reconnection attempt {}/{}", self.reconnect_attempts, MAX_RECONNECT_ATTEMPTS);
                        self.reconnect_timer = 0.0;
                        self.start_matchmaking();
                    }
                } else {
                    // Initial connection - just retry without counting
                    self.reconnect_timer += 1.0 / 60.0;
                    if self.reconnect_timer > 2.0 {
                        self.reconnect_timer = 0.0;
                        self.start_matchmaking();
                    }
                }
                return;
            }
            ConnectionState::Matchmaking => return,
            ConnectionState::Connecting | ConnectionState::Connected => {}
        }

        if self.receiver.is_none() {
            self.connection_state = ConnectionState::Disconnected;
            return;
        }

        // Collect events from receiver, handling potential panics from the websocket library
        let mut events = Vec::new();
        let mut receiver_failed = false;
        if let Some(receiver) = &self.receiver {
            // Use AssertUnwindSafe since we're handling the failure case
            let receiver_ref = std::panic::AssertUnwindSafe(receiver);
            let result = std::panic::catch_unwind(|| {
                let mut collected = Vec::new();
                while let Some(event) = receiver_ref.try_recv() {
                    collected.push(event);
                }
                collected
            });

            match result {
                Ok(collected) => events = collected,
                Err(_) => {
                    log::error!("WebSocket receiver panicked - treating as disconnect");
                    receiver_failed = true;
                }
            }
        }

        // If the receiver panicked, treat it as a disconnect
        if receiver_failed {
            self.connection_state = ConnectionState::Disconnected;
            state.connection_status = ConnectionStatus::Disconnected;
            self.sender = None;
            self.receiver = None;
            self.room_id = None;
            self.session_token = None;
            return;
        }

        let mut should_disconnect = false;
        for event in events {
            match event {
                WsEvent::Opened => {
                    log::info!("WebSocket connected!");
                    self.connection_state = ConnectionState::Connected;
                    state.connection_status = ConnectionStatus::Connected;
                    // Reset reconnection tracking on successful connection
                    self.reconnect_attempts = 0;
                    self.was_connected = true;
                    // local_player_id is set by the "welcome" message from server
                }

                WsEvent::Message(WsMessage::Binary(bytes)) => {
                    self.handle_binary_message(&bytes, state);
                }

                WsEvent::Message(WsMessage::Text(text)) => {
                    // Colyseus shouldn't send text, but log it for debugging
                    log::debug!("Received text message: {}", text);
                }

                WsEvent::Closed => {
                    log::info!("WebSocket disconnected");
                    self.connection_state = ConnectionState::Disconnected;
                    state.connection_status = ConnectionStatus::Disconnected;
                    should_disconnect = true;
                }

                WsEvent::Error(err) => {
                    log::error!("WebSocket error: {}", err);
                    // Treat errors like disconnects to trigger reconnection
                    self.connection_state = ConnectionState::Disconnected;
                    state.connection_status = ConnectionStatus::Disconnected;
                    should_disconnect = true;
                }

                _ => {}
            }
        }

        if should_disconnect {
            self.sender = None;
            self.receiver = None;
            self.room_id = None;
            self.session_token = None;
        }
    }

    fn handle_binary_message(&self, data: &[u8], state: &mut GameState) {
        log::trace!("Received {} bytes: {:?}", data.len(), &data[..data.len().min(50)]);
        match protocol::decode_message(data) {
            Ok(decoded) => {
                match decoded {
                    DecodedMessage::RoomData { msg_type, data } => {
                        self.handle_room_data(&msg_type, data.as_ref(), state);
                    }
                    DecodedMessage::RoomState { .. } => {
                        // Colyseus Schema state - we're using custom JSON messages instead
                        log::debug!("Received RoomState (ignored - using custom messages)");
                    }
                    DecodedMessage::RoomStatePatch { .. } => {
                        // Colyseus Schema patch - ignored
                        log::debug!("Received RoomStatePatch (ignored)");
                    }
                    DecodedMessage::Error { code, message } => {
                        log::error!("Server error {}: {}", code, message);
                    }
                    DecodedMessage::Handshake => {
                        log::debug!("Received Handshake");
                    }
                    DecodedMessage::Unknown { protocol, .. } => {
                        log::debug!("Unknown protocol: {}", protocol);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to decode message: {}", e);
            }
        }
    }

    fn handle_room_data(&self, msg_type: &str, data: Option<&rmpv::Value>, state: &mut GameState) {
        match msg_type {
            "welcome" => {
                if let Some(value) = data {
                    if let Some(player_id) = extract_string(value, "player_id") {
                        log::info!("Welcome! Player ID: {}", player_id);
                        state.local_player_id = Some(player_id);
                        state.connection_status = ConnectionStatus::Connected;
                    }
                }
            }

            "playerJoined" => {
                if let Some(value) = data {
                    let id = extract_string(value, "id").unwrap_or_default();
                    let name = extract_string(value, "name").unwrap_or_default();
                    // Server sends i32 grid positions
                    let x = extract_i32(value, "x").unwrap_or(0) as f32;
                    let y = extract_i32(value, "y").unwrap_or(0) as f32;
                    // Appearance
                    let gender = extract_string(value, "gender").unwrap_or_else(|| "male".to_string());
                    let skin = extract_string(value, "skin").unwrap_or_else(|| "tan".to_string());
                    let hair_style = extract_i32(value, "hair_style");
                    let hair_color = extract_i32(value, "hair_color");
                    // Equipment (filter empty strings to None)
                    let equipped_head = extract_string(value, "equipped_head").filter(|s| !s.is_empty());
                    let equipped_body = extract_string(value, "equipped_body").filter(|s| !s.is_empty());
                    let equipped_weapon = extract_string(value, "equipped_weapon").filter(|s| !s.is_empty());
                    let equipped_back = extract_string(value, "equipped_back").filter(|s| !s.is_empty());
                    let equipped_feet = extract_string(value, "equipped_feet").filter(|s| !s.is_empty());
                    let equipped_ring = extract_string(value, "equipped_ring").filter(|s| !s.is_empty());
                    let equipped_gloves = extract_string(value, "equipped_gloves").filter(|s| !s.is_empty());
                    let equipped_necklace = extract_string(value, "equipped_necklace").filter(|s| !s.is_empty());
                    let equipped_belt = extract_string(value, "equipped_belt").filter(|s| !s.is_empty());
                    // Admin status
                    let is_admin = extract_bool(value, "is_admin").unwrap_or(false);

                    log::info!("Player joined: {} at ({}, {}) [{}/{}]", name, x, y, gender, skin);
                    let mut player = Player::new(id.clone(), name, x, y, gender, skin);
                    player.hair_style = hair_style;
                    player.hair_color = hair_color;
                    player.equipped_head = equipped_head;
                    player.equipped_body = equipped_body;
                    player.equipped_weapon = equipped_weapon;
                    player.equipped_back = equipped_back;
                    player.equipped_feet = equipped_feet;
                    player.equipped_ring = equipped_ring;
                    player.equipped_gloves = equipped_gloves;
                    player.equipped_necklace = equipped_necklace;
                    player.equipped_belt = equipped_belt;
                    player.is_admin = is_admin;
                    state.players.insert(id, player);
                }
            }

            "playerLeft" => {
                if let Some(value) = data {
                    if let Some(id) = extract_string(value, "id") {
                        log::info!("Player left: {}", id);
                        state.players.remove(&id);
                    }
                }
            }

            "stateSync" => {
                if let Some(value) = data {
                    let tick = extract_u64(value, "tick").unwrap_or(0);

                    // Only process newer ticks
                    if tick >= state.server_tick {
                        state.server_tick = tick;
                    }

                    // Update players (grid positions from server)
                    let mut player_regen_events: Vec<(String, f32, f32, i32)> = Vec::new();
                    if let Some(players) = extract_array(value, "players") {
                        for player_value in players {
                            let id = extract_string(player_value, "id").unwrap_or_default();
                            let name = extract_string(player_value, "name").unwrap_or_default();
                            // Server sends i32 grid positions
                            let x = extract_i32(player_value, "x");
                            let y = extract_i32(player_value, "y");
                            let direction = extract_i32(player_value, "direction");
                            let hp = extract_i32(player_value, "hp");
                            let max_hp = extract_i32(player_value, "maxHp");
                            // Skill levels (consolidated combat system)
                            let hitpoints_level = extract_i32(player_value, "hitpointsLevel");
                            let combat_skill_level = extract_i32(player_value, "combatSkillLevel");
                            let gold = extract_i32(player_value, "gold");
                            let gender = extract_string(player_value, "gender").unwrap_or_else(|| "male".to_string());
                            let skin = extract_string(player_value, "skin").unwrap_or_else(|| "tan".to_string());
                            let hair_style = extract_i32(player_value, "hair_style");
                            let hair_color = extract_i32(player_value, "hair_color");
                            let equipped_head = extract_string(player_value, "equipped_head").filter(|s| !s.is_empty());
                            let equipped_body = extract_string(player_value, "equipped_body").filter(|s| !s.is_empty());
                            let equipped_weapon = extract_string(player_value, "equipped_weapon").filter(|s| !s.is_empty());
                            let equipped_back = extract_string(player_value, "equipped_back").filter(|s| !s.is_empty());
                            let equipped_feet = extract_string(player_value, "equipped_feet").filter(|s| !s.is_empty());
                            let equipped_ring = extract_string(player_value, "equipped_ring").filter(|s| !s.is_empty());
                            let equipped_gloves = extract_string(player_value, "equipped_gloves").filter(|s| !s.is_empty());
                            let equipped_necklace = extract_string(player_value, "equipped_necklace").filter(|s| !s.is_empty());
                            let equipped_belt = extract_string(player_value, "equipped_belt").filter(|s| !s.is_empty());
                            let is_admin = extract_bool(player_value, "is_admin").unwrap_or(false);

                            let is_local_player = state.local_player_id.as_ref() == Some(&id);

                            if let Some(player) = state.players.get_mut(&id) {
                                // Read velocity for client-side prediction
                                let vel_x = extract_i32(player_value, "velX").unwrap_or(0) as f32;
                                let vel_y = extract_i32(player_value, "velY").unwrap_or(0) as f32;

                                if let (Some(x), Some(y)) = (x, y) {
                                    // Set server target with velocity for prediction
                                    player.set_server_position_with_velocity(x as f32, y as f32, vel_x, vel_y);
                                }
                                if let Some(dir) = direction {
                                    // For local player: only update direction from server when safe
                                    // This prevents flickering when client prediction differs from server state
                                    let should_update_direction = if is_local_player {
                                        let current_time = macroquad::time::get_time();
                                        let time_since_face = current_time - state.last_face_command_time;
                                        // Only accept server direction when:
                                        // 1. Stationary (no velocity), AND
                                        // 2. Enough time has passed since we sent a Face command (200ms grace period)
                                        vel_x == 0.0 && vel_y == 0.0 && time_since_face > 0.2
                                    } else {
                                        true // Always update other players from server
                                    };

                                    if should_update_direction {
                                        let new_dir = Direction::from_u8(dir as u8);
                                        player.direction = new_dir;
                                        // For local player: also update animation.direction directly
                                        // (local player has special handling in state.rs update())
                                        // For remote players: let interpolate_visual() handle animation.direction
                                        // to prevent moonwalking (direction changing before movement catches up)
                                        if is_local_player {
                                            player.animation.direction = new_dir;
                                        }
                                    }
                                }
                                if let Some(hp) = hp {
                                    // Update last_damage_time if HP decreased (ensures HP bar shows)
                                    if hp < player.hp {
                                        player.last_damage_time = macroquad::time::get_time();
                                    } else if hp > player.hp && player.hp > 0 {
                                        // HP increased (regen) - record for floating text
                                        let heal_amount = hp - player.hp;
                                        player_regen_events.push((id.clone(), player.x, player.y, heal_amount));
                                    }
                                    player.hp = hp;
                                }
                                if let Some(max_hp) = max_hp {
                                    player.max_hp = max_hp;
                                }
                                // Update skill levels
                                if let Some(level) = hitpoints_level {
                                    player.skills.hitpoints.level = level;
                                }
                                if let Some(level) = combat_skill_level {
                                    player.skills.combat.level = level;
                                }
                                // Update hair
                                player.hair_style = hair_style;
                                player.hair_color = hair_color;
                                // Update equipment
                                player.equipped_head = equipped_head.clone();
                                player.equipped_body = equipped_body.clone();
                                player.equipped_weapon = equipped_weapon.clone();
                                player.equipped_back = equipped_back.clone();
                                player.equipped_feet = equipped_feet.clone();
                                player.equipped_ring = equipped_ring.clone();
                                player.equipped_gloves = equipped_gloves.clone();
                                player.equipped_necklace = equipped_necklace.clone();
                                player.equipped_belt = equipped_belt.clone();
                                // Update admin status
                                player.is_admin = is_admin;
                            } else if !is_local_player && !id.is_empty() {
                                // Player not in our map - create them from stateSync data
                                // This handles players re-appearing after map transitions
                                if let (Some(px), Some(py)) = (x, y) {
                                    log::info!("Creating player from stateSync: {} at ({}, {})", name, px, py);
                                    let mut new_player = Player::new(
                                        id.clone(),
                                        name.clone(),
                                        px as f32,
                                        py as f32,
                                        gender,
                                        skin,
                                    );
                                    new_player.hair_style = hair_style;
                                    new_player.hair_color = hair_color;
                                    new_player.equipped_head = equipped_head;
                                    new_player.equipped_body = equipped_body;
                                    new_player.equipped_weapon = equipped_weapon;
                                    new_player.equipped_back = equipped_back;
                                    new_player.equipped_feet = equipped_feet;
                                    new_player.equipped_ring = equipped_ring;
                                    new_player.equipped_gloves = equipped_gloves;
                                    new_player.equipped_necklace = equipped_necklace;
                                    new_player.equipped_belt = equipped_belt;
                                    new_player.is_admin = is_admin;
                                    if let Some(hp_val) = hp {
                                        new_player.hp = hp_val;
                                    }
                                    if let Some(max_hp_val) = max_hp {
                                        new_player.max_hp = max_hp_val;
                                    }
                                    if let Some(dir) = direction {
                                        let new_dir = Direction::from_u8(dir as u8);
                                        new_player.direction = new_dir;
                                        new_player.animation.direction = new_dir;
                                    }
                                    state.players.insert(id.clone(), new_player);
                                }
                            }

                            // Update inventory gold for local player
                            if state.local_player_id.as_ref() == Some(&id) {
                                if let Some(gold) = gold {
                                    state.inventory.gold = gold;
                                }
                            }
                        }
                    }

                    // Check if local player walked onto a portal (auto-trigger)
                    // Only triggers when player moves to a new tile, not when spawning on one
                    if let Some(local_id) = &state.local_player_id {
                        if let Some(player) = state.players.get(local_id) {
                            let current_tile = (player.x.floor() as i32, player.y.floor() as i32);
                            let prev_tile = state.last_portal_check_pos;

                            // Only check for portal if we moved to a different tile
                            let moved_tiles = prev_tile.map_or(false, |prev| prev != current_tile);

                            if moved_tiles &&
                               state.pending_portal_id.is_none() &&
                               matches!(state.map_transition.state, TransitionState::None) {
                                if let Some(portal) = state.chunk_manager.get_portal_at(player.x, player.y) {
                                    state.pending_portal_id = Some(portal.id.clone());
                                }
                            }

                            // Always update last checked position
                            state.last_portal_check_pos = Some(current_tile);
                        }
                    }

                    // Push player regen events as healing numbers (negative damage = green +X)
                    let current_time = macroquad::time::get_time();
                    for (target_id, x, y, heal_amount) in player_regen_events {
                        state.damage_events.push(DamageEvent {
                            x,
                            y,
                            damage: -heal_amount, // Negative = healing
                            time: current_time,
                            target_id,
                            source_id: None,
                            projectile: None,
                        });
                    }

                    // Update NPCs (grid positions from server, converted to f32 for interpolation)
                    let mut npc_regen_events: Vec<(String, f32, f32, i32)> = Vec::new();
                    if let Some(npcs) = extract_array(value, "npcs") {
                        for npc_value in npcs {
                            let id = extract_string(npc_value, "id").unwrap_or_default();
                            let npc_type = extract_u8(npc_value, "npc_type").unwrap_or(0);
                            let entity_type = extract_string(npc_value, "entity_type").unwrap_or_else(|| "pig".to_string());
                            let display_name = extract_string(npc_value, "display_name").unwrap_or_else(|| "???".to_string());
                            // Server sends i32 grid positions
                            let x = extract_i32(npc_value, "x").unwrap_or(0) as f32;
                            let y = extract_i32(npc_value, "y").unwrap_or(0) as f32;
                            let direction = extract_u8(npc_value, "direction").unwrap_or(0);
                            let hp = extract_i32(npc_value, "hp").unwrap_or(50);
                            let max_hp = extract_i32(npc_value, "max_hp").unwrap_or(50);
                            let level = extract_i32(npc_value, "level").unwrap_or(1);
                            let npc_state = extract_u8(npc_value, "state").unwrap_or(0);
                            let hostile = extract_bool(npc_value, "hostile").unwrap_or(true);
                            let is_quest_giver = extract_bool(npc_value, "is_quest_giver").unwrap_or(false);
                            let is_merchant = extract_bool(npc_value, "is_merchant").unwrap_or(false);
                            let move_speed = extract_f32(npc_value, "move_speed").unwrap_or(2.0);

                            if let Some(npc) = state.npcs.get_mut(&id) {
                                // Update existing NPC - interpolate toward new grid position
                                npc.set_server_position(x, y);
                                npc.direction = Direction::from_u8(direction);
                                // Update last_damage_time if HP decreased (ensures HP bar shows)
                                if hp < npc.hp {
                                    npc.last_damage_time = macroquad::time::get_time();
                                } else if hp > npc.hp && npc.hp > 0 {
                                    // HP increased (regen) - record for floating text
                                    let heal_amount = hp - npc.hp;
                                    npc_regen_events.push((id.clone(), npc.x, npc.y, heal_amount));
                                }
                                npc.hp = hp;
                                npc.max_hp = max_hp;
                                // Handle state transitions
                                let new_state = NpcState::from_u8(npc_state);
                                if new_state != NpcState::Dead {
                                    // NPC is alive - clear death state if it was dying
                                    npc.death_timer = None;
                                    npc.pending_death = false;
                                    npc.state = new_state;
                                } else if npc.death_timer.is_none() && !npc.pending_death {
                                    // Server says dead, start death sequence if not already
                                    npc.start_death();
                                }
                                // If death_timer is Some and new_state is Dead, let animation continue
                                // Update display name in case it changed
                                npc.display_name = display_name;
                                npc.hostile = hostile;
                                npc.is_quest_giver = is_quest_giver;
                                npc.is_merchant = is_merchant;
                                npc.move_speed = move_speed;
                            } else {
                                // New NPC - add to state
                                let mut npc = Npc::new(id.clone(), entity_type, x, y);
                                npc.display_name = display_name;
                                npc.direction = Direction::from_u8(direction);
                                npc.hp = hp;
                                npc.max_hp = max_hp;
                                npc.level = level;
                                npc.state = NpcState::from_u8(npc_state);
                                npc.hostile = hostile;
                                npc.is_quest_giver = is_quest_giver;
                                npc.is_merchant = is_merchant;
                                npc.move_speed = move_speed;
                                state.npcs.insert(id, npc);
                            }
                        }
                    }

                    // Push NPC regen events as healing numbers (negative damage = green +X)
                    for (target_id, x, y, heal_amount) in npc_regen_events {
                        state.damage_events.push(DamageEvent {
                            x,
                            y,
                            damage: -heal_amount, // Negative = healing
                            time: current_time,
                            target_id,
                            source_id: None,
                            projectile: None,
                        });
                    }
                }
            }

            "chatMessage" => {
                if let Some(value) = data {
                    let sender_name = extract_string(value, "senderName").unwrap_or_default();
                    let text = extract_string(value, "text").unwrap_or_default();
                    let timestamp = extract_u64(value, "timestamp").unwrap_or(0) as f64;

                    // Add to chat log
                    state.ui_state.chat_messages.push(ChatMessage {
                        sender_name: sender_name.clone(),
                        text: text.clone(),
                        timestamp,
                        channel: ChatChannel::Local,
                    });

                    if state.ui_state.chat_messages.len() > 100 {
                        state.ui_state.chat_messages.remove(0);
                    }

                    // Create chat bubble above the player who sent the message
                    // Find player by name
                    if let Some((player_id, _)) = state.players.iter().find(|(_, p)| p.name == sender_name) {
                        let player_id = player_id.clone();

                        // Remove any existing bubble for this player (only one bubble per player)
                        state.chat_bubbles.retain(|b| b.player_id != player_id);

                        // Add new bubble
                        state.chat_bubbles.push(ChatBubble {
                            player_id,
                            text,
                            time: macroquad::time::get_time(),
                        });
                    }
                }
            }

            "targetChanged" => {
                if let Some(value) = data {
                    let player_id = extract_string(value, "player_id").unwrap_or_default();
                    let target_id = extract_string(value, "target_id");

                    // Update local selection if this is our player
                    if state.local_player_id.as_ref() == Some(&player_id) {
                        state.selected_entity_id = target_id.clone();
                        log::debug!("Target changed to: {:?}", state.selected_entity_id);
                    }
                }
            }

            "playerAttack" => {
                if let Some(value) = data {
                    let player_id = extract_string(value, "player_id").unwrap_or_default();
                    let attack_type = extract_string(value, "attack_type").unwrap_or_else(|| "melee".to_string());

                    // Trigger attack animation for remote players (local player handles own animation)
                    if state.local_player_id.as_ref() != Some(&player_id) {
                        if let Some(player) = state.players.get_mut(&player_id) {
                            match attack_type.as_str() {
                                "ranged" => player.play_shoot_bow(),
                                "spell" => player.play_cast(),
                                _ => player.play_attack(),
                            }
                        }
                    }
                }
            }

            "damageEvent" => {
                if let Some(value) = data {
                    let source_id = extract_string(value, "source_id");
                    let target_id = extract_string(value, "target_id").unwrap_or_default();
                    let damage = extract_i32(value, "damage").unwrap_or(0);
                    let target_hp = extract_i32(value, "target_hp").unwrap_or(0);
                    let target_x = extract_f32(value, "target_x").unwrap_or(0.0);
                    let target_y = extract_f32(value, "target_y").unwrap_or(0.0);
                    let projectile = extract_string(value, "projectile");

                    log::debug!("Damage event: {} took {} damage from {:?} (HP: {})", target_id, damage, source_id, target_hp);

                    // Trigger attack animation for NPCs (players use playerAttack event)
                    if let Some(ref src_id) = source_id {
                        if let Some(npc) = state.npcs.get_mut(src_id) {
                            npc.trigger_attack_animation();
                        }
                    }

                    // Update target's HP and last damage time (could be player or NPC)
                    let current_time = macroquad::time::get_time();
                    if let Some(player) = state.players.get_mut(&target_id) {
                        player.hp = target_hp;
                        player.last_damage_time = current_time;
                    } else if let Some(npc) = state.npcs.get_mut(&target_id) {
                        npc.hp = target_hp;
                        npc.last_damage_time = current_time;
                    }

                    // Create floating damage number with target_id for height lookup at render time
                    state.damage_events.push(DamageEvent {
                        x: target_x,
                        y: target_y,
                        damage,
                        time: macroquad::time::get_time(),
                        target_id,
                        source_id: source_id.clone(),
                        projectile: projectile.clone(),
                    });

                    // Spawn projectile for ranged attacks
                    if let Some(ref projectile_type) = projectile {
                        if let Some(ref source_id) = source_id {
                            // Get source tile center (rounded to ensure straight isometric lines)
                            let source_pos = if let Some(player) = state.players.get(source_id) {
                                Some((player.x.round(), player.y.round()))
                            } else {
                                None
                            };

                            if let Some((src_x, src_y)) = source_pos {
                                // Target tile center (rounded for straight isometric lines)
                                let end_x = target_x.round();
                                let end_y = target_y.round();

                                state.projectiles.push(crate::game::Projectile {
                                    sprite: projectile_type.clone(),
                                    start_x: src_x,
                                    start_y: src_y,
                                    end_x,
                                    end_y,
                                    start_time: current_time,
                                    duration: 0.15, // Fast arrow travel
                                });
                            }
                        }
                    }
                }
            }

            "npcDied" => {
                if let Some(value) = data {
                    let npc_id = extract_string(value, "npc_id").unwrap_or_default();
                    log::debug!("NPC died: {}", npc_id);

                    if let Some(npc) = state.npcs.get_mut(&npc_id) {
                        npc.start_death();
                    }

                    // Clear selection if we had this NPC targeted
                    if state.selected_entity_id.as_ref() == Some(&npc_id) {
                        state.selected_entity_id = None;
                    }

                    // Close shop if this NPC was the merchant
                    if let Some(shop_npc_id) = &state.ui_state.shop_npc_id {
                        if shop_npc_id == &npc_id {
                            state.ui_state.crafting_open = false;
                            state.ui_state.shop_data = None;
                        }
                    }
                }
            }

            "npcRespawned" => {
                if let Some(value) = data {
                    let npc_id = extract_string(value, "id").unwrap_or_default();
                    // Server sends i32 grid positions
                    let x = extract_i32(value, "x").unwrap_or(0) as f32;
                    let y = extract_i32(value, "y").unwrap_or(0) as f32;
                    let hp = extract_i32(value, "hp").unwrap_or(50);
                    log::debug!("NPC respawned: {} at ({}, {})", npc_id, x, y);

                    if let Some(npc) = state.npcs.get_mut(&npc_id) {
                        npc.state = NpcState::Idle;
                        npc.hp = hp;
                        npc.max_hp = hp;
                        npc.x = x;
                        npc.y = y;
                        npc.target_x = x;
                        npc.target_y = y;
                    }
                }
            }

            "playerDied" => {
                if let Some(value) = data {
                    let player_id = extract_string(value, "id").unwrap_or_default();
                    let killer_id = extract_string(value, "killer_id").unwrap_or_default();
                    log::info!("Player {} was killed by {}", player_id, killer_id);

                    if let Some(player) = state.players.get_mut(&player_id) {
                        player.die();
                    }

                    // Clear selection if we had this player targeted
                    if state.selected_entity_id.as_ref() == Some(&player_id) {
                        state.selected_entity_id = None;
                    }
                }
            }

            "playerRespawned" => {
                if let Some(value) = data {
                    let player_id = extract_string(value, "id").unwrap_or_default();
                    // Server sends i32 grid positions
                    let x = extract_i32(value, "x").unwrap_or(0) as f32;
                    let y = extract_i32(value, "y").unwrap_or(0) as f32;
                    let hp = extract_i32(value, "hp").unwrap_or(100);
                    log::info!("Player {} respawned at ({}, {})", player_id, x, y);

                    if let Some(player) = state.players.get_mut(&player_id) {
                        player.respawn(x, y, hp);
                    }
                }
            }

            "attackResult" => {
                if let Some(value) = data {
                    let success = value.as_map()
                        .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("success")))
                        .and_then(|(_, v)| v.as_bool())
                        .unwrap_or(false);
                    let reason = extract_string(value, "reason");

                    if !success {
                        if let Some(reason) = reason {
                            log::debug!("Attack failed: {}", reason);
                        }
                    }
                }
            }

            "skillXp" => {
                if let Some(value) = data {
                    let player_id = extract_string(value, "player_id").unwrap_or_default();
                    let skill_name = extract_string(value, "skill").unwrap_or_default();
                    let xp_gained = extract_i32(value, "xp_gained").unwrap_or(0) as i64;
                    let total_xp = extract_i32(value, "total_xp").unwrap_or(0) as i64;
                    let level = extract_i32(value, "level").unwrap_or(1);

                    log::debug!("Player {} gained {} {} XP (total: {}, level: {})",
                        player_id, xp_gained, skill_name, total_xp, level);

                    if let Some(player) = state.players.get_mut(&player_id) {
                        // Update the specific skill
                        if let Some(skill_type) = SkillType::from_str(&skill_name) {
                            let skill = player.skills.get_mut(skill_type);
                            skill.xp = total_xp;
                            skill.level = level;

                            // Update max_hp if hitpoints changed
                            if skill_type == SkillType::Hitpoints {
                                player.max_hp = level;
                            }
                        }

                        // Create floating XP event and system message for local player
                        if state.local_player_id.as_ref() == Some(&player_id) {
                            // Add system chat message
                            state.ui_state.chat_messages.push(ChatMessage::system(
                                format!("+{} {} XP", xp_gained, skill_name)
                            ));

                            state.skill_xp_events.push(SkillXpEvent {
                                x: player.x,
                                y: player.y,
                                skill: skill_name.clone(),
                                xp_gained,
                                time: macroquad::time::get_time(),
                            });

                            // Update XP globes
                            if let Some(skill_type) = SkillType::from_str(&skill_name) {
                                let xp_for_next = crate::game::skills::total_xp_for_level(level + 1);
                                state.xp_globes.on_xp_gain(skill_type, total_xp, xp_for_next, level);
                            }
                        }
                    }
                }
            }

            "skillLevelUp" => {
                if let Some(value) = data {
                    let player_id = extract_string(value, "player_id").unwrap_or_default();
                    let skill_name = extract_string(value, "skill").unwrap_or_default();
                    let new_level = extract_i32(value, "new_level").unwrap_or(1);

                    log::info!("Player {} leveled up {} to {}!", player_id, skill_name, new_level);

                    // Get player position for floating text
                    if let Some(player) = state.players.get_mut(&player_id) {
                        // Update the specific skill level
                        if let Some(skill_type) = SkillType::from_str(&skill_name) {
                            let skill = player.skills.get_mut(skill_type);
                            skill.level = new_level;

                            // Update max_hp and current HP if hitpoints leveled up
                            if skill_type == SkillType::Hitpoints {
                                let old_max = player.max_hp;
                                player.max_hp = new_level;
                                // Heal the difference (new HP from the level)
                                player.hp += new_level - old_max;
                            }
                        }

                        // Create floating level up event and system message for local player
                        if state.local_player_id.as_ref() == Some(&player_id) {
                            state.ui_state.chat_messages.push(ChatMessage::system(
                                format!("{} leveled up to {}!", skill_name, new_level)
                            ));
                        }

                        state.level_up_events.push(LevelUpEvent {
                            x: player.x,
                            y: player.y,
                            skill: skill_name,
                            new_level,
                            time: macroquad::time::get_time(),
                        });
                    }
                }
            }

            "itemDropped" => {
                if let Some(value) = data {
                    let id = extract_string(value, "id").unwrap_or_default();
                    let item_id = extract_string(value, "item_id").unwrap_or_else(|| "unknown".to_string());
                    let x = extract_f32(value, "x").unwrap_or(0.0);
                    let y = extract_f32(value, "y").unwrap_or(0.0);
                    let quantity = extract_i32(value, "quantity").unwrap_or(1);

                    log::debug!("Item dropped: {} ({}) at ({}, {})", id, item_id, x, y);

                    let item = if item_id == "gold" {
                        GroundItem::new_gold(id.clone(), x, y, quantity)
                    } else {
                        GroundItem::new(id.clone(), item_id, x, y, quantity)
                    };

                    // Check if there's a dying NPC near this drop location
                    let near_dying_npc = state.npcs.values().any(|npc| {
                        let dx = npc.x - x;
                        let dy = npc.y - y;
                        let dist_sq = dx * dx + dy * dy;
                        npc.is_dying() && dist_sq < 2.0 // Within ~1.4 tiles
                    });

                    if near_dying_npc {
                        // Delay item appearance by 0.6s to let death animation complete
                        let spawn_time = macroquad::time::get_time() + 0.6;
                        state.pending_ground_items.push((item, spawn_time));
                    } else {
                        // Spawn immediately (player drop, etc.)
                        state.ground_items.insert(id, item);
                    }
                }
            }

            "itemPickedUp" => {
                if let Some(value) = data {
                    let item_id = extract_string(value, "item_id").unwrap_or_default();
                    let player_id = extract_string(value, "player_id").unwrap_or_default();

                    log::debug!("Item {} picked up by {}", item_id, player_id);
                    state.ground_items.remove(&item_id);
                }
            }

            "itemDespawned" => {
                if let Some(value) = data {
                    let item_id = extract_string(value, "item_id").unwrap_or_default();
                    log::debug!("Item {} despawned", item_id);
                    state.ground_items.remove(&item_id);
                }
            }

            "itemQuantityUpdated" => {
                if let Some(value) = data {
                    let id = extract_string(value, "id").unwrap_or_default();
                    let quantity = extract_i32(value, "quantity").unwrap_or(1);

                    log::debug!("Item {} quantity updated to {}", id, quantity);

                    if let Some(item) = state.ground_items.get_mut(&id) {
                        item.quantity = quantity;
                        // Regenerate gold pile with new quantity
                        if item.item_id == "gold" {
                            item.gold_pile = Some(crate::game::item::GoldPileState::new(
                                quantity,
                                macroquad::time::get_time(),
                            ));
                        }
                    }
                }
            }

            "inventoryUpdate" => {
                // Server sends this only to the owning player (unicast)
                if let Some(value) = data {
                    // Clear current inventory
                    for slot in state.inventory.slots.iter_mut() {
                        *slot = None;
                    }

                    // Update slots
                    if let Some(slots) = extract_array(value, "slots") {
                        for slot_value in slots {
                            let slot_idx = extract_u8(slot_value, "slot").unwrap_or(0) as usize;
                            let item_id = extract_string(slot_value, "item_id").unwrap_or_default();
                            let quantity = extract_i32(slot_value, "quantity").unwrap_or(0);

                            if slot_idx < state.inventory.slots.len() && !item_id.is_empty() && quantity > 0 {
                                state.inventory.slots[slot_idx] = Some(InventorySlot::new(item_id, quantity));
                            }
                        }
                    }

                    // Update gold
                    if let Some(gold) = extract_i32(value, "gold") {
                        state.inventory.gold = gold;
                    }

                    log::debug!("Inventory updated: {} gold, {} items",
                        state.inventory.gold,
                        state.inventory.slots.iter().filter(|s| s.is_some()).count()
                    );
                }
            }

            "itemUsed" => {
                // Server sends this only to the owning player (unicast)
                if let Some(value) = data {
                    let slot = extract_u8(value, "slot").unwrap_or(0);
                    let item_id = extract_string(value, "item_id").unwrap_or_default();
                    let effect = extract_string(value, "effect").unwrap_or_default();
                    log::debug!("Item used: slot {} item {} effect {}", slot, item_id, effect);
                }
            }

            "chunkData" => {
                if let Some(value) = data {
                    let chunk_x = extract_i32(value, "chunkX").unwrap_or(0);
                    let chunk_y = extract_i32(value, "chunkY").unwrap_or(0);

                    // Parse layers array
                    let mut layers: Vec<(u8, Vec<u32>)> = Vec::new();
                    if let Some(layers_arr) = extract_array(value, "layers") {
                        for layer_value in layers_arr {
                            let layer_type = extract_u8(layer_value, "layerType").unwrap_or(0);
                            let tiles: Vec<u32> = extract_array(layer_value, "tiles")
                                .map(|arr| arr.iter()
                                    .filter_map(|v| v.as_u64().map(|u| u as u32))
                                    .collect())
                                .unwrap_or_default();
                            layers.push((layer_type, tiles));
                        }
                    }

                    // Parse collision bytes
                    let collision: Vec<u8> = extract_array(value, "collision")
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_u64().map(|u| u as u8))
                            .collect())
                        .unwrap_or_default();

                    // Parse map objects
                    let mut objects: Vec<MapObject> = Vec::new();
                    if let Some(objects_arr) = extract_array(value, "objects") {
                        for obj_value in objects_arr {
                            let gid = obj_value["gid"].as_u64().unwrap_or(0) as u32;
                            let tile_x = obj_value["tileX"].as_i64().unwrap_or(0) as i32;
                            let tile_y = obj_value["tileY"].as_i64().unwrap_or(0) as i32;
                            let width = obj_value["width"].as_u64().unwrap_or(0) as u32;
                            let height = obj_value["height"].as_u64().unwrap_or(0) as u32;
                            log::info!("CLIENT received object gid {} at WORLD tile ({}, {})", gid, tile_x, tile_y);
                            objects.push(MapObject {
                                gid,
                                tile_x,
                                tile_y,
                                width,
                                height,
                            });
                        }
                    }

                    // Parse walls from server message
                    let mut walls: Vec<Wall> = Vec::new();
                    if let Some(walls_arr) = extract_array(value, "walls") {
                        for w in walls_arr {
                            let gid = w["gid"].as_u64().unwrap_or(0) as u32;
                            let tile_x = w["tileX"].as_i64().unwrap_or(0) as i32;
                            let tile_y = w["tileY"].as_i64().unwrap_or(0) as i32;
                            let edge_str = w["edge"].as_str().unwrap_or("down");
                            let edge = match edge_str {
                                "right" => WallEdge::Right,
                                _ => WallEdge::Down,
                            };
                            walls.push(Wall { gid, tile_x, tile_y, edge });
                        }
                    }

                    // Parse portals from server message
                    let mut portals: Vec<Portal> = Vec::new();
                    if let Some(portals_arr) = extract_array(value, "portals") {
                        for p in portals_arr {
                            let id = extract_string(p, "id").unwrap_or_default();
                            let x = extract_i32(p, "x").unwrap_or(0);
                            let y = extract_i32(p, "y").unwrap_or(0);
                            let width = extract_i32(p, "width").unwrap_or(1);
                            let height = extract_i32(p, "height").unwrap_or(1);
                            let target_map = extract_string(p, "targetMap").unwrap_or_default();
                            let target_spawn = extract_string(p, "targetSpawn").unwrap_or_default();
                            portals.push(Portal {
                                id,
                                x,
                                y,
                                width,
                                height,
                                target_map,
                                target_spawn,
                            });
                        }
                    }

                    log::debug!("Received chunk data: ({}, {}) with {} layers, {} collision bytes, {} objects, {} walls, {} portals",
                        chunk_x, chunk_y, layers.len(), collision.len(), objects.len(), walls.len(), portals.len());

                    state.chunk_manager.load_chunk(chunk_x, chunk_y, layers, &collision, objects, walls, portals);
                }
            }

            "chunkNotFound" => {
                if let Some(value) = data {
                    let chunk_x = extract_i32(value, "chunkX").unwrap_or(0);
                    let chunk_y = extract_i32(value, "chunkY").unwrap_or(0);
                    log::warn!("Chunk not found: ({}, {})", chunk_x, chunk_y);
                }
            }

            // ========== Quest System Messages ==========

            "showDialogue" => {
                if let Some(value) = data {
                    let quest_id = extract_string(value, "quest_id").unwrap_or_default();
                    let npc_id = extract_string(value, "npc_id").unwrap_or_default();
                    let speaker = extract_string(value, "speaker").unwrap_or_default();
                    let text = extract_string(value, "text").unwrap_or_default();

                    // Parse choices array
                    let mut choices = Vec::new();
                    if let Some(choices_arr) = extract_array(value, "choices") {
                        for choice_value in choices_arr {
                            let id = extract_string(choice_value, "id").unwrap_or_default();
                            let choice_text = extract_string(choice_value, "text").unwrap_or_default();
                            choices.push(DialogueChoice { id, text: choice_text });
                        }
                    }

                    log::info!("Showing dialogue from {}: {} ({} choices)", speaker, text, choices.len());

                    state.ui_state.active_dialogue = Some(ActiveDialogue {
                        quest_id,
                        npc_id,
                        speaker,
                        text,
                        choices,
                        show_time: macroquad::time::get_time(),
                    });
                }
            }

            "questAccepted" => {
                if let Some(value) = data {
                    let quest_id = extract_string(value, "quest_id").unwrap_or_default();
                    let quest_name = extract_string(value, "quest_name").unwrap_or_default();

                    // Parse objectives
                    let mut objectives = Vec::new();
                    if let Some(obj_arr) = extract_array(value, "objectives") {
                        for obj_value in obj_arr {
                            let id = extract_string(obj_value, "id").unwrap_or_default();
                            let description = extract_string(obj_value, "description").unwrap_or_default();
                            let current = extract_i32(obj_value, "current").unwrap_or(0);
                            let target = extract_i32(obj_value, "target").unwrap_or(1);
                            objectives.push(QuestObjective {
                                id,
                                description,
                                current,
                                target,
                                completed: current >= target,
                            });
                        }
                    }

                    log::info!("Quest accepted: {} - {}", quest_id, quest_name);

                    // Add to active quests (or update if exists)
                    if let Some(existing) = state.ui_state.active_quests.iter_mut().find(|q| q.id == quest_id) {
                        existing.objectives = objectives;
                    } else {
                        state.ui_state.active_quests.push(ActiveQuest {
                            id: quest_id,
                            name: quest_name,
                            objectives,
                        });
                    }

                    // Don't close dialogue here - let user read the quest acceptance message
                    // Dialogue will close when user presses continue or server sends dialogueClosed
                }
            }

            "questObjectiveProgress" => {
                if let Some(value) = data {
                    let quest_id = extract_string(value, "quest_id").unwrap_or_default();
                    let objective_id = extract_string(value, "objective_id").unwrap_or_default();
                    let current = extract_i32(value, "current").unwrap_or(0);
                    let target = extract_i32(value, "target").unwrap_or(1);

                    log::debug!("Quest objective progress: {}:{} = {}/{}", quest_id, objective_id, current, target);

                    // Update the objective in the active quest
                    if let Some(quest) = state.ui_state.active_quests.iter_mut().find(|q| q.id == quest_id) {
                        if let Some(obj) = quest.objectives.iter_mut().find(|o| o.id == objective_id) {
                            obj.current = current;
                            obj.target = target;
                            obj.completed = current >= target;
                        }
                    }
                }
            }

            "questCompleted" => {
                if let Some(value) = data {
                    let quest_id = extract_string(value, "quest_id").unwrap_or_default();
                    let quest_name = extract_string(value, "quest_name").unwrap_or_default();
                    let exp_reward = extract_i32(value, "rewards_exp").unwrap_or(0);
                    let gold_reward = extract_i32(value, "rewards_gold").unwrap_or(0);

                    log::info!("Quest completed: {} - {} (EXP: {}, Gold: {})", quest_id, quest_name, exp_reward, gold_reward);

                    // Add system chat message
                    state.ui_state.chat_messages.push(ChatMessage::system(
                        format!("Quest '{}' complete!", quest_name)
                    ));

                    // Remove from active quests
                    state.ui_state.active_quests.retain(|q| q.id != quest_id);

                    // Add completion notification
                    state.ui_state.quest_completed_events.push(QuestCompletedEvent {
                        quest_id,
                        quest_name,
                        exp_reward,
                        gold_reward,
                        time: macroquad::time::get_time(),
                    });

                    // Close any open dialogue
                    state.ui_state.active_dialogue = None;
                }
            }

            "dialogueClosed" => {
                // Server tells us to close dialogue
                state.ui_state.active_dialogue = None;
            }

            // ========== Item Definition Messages ==========

            "itemDefinitions" => {
                if let Some(value) = data {
                    let mut items = Vec::new();

                    if let Some(items_arr) = extract_array(value, "items") {
                        for item_value in items_arr {
                            let id = extract_string(item_value, "id").unwrap_or_default();
                            let display_name = extract_string(item_value, "displayName").unwrap_or_default();
                            let sprite = extract_string(item_value, "sprite").unwrap_or_default();
                            let category = extract_string(item_value, "category").unwrap_or_else(|| "material".to_string());
                            let max_stack = extract_i32(item_value, "maxStack").unwrap_or(99);
                            let description = extract_string(item_value, "description").unwrap_or_default();
                            let base_price = extract_i32(item_value, "basePrice").unwrap_or(0);
                            let sellable = extract_bool(item_value, "sellable").unwrap_or(false);

                            // Parse equipment stats if present
                            let equipment = extract_string(item_value, "equipment_slot")
                                .map(|slot_type| {
                                    EquipmentStats {
                                        slot_type,
                                        attack_level_required: extract_i32(item_value, "attack_level_required").unwrap_or(1),
                                        defence_level_required: extract_i32(item_value, "defence_level_required").unwrap_or(1),
                                        attack_bonus: extract_i32(item_value, "attack_bonus").unwrap_or(0),
                                        strength_bonus: extract_i32(item_value, "strength_bonus").unwrap_or(0),
                                        defence_bonus: extract_i32(item_value, "defence_bonus").unwrap_or(0),
                                    }
                                });

                            // Parse weapon fields
                            let weapon_type = extract_string(item_value, "weapon_type");
                            let range = extract_i32(item_value, "range");

                            items.push(ItemDefinition {
                                id,
                                display_name,
                                sprite,
                                category,
                                max_stack,
                                description,
                                base_price,
                                sellable,
                                equipment,
                                weapon_type,
                                range,
                            });
                        }
                    }

                    state.item_registry.load_from_server(items);
                }
            }

            // ========== Crafting System Messages ==========

            "recipeDefinitions" => {
                if let Some(value) = data {
                    state.recipe_definitions.clear();

                    if let Some(recipes_arr) = extract_array(value, "recipes") {
                        for recipe_value in recipes_arr {
                            let id = extract_string(recipe_value, "id").unwrap_or_default();
                            let display_name = extract_string(recipe_value, "display_name").unwrap_or_default();
                            let description = extract_string(recipe_value, "description").unwrap_or_default();
                            let category = extract_string(recipe_value, "category").unwrap_or_else(|| "consumables".to_string());
                            let level_required = extract_i32(recipe_value, "level_required").unwrap_or(1);

                            // Parse ingredients
                            let mut ingredients = Vec::new();
                            if let Some(ing_arr) = extract_array(recipe_value, "ingredients") {
                                for ing_value in ing_arr {
                                    let item_id = extract_string(ing_value, "item_id").unwrap_or_default();
                                    let item_name = extract_string(ing_value, "item_name").unwrap_or_default();
                                    let count = extract_i32(ing_value, "count").unwrap_or(1);
                                    ingredients.push(RecipeIngredient { item_id, item_name, count });
                                }
                            }

                            // Parse results
                            let mut results = Vec::new();
                            if let Some(res_arr) = extract_array(recipe_value, "results") {
                                for res_value in res_arr {
                                    let item_id = extract_string(res_value, "item_id").unwrap_or_default();
                                    let item_name = extract_string(res_value, "item_name").unwrap_or_default();
                                    let count = extract_i32(res_value, "count").unwrap_or(1);
                                    results.push(RecipeResult { item_id, item_name, count });
                                }
                            }

                            state.recipe_definitions.push(RecipeDefinition {
                                id,
                                display_name,
                                description,
                                category,
                                level_required,
                                ingredients,
                                results,
                            });
                        }
                    }

                    log::info!("Received {} recipe definitions", state.recipe_definitions.len());
                }
            }

            "shopOpen" => {
                if let Some(value) = data {
                    let npc_id = extract_string(value, "npc_id").unwrap_or_default();
                    log::info!("Opening shop for NPC: {}", npc_id);

                    state.ui_state.crafting_open = true;
                    state.ui_state.crafting_npc_id = Some(npc_id);
                    state.ui_state.crafting_selected_category = 0;
                    state.ui_state.crafting_selected_recipe = 0;
                }
            }

            "craftResult" => {
                if let Some(value) = data {
                    let success = value.as_map()
                        .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("success")))
                        .and_then(|(_, v)| v.as_bool())
                        .unwrap_or(false);
                    let recipe_id = extract_string(value, "recipe_id").unwrap_or_default();
                    let error = extract_string(value, "error");

                    if success {
                        log::info!("Crafting success: {}", recipe_id);
                        // Inventory update will come separately
                    } else {
                        log::warn!("Crafting failed: {} - {:?}", recipe_id, error);
                        // TODO: Show error message in UI
                    }
                }
            }

            // ========== Equipment Messages ==========

            "equipmentUpdate" => {
                if let Some(value) = data {
                    let player_id = extract_string(value, "player_id").unwrap_or_default();
                    let equipped_head = extract_string(value, "equipped_head").filter(|s| !s.is_empty());
                    let equipped_body = extract_string(value, "equipped_body").filter(|s| !s.is_empty());
                    let equipped_weapon = extract_string(value, "equipped_weapon").filter(|s| !s.is_empty());
                    let equipped_back = extract_string(value, "equipped_back").filter(|s| !s.is_empty());
                    let equipped_feet = extract_string(value, "equipped_feet").filter(|s| !s.is_empty());
                    let equipped_ring = extract_string(value, "equipped_ring").filter(|s| !s.is_empty());
                    let equipped_gloves = extract_string(value, "equipped_gloves").filter(|s| !s.is_empty());
                    let equipped_necklace = extract_string(value, "equipped_necklace").filter(|s| !s.is_empty());
                    let equipped_belt = extract_string(value, "equipped_belt").filter(|s| !s.is_empty());

                    if let Some(player) = state.players.get_mut(&player_id) {
                        player.equipped_head = equipped_head.clone();
                        player.equipped_body = equipped_body.clone();
                        player.equipped_weapon = equipped_weapon.clone();
                        player.equipped_back = equipped_back.clone();
                        player.equipped_feet = equipped_feet.clone();
                        player.equipped_ring = equipped_ring.clone();
                        player.equipped_gloves = equipped_gloves.clone();
                        player.equipped_necklace = equipped_necklace.clone();
                        player.equipped_belt = equipped_belt.clone();
                        log::info!("Player {} equipment updated", player_id);
                    }
                }
            }

            "equipResult" => {
                if let Some(value) = data {
                    let success = value.as_map()
                        .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("success")))
                        .and_then(|(_, v)| v.as_bool())
                        .unwrap_or(false);
                    let slot_type = extract_string(value, "slot_type").unwrap_or_default();
                    let item_id = extract_string(value, "item_id");
                    let error = extract_string(value, "error");

                    if success {
                        log::info!("Equipment {} success: {:?}", slot_type, item_id);
                    } else {
                        log::warn!("Equipment {} failed: {:?}", slot_type, error);
                        // TODO: Show error message in UI
                    }
                }
            }

            // ========== Admin Messages ==========

            "announcement" => {
                if let Some(value) = data {
                    let text = extract_string(value, "text").unwrap_or_default();
                    log::info!("Server announcement: {}", text);
                    state.ui_state.announcements.push(crate::game::Announcement {
                        text,
                        time: macroquad::time::get_time(),
                    });
                }
            }

            // ========== Shop System Messages ==========

            "shopData" => {
                if let Some(value) = data {
                    // Extract npcId from top level (camelCase from server)
                    let npc_id = extract_string(value, "npcId").unwrap_or_default();

                    // Extract shop data from nested "shop" field
                    let shop_value = value.as_map()
                        .and_then(|m| {
                            m.iter()
                                .find(|(k, _)| k.as_str() == Some("shop"))
                                .map(|(_, v)| v)
                        })
                        .unwrap_or(value);

                    let shop_id = extract_string(shop_value, "shopId").unwrap_or_default();
                    let display_name = extract_string(shop_value, "displayName").unwrap_or_else(|| "Shop".to_string());
                    let buy_multiplier = extract_f32(shop_value, "buyMultiplier").unwrap_or(0.5);
                    let sell_multiplier = extract_f32(shop_value, "sellMultiplier").unwrap_or(1.0);

                    let mut stock = Vec::new();
                    if let Some(stock_arr) = extract_array(shop_value, "stock") {
                        for item_value in stock_arr {
                            let item_id = extract_string(item_value, "itemId").unwrap_or_default();
                            let quantity = extract_i32(item_value, "quantity").unwrap_or(0);
                            let price = extract_i32(item_value, "price").unwrap_or(0);

                            stock.push(ShopStockItem {
                                item_id,
                                quantity,
                                price,
                            });
                        }
                    }

                    log::info!("Shop data received: {} items from {} (npc: {})", stock.len(), display_name, npc_id);
                    state.ui_state.shop_npc_id = Some(npc_id);
                    state.ui_state.shop_data = Some(ShopData {
                        shop_id,
                        display_name,
                        buy_multiplier,
                        sell_multiplier,
                        stock,
                    });
                    state.ui_state.crafting_open = true; // Open crafting window (which has shop tab)
                    state.ui_state.shop_main_tab = 1; // Switch to Shop tab
                }
            }

            "shopResult" => {
                if let Some(value) = data {
                    let success = value.as_map()
                        .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("success")))
                        .and_then(|(_, v)| v.as_bool())
                        .unwrap_or(false);
                    let action = extract_string(value, "action").unwrap_or_default();
                    let item_id = extract_string(value, "itemId").unwrap_or_default();
                    let quantity = extract_i32(value, "quantity").unwrap_or(0);
                    let gold_change = extract_i32(value, "goldChange").unwrap_or(0);
                    let error = extract_string(value, "error");

                    if success {
                        log::info!("Shop transaction successful");

                        // Get item display name from registry
                        let item_name = state.item_registry.get(&item_id)
                            .map(|def| def.display_name.clone())
                            .unwrap_or_else(|| item_id.clone());

                        // Add system chat message
                        let message = if action == "buy" {
                            format!("Bought {}x {} for {}g", quantity, item_name, gold_change.abs())
                        } else {
                            format!("Sold {}x {} for {}g", quantity, item_name, gold_change.abs())
                        };
                        state.ui_state.chat_messages.push(ChatMessage::system(message));
                    } else if let Some(err) = error {
                        log::warn!("Shop transaction failed: {}", err);
                        // Show error in system chat
                        state.ui_state.chat_messages.push(ChatMessage::system(
                            format!("Transaction failed: {}", err)
                        ));
                    }
                }
            }

            "shopStockUpdate" => {
                if let Some(value) = data {
                    let item_id = extract_string(value, "itemId").unwrap_or_default();
                    let new_quantity = extract_i32(value, "newQuantity").unwrap_or(0);

                    // Update the stock in the current shop data if it's open
                    if let Some(shop_data) = &mut state.ui_state.shop_data {
                        if let Some(item) = shop_data.stock.iter_mut().find(|i| i.item_id == item_id) {
                            item.quantity = new_quantity;
                            log::debug!("Shop stock updated: {} now has {} in stock", item_id, new_quantity);
                        }
                    }
                }
            }

            "mapTransition" => {
                if let Some(value) = data {
                    let map_type = extract_string(value, "mapType").unwrap_or_default();
                    let map_id = extract_string(value, "mapId").unwrap_or_default();
                    let spawn_x = extract_f32(value, "spawnX").unwrap_or(0.0);
                    let spawn_y = extract_f32(value, "spawnY").unwrap_or(0.0);
                    let instance_id = extract_string(value, "instanceId").unwrap_or_default();

                    if map_type == "overworld" {
                        // Returning to overworld from interior

                        // Trigger area banner for overworld
                        state.area_banner.show(OVERWORLD_NAME);

                        // Clear interior mode
                        state.chunk_manager.clear_interior();
                        state.current_interior = None;
                        state.current_instance = None;

                        // Clear interior NPCs and ground items (will be repopulated by stateSync)
                        state.npcs.clear();
                        state.ground_items.clear();

                        // Reset portal check position to prevent immediate re-trigger
                        state.last_portal_check_pos = None;

                        // Update player position
                        if let Some(local_id) = &state.local_player_id {
                            if let Some(player) = state.players.get_mut(local_id) {
                                player.x = spawn_x;
                                player.y = spawn_y;
                            }
                        }

                        // Start fade-in transition directly (no loading needed)
                        state.map_transition.state = crate::game::state::TransitionState::FadingIn;
                        state.map_transition.progress = 1.0;
                    } else {
                        // Transitioning to interior - wait for interiorData
                        state.start_transition(map_type, map_id, spawn_x, spawn_y, instance_id);
                    }
                }
            }

            "interiorData" => {
                if let Some(value) = data {
                    let map_id = extract_string(value, "mapId").unwrap_or_default();
                    let instance_id = extract_string(value, "instanceId").unwrap_or_default();
                    let width = extract_u32(value, "width").unwrap_or(32);
                    let height = extract_u32(value, "height").unwrap_or(32);
                    let spawn_x = extract_f32(value, "spawnX").unwrap_or(0.0);
                    let spawn_y = extract_f32(value, "spawnY").unwrap_or(0.0);

                    // Extract interior name (fallback to map_id if missing)
                    let name = extract_string(value, "name").unwrap_or(map_id.clone());

                    // Trigger area banner
                    state.area_banner.show(&name);

                    // Parse layers
                    let mut layers: Vec<(u8, Vec<u32>)> = Vec::new();
                    if let Some(layers_arr) = extract_array(value, "layers") {
                        for layer_data in layers_arr {
                            let layer_type = extract_u8(layer_data, "layerType").unwrap_or(0);
                            let tiles: Vec<u32> = extract_array(layer_data, "tiles")
                                .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect())
                                .unwrap_or_default();
                            layers.push((layer_type, tiles));
                        }
                    }

                    // Parse collision
                    let collision: Vec<u8> = extract_array(value, "collision")
                        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
                        .unwrap_or_default();

                    // Parse portals
                    let mut portals: Vec<Portal> = Vec::new();
                    if let Some(portals_arr) = extract_array(value, "portals") {
                        for p in portals_arr {
                            portals.push(Portal {
                                id: extract_string(p, "id").unwrap_or_default(),
                                x: extract_i32(p, "x").unwrap_or(0),
                                y: extract_i32(p, "y").unwrap_or(0),
                                width: extract_i32(p, "width").unwrap_or(1),
                                height: extract_i32(p, "height").unwrap_or(1),
                                target_map: extract_string(p, "targetMap").unwrap_or_default(),
                                target_spawn: extract_string(p, "targetSpawn").unwrap_or_default(),
                            });
                        }
                    }

                    // Parse objects (trees, rocks, decorations)
                    let mut objects: Vec<MapObject> = Vec::new();
                    if let Some(objects_arr) = extract_array(value, "objects") {
                        for o in objects_arr {
                            objects.push(MapObject {
                                gid: extract_u32(o, "gid").unwrap_or(0),
                                tile_x: extract_i32(o, "tileX").unwrap_or(0),
                                tile_y: extract_i32(o, "tileY").unwrap_or(0),
                                width: extract_u32(o, "width").unwrap_or(32),
                                height: extract_u32(o, "height").unwrap_or(32),
                            });
                        }
                    }

                    // Parse walls
                    let mut walls: Vec<Wall> = Vec::new();
                    if let Some(walls_arr) = extract_array(value, "walls") {
                        for w in walls_arr {
                            let edge_str = extract_string(w, "edge").unwrap_or_default();
                            let edge = match edge_str.as_str() {
                                "right" | "Right" => WallEdge::Right,
                                _ => WallEdge::Down,
                            };
                            walls.push(Wall {
                                gid: extract_u32(w, "gid").unwrap_or(0),
                                tile_x: extract_i32(w, "tileX").unwrap_or(0),
                                tile_y: extract_i32(w, "tileY").unwrap_or(0),
                                edge,
                            });
                        }
                    }

                    // Clear world data when entering interior
                    state.npcs.clear();
                    state.ground_items.clear();

                    // Clear other players (keep only local player) to avoid ghost collisions
                    if let Some(local_id) = &state.local_player_id {
                        let local_player = state.players.remove(local_id);
                        state.players.clear();
                        if let Some(player) = local_player {
                            state.players.insert(local_id.clone(), player);
                        }
                    } else {
                        state.players.clear();
                    }

                    // Load the interior
                    state.chunk_manager.load_interior(width, height, layers, &collision, portals, objects, walls);
                    state.current_interior = Some(map_id.clone());
                    state.current_instance = Some(instance_id);

                    // Reset portal check position to prevent immediate re-trigger
                    state.last_portal_check_pos = None;

                    // Update player position to spawn point
                    if let Some(local_id) = &state.local_player_id {
                        if let Some(player) = state.players.get_mut(local_id) {
                            player.x = spawn_x;
                            player.y = spawn_y;
                        }
                    }

                    // Complete the transition (fade in)
                    // Handle both Loading (normal case) and FadingOut (data arrived quickly)
                    match state.map_transition.state {
                        crate::game::state::TransitionState::Loading => {
                            state.map_transition.state = crate::game::state::TransitionState::FadingIn;
                        }
                        crate::game::state::TransitionState::FadingOut => {
                            // Data arrived before fade out completed - skip to fade in
                            state.map_transition.progress = 1.0;
                            state.map_transition.state = crate::game::state::TransitionState::FadingIn;
                        }
                        _ => {}
                    }
                }
            }

            _ => {
                log::debug!("Unhandled message type: {}", msg_type);
            }
        }
    }

    pub fn send(&mut self, msg: &ClientMessage) {
        if let Some(sender) = &mut self.sender {
            if self.connection_state == ConnectionState::Connected {
                let (msg_type, msg_data) = msg.to_protocol();
                match protocol::encode_message(msg_type, &msg_data) {
                    Ok(bytes) => {
                        sender.send(WsMessage::Binary(bytes));
                    }
                    Err(e) => {
                        log::error!("Failed to encode message: {}", e);
                    }
                }
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }

    pub fn disconnect(&mut self) {
        self.sender = None;
        self.receiver = None;
        self.connection_state = ConnectionState::Disconnected;
        self.room_id = None;
        self.session_token = None;
        log::info!("Disconnected from server");
    }
}

mod rand {
    pub fn random<T: RandomGen>() -> T {
        T::random()
    }

    pub trait RandomGen {
        fn random() -> Self;
    }

    impl RandomGen for u16 {
        fn random() -> Self {
            use std::time::{SystemTime, UNIX_EPOCH};
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            ((time.as_nanos() % 65536) as u16).wrapping_mul(31337)
        }
    }
}
