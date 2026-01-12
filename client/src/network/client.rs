use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use serde::{Deserialize, Serialize};
use crate::game::{GameState, ConnectionStatus, Player, Direction, ChatMessage, ChatBubble, DamageEvent, LevelUpEvent, GroundItem, InventorySlot, ActiveDialogue, DialogueChoice, ActiveQuest, QuestObjective, QuestCompletedEvent, RecipeDefinition, RecipeIngredient, RecipeResult, ItemDefinition, EquipmentStats, MapObject};
use crate::game::npc::{Npc, NpcType, NpcState};
use super::messages::ClientMessage;
use super::protocol::{self, DecodedMessage, extract_string, extract_f32, extract_i32, extract_u64, extract_array, extract_u8, extract_bool};

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
            // Authenticated matchmaking
            log::info!("Matchmaking (authenticated): POST {}", matchmake_url);
            let options = JoinOptions {
                name: self.player_name.clone(),
            };
            request
                .set("Authorization", &format!("Bearer {}", token))
                .send_json(&options)
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

        let mut events = Vec::new();
        if let Some(receiver) = &self.receiver {
            while let Some(event) = receiver.try_recv() {
                events.push(event);
            }
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
        log::debug!("Received {} bytes: {:?}", data.len(), &data[..data.len().min(50)]);
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
                    if let Some(players) = extract_array(value, "players") {
                        for player_value in players {
                            let id = extract_string(player_value, "id").unwrap_or_default();
                            // Server sends i32 grid positions
                            let x = extract_i32(player_value, "x");
                            let y = extract_i32(player_value, "y");
                            let direction = extract_i32(player_value, "direction");
                            let hp = extract_i32(player_value, "hp");
                            let max_hp = extract_i32(player_value, "maxHp");
                            let level = extract_i32(player_value, "level");
                            let exp = extract_i32(player_value, "exp");
                            let exp_to_next_level = extract_i32(player_value, "expToNextLevel");
                            let gold = extract_i32(player_value, "gold");
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
                                        player.animation.direction = new_dir;
                                    }
                                }
                                if let Some(hp) = hp {
                                    player.hp = hp;
                                }
                                if let Some(max_hp) = max_hp {
                                    player.max_hp = max_hp;
                                }
                                if let Some(level) = level {
                                    player.level = level;
                                }
                                if let Some(exp) = exp {
                                    player.exp = exp;
                                }
                                if let Some(exp_to_next_level) = exp_to_next_level {
                                    player.exp_to_next_level = exp_to_next_level;
                                }
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
                            }

                            // Update inventory gold for local player
                            if state.local_player_id.as_ref() == Some(&id) {
                                if let Some(gold) = gold {
                                    state.inventory.gold = gold;
                                }
                            }
                        }
                    }

                    // Update NPCs (grid positions from server, converted to f32 for interpolation)
                    if let Some(npcs) = extract_array(value, "npcs") {
                        for npc_value in npcs {
                            let id = extract_string(npc_value, "id").unwrap_or_default();
                            let npc_type = extract_u8(npc_value, "npc_type").unwrap_or(0);
                            let entity_type = extract_string(npc_value, "entity_type").unwrap_or_else(|| "slime".to_string());
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

                            if let Some(npc) = state.npcs.get_mut(&id) {
                                // Update existing NPC - interpolate toward new grid position
                                npc.set_server_position(x, y);
                                npc.direction = Direction::from_u8(direction);
                                npc.hp = hp;
                                npc.max_hp = max_hp;
                                npc.state = NpcState::from_u8(npc_state);
                                // Update display name in case it changed
                                npc.display_name = display_name;
                                npc.hostile = hostile;
                            } else {
                                // New NPC - add to state
                                let mut npc = Npc::new(id.clone(), NpcType::from_u8(npc_type), x, y);
                                npc.entity_type = entity_type;
                                npc.display_name = display_name;
                                npc.direction = Direction::from_u8(direction);
                                npc.hp = hp;
                                npc.max_hp = max_hp;
                                npc.level = level;
                                npc.state = NpcState::from_u8(npc_state);
                                npc.hostile = hostile;
                                state.npcs.insert(id, npc);
                            }
                        }
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

            "damageEvent" => {
                if let Some(value) = data {
                    let target_id = extract_string(value, "target_id").unwrap_or_default();
                    let damage = extract_i32(value, "damage").unwrap_or(0);
                    let target_hp = extract_i32(value, "target_hp").unwrap_or(0);
                    let target_x = extract_f32(value, "target_x").unwrap_or(0.0);
                    let target_y = extract_f32(value, "target_y").unwrap_or(0.0);

                    log::debug!("Damage event: {} took {} damage (HP: {})", target_id, damage, target_hp);

                    // Update target's HP (could be player or NPC)
                    if let Some(player) = state.players.get_mut(&target_id) {
                        player.hp = target_hp;
                    } else if let Some(npc) = state.npcs.get_mut(&target_id) {
                        npc.hp = target_hp;
                    }

                    // Create floating damage number
                    state.damage_events.push(DamageEvent {
                        x: target_x,
                        y: target_y,
                        damage,
                        time: macroquad::time::get_time(),
                    });
                }
            }

            "npcDied" => {
                if let Some(value) = data {
                    let npc_id = extract_string(value, "npc_id").unwrap_or_default();
                    log::debug!("NPC died: {}", npc_id);

                    if let Some(npc) = state.npcs.get_mut(&npc_id) {
                        npc.state = NpcState::Dead;
                        npc.hp = 0;
                    }

                    // Clear selection if we had this NPC targeted
                    if state.selected_entity_id.as_ref() == Some(&npc_id) {
                        state.selected_entity_id = None;
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

            "expGained" => {
                if let Some(value) = data {
                    let player_id = extract_string(value, "player_id").unwrap_or_default();
                    let amount = extract_i32(value, "amount").unwrap_or(0);
                    let total_exp = extract_i32(value, "total_exp").unwrap_or(0);
                    let exp_to_next_level = extract_i32(value, "exp_to_next_level").unwrap_or(100);

                    log::debug!("Player {} gained {} EXP (total: {}/{})",
                        player_id, amount, total_exp, exp_to_next_level);

                    if let Some(player) = state.players.get_mut(&player_id) {
                        player.exp = total_exp;
                        player.exp_to_next_level = exp_to_next_level;
                    }
                }
            }

            "levelUp" => {
                if let Some(value) = data {
                    let player_id = extract_string(value, "player_id").unwrap_or_default();
                    let new_level = extract_i32(value, "new_level").unwrap_or(1);
                    let new_max_hp = extract_i32(value, "new_max_hp").unwrap_or(100);

                    log::info!("Player {} leveled up to {}!", player_id, new_level);

                    // Get player position for floating text
                    if let Some(player) = state.players.get_mut(&player_id) {
                        player.level = new_level;
                        player.max_hp = new_max_hp;
                        player.hp = new_max_hp; // Full heal on level up

                        // Create floating level up event
                        state.level_up_events.push(LevelUpEvent {
                            x: player.x,
                            y: player.y,
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

                    let item = GroundItem::new(id.clone(), item_id, x, y, quantity);
                    state.ground_items.insert(id, item);
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

                    log::debug!("Received chunk data: ({}, {}) with {} layers, {} collision bytes, {} objects",
                        chunk_x, chunk_y, layers.len(), collision.len(), objects.len());

                    state.chunk_manager.load_chunk(chunk_x, chunk_y, layers, &collision, objects);
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

                            // Parse equipment stats if present
                            let equipment = extract_string(item_value, "equipment_slot")
                                .map(|slot_type| {
                                    EquipmentStats {
                                        slot_type,
                                        level_required: extract_i32(item_value, "level_required").unwrap_or(1),
                                        damage_bonus: extract_i32(item_value, "damage_bonus").unwrap_or(0),
                                        defense_bonus: extract_i32(item_value, "defense_bonus").unwrap_or(0),
                                    }
                                });

                            items.push(ItemDefinition {
                                id,
                                display_name,
                                sprite,
                                category,
                                max_stack,
                                description,
                                equipment,
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
