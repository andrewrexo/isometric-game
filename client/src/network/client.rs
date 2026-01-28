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
        super::message_handler::handle_room_data(msg_type, data, state);
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
