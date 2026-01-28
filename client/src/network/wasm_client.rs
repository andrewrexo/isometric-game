use sapp_jsutils::JsObject;

use crate::game::GameState;
use crate::game::state::ConnectionStatus;
use super::messages::ClientMessage;
use super::protocol;

extern "C" {
    fn ws_connect(url: JsObject);
    fn ws_disconnect();
    fn ws_send(data: JsObject);
    fn ws_try_recv() -> JsObject;
    fn ws_is_connected() -> i32;
    fn ws_has_error() -> i32;
    fn ws_has_closed() -> i32;
    fn ws_has_opened() -> i32;
}

#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    Disconnected,
    Matchmaking,
    Connecting,
    Connected,
}

pub struct NetworkClient {
    base_url: String,
    connection_state: ConnectionState,
    reconnect_timer: f32,
    room_id: Option<String>,
    session_token: Option<String>,
    auth_token: Option<String>,
    character_id: Option<i64>,
    reconnect_attempts: u32,
    was_connected: bool,
}

const MAX_RECONNECT_ATTEMPTS: u32 = 3;

impl NetworkClient {
    pub fn new(base_url: &str) -> Self {
        Self::new_guest(base_url)
    }

    pub fn new_guest(base_url: &str) -> Self {
        let mut client = Self {
            base_url: base_url.to_string(),
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

    pub fn new_authenticated(base_url: &str, auth_token: &str, character_id: i64) -> Self {
        let mut client = Self {
            base_url: base_url.to_string(),
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

    pub fn new_with_token(base_url: &str, auth_token: &str, _player_name: &str) -> Self {
        let mut client = Self {
            base_url: base_url.to_string(),
            connection_state: ConnectionState::Disconnected,
            reconnect_timer: 0.0,
            room_id: None,
            session_token: None,
            auth_token: Some(auth_token.to_string()),
            character_id: None,
            reconnect_attempts: 0,
            was_connected: false,
        };
        client.start_matchmaking();
        client
    }

    fn start_matchmaking(&mut self) {
        let room_id = quad_storage::STORAGE.lock().unwrap().get("roomId");
        let session_token = quad_storage::STORAGE.lock().unwrap().get("sessionToken");

        if let (Some(rid), Some(token)) = (room_id, session_token) {
            log::info!("WASM: Connecting with roomId={}", rid);
            self.room_id = Some(rid);
            self.session_token = Some(token);
            self.connect_websocket();
        } else {
            log::error!("WASM: Missing roomId or sessionToken in localStorage");
            self.connection_state = ConnectionState::Disconnected;
        }
    }

    fn connect_websocket(&mut self) {
        let room_id = match &self.room_id {
            Some(id) => id.clone(),
            None => return,
        };
        let token = match &self.session_token {
            Some(t) => t.clone(),
            None => {
                log::error!("No session token available");
                return;
            }
        };

        let ws_url = format!("{}/{}?sessionToken={}", self.base_url, room_id, token);
        log::info!("WASM: Connecting WebSocket: {}...", &ws_url[..ws_url.len().min(80)]);

        self.connection_state = ConnectionState::Connecting;

        unsafe {
            ws_connect(JsObject::string(&ws_url));
        }
    }

    pub fn poll(&mut self, state: &mut GameState) {
        match self.connection_state {
            ConnectionState::Disconnected => {
                if self.was_connected {
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

        let mut should_disconnect = false;

        unsafe {
            if ws_has_opened() == 1 {
                log::info!("WebSocket connected!");
                self.connection_state = ConnectionState::Connected;
                state.connection_status = ConnectionStatus::Connected;
                self.reconnect_attempts = 0;
                self.was_connected = true;
            }

            if ws_has_error() == 1 {
                log::error!("WebSocket error");
                self.connection_state = ConnectionState::Disconnected;
                state.connection_status = ConnectionStatus::Disconnected;
                should_disconnect = true;
            }

            if ws_has_closed() == 1 {
                log::info!("WebSocket disconnected");
                self.connection_state = ConnectionState::Disconnected;
                state.connection_status = ConnectionStatus::Disconnected;
                should_disconnect = true;
            }

            // Drain received messages
            loop {
                let obj = ws_try_recv();
                if obj.is_nil() {
                    break;
                }
                let mut buf = Vec::new();
                obj.to_byte_buffer(&mut buf);
                self.handle_binary_message(&buf, state);
            }
        }

        if should_disconnect {
            self.room_id = None;
            self.session_token = None;
        }
    }

    fn handle_binary_message(&self, data: &[u8], state: &mut GameState) {
        log::trace!("Received {} bytes", data.len());
        match protocol::decode_message(data) {
            Ok(decoded) => {
                match decoded {
                    protocol::DecodedMessage::RoomData { msg_type, data } => {
                        super::message_handler::handle_room_data(&msg_type, data.as_ref(), state);
                    }
                    protocol::DecodedMessage::RoomState { .. } => {
                        log::debug!("Received RoomState (ignored)");
                    }
                    protocol::DecodedMessage::RoomStatePatch { .. } => {
                        log::debug!("Received RoomStatePatch (ignored)");
                    }
                    protocol::DecodedMessage::Error { code, message } => {
                        log::error!("Server error {}: {}", code, message);
                    }
                    protocol::DecodedMessage::Handshake => {
                        log::debug!("Received Handshake");
                    }
                    protocol::DecodedMessage::Unknown { protocol: proto, .. } => {
                        log::debug!("Unknown protocol: {}", proto);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to decode message: {}", e);
            }
        }
    }

    pub fn send(&mut self, msg: &ClientMessage) {
        if self.connection_state == ConnectionState::Connected {
            let (msg_type, msg_data) = msg.to_protocol();
            match protocol::encode_message(msg_type, &msg_data) {
                Ok(bytes) => {
                    unsafe {
                        ws_send(JsObject::buffer(&bytes));
                    }
                }
                Err(e) => {
                    log::error!("Failed to encode message: {}", e);
                }
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }

    pub fn disconnect(&mut self) {
        unsafe {
            ws_disconnect();
        }
        self.connection_state = ConnectionState::Disconnected;
        self.room_id = None;
        self.session_token = None;
        log::info!("Disconnected from server");
    }
}
