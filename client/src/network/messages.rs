use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// === Client -> Server Messages ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "auth")]
    Auth { username: String, password: String },

    #[serde(rename = "register")]
    Register { username: String, password: String },

    #[serde(rename = "move")]
    Move { dx: f32, dy: f32 },

    #[serde(rename = "moveTo")]
    MoveTo { x: f32, y: f32 },

    #[serde(rename = "target")]
    Target { entity_id: String },

    #[serde(rename = "attack")]
    Attack,

    #[serde(rename = "chat")]
    Chat { text: String },

    #[serde(rename = "pickup")]
    Pickup { item_id: String },

    #[serde(rename = "useItem")]
    UseItem { slot_index: u32 },
}

impl ClientMessage {
    /// Convert message to Colyseus protocol format (type, data)
    pub fn to_protocol(&self) -> (&'static str, HashMap<String, rmpv::Value>) {
        use rmpv::Value;

        let mut data = HashMap::new();

        let msg_type = match self {
            ClientMessage::Auth { username, password } => {
                data.insert("username".into(), Value::String(username.clone().into()));
                data.insert("password".into(), Value::String(password.clone().into()));
                "auth"
            }
            ClientMessage::Register { username, password } => {
                data.insert("username".into(), Value::String(username.clone().into()));
                data.insert("password".into(), Value::String(password.clone().into()));
                "register"
            }
            ClientMessage::Move { dx, dy } => {
                data.insert("dx".into(), Value::F64(*dx as f64));
                data.insert("dy".into(), Value::F64(*dy as f64));
                "move"
            }
            ClientMessage::MoveTo { x, y } => {
                data.insert("x".into(), Value::F64(*x as f64));
                data.insert("y".into(), Value::F64(*y as f64));
                "moveTo"
            }
            ClientMessage::Target { entity_id } => {
                data.insert("entity_id".into(), Value::String(entity_id.clone().into()));
                "target"
            }
            ClientMessage::Attack => "attack",
            ClientMessage::Chat { text } => {
                data.insert("text".into(), Value::String(text.clone().into()));
                "chat"
            }
            ClientMessage::Pickup { item_id } => {
                data.insert("item_id".into(), Value::String(item_id.clone().into()));
                "pickup"
            }
            ClientMessage::UseItem { slot_index } => {
                data.insert("slot_index".into(), Value::Integer((*slot_index as i64).into()));
                "useItem"
            }
        };

        (msg_type, data)
    }
}

// Note: Server messages are handled via the protocol module's MessagePack decoder
// The ServerMessage enum below is kept for reference but is not directly used

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "welcome")]
    Welcome { player_id: String },

    #[serde(rename = "error")]
    Error { code: u32, message: String },

    #[serde(rename = "playerJoined")]
    PlayerJoined {
        id: String,
        name: String,
        x: f32,
        y: f32,
    },

    #[serde(rename = "playerLeft")]
    PlayerLeft { id: String },

    #[serde(rename = "stateSync")]
    StateSync {
        tick: u64,
        players: Vec<PlayerUpdate>,
    },

    #[serde(rename = "chatMessage")]
    ChatMessage {
        sender_name: String,
        text: String,
        timestamp: f64,
    },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerUpdate {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hp: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxHp")]
    pub max_hp: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "expToNextLevel")]
    pub exp_to_next_level: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gold: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_moving: Option<bool>,
}
