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

    #[serde(rename = "face")]
    Face { direction: u8 },

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

    #[serde(rename = "requestChunk")]
    RequestChunk { chunk_x: i32, chunk_y: i32 },

    // Quest-related messages
    #[serde(rename = "interact")]
    Interact { npc_id: String },

    #[serde(rename = "dialogueChoice")]
    DialogueChoice { quest_id: String, choice_id: String },

    #[serde(rename = "acceptQuest")]
    AcceptQuest { quest_id: String },

    #[serde(rename = "abandonQuest")]
    AbandonQuest { quest_id: String },

    #[serde(rename = "craft")]
    Craft { recipe_id: String },

    #[serde(rename = "equip")]
    Equip { slot_index: u8 },

    #[serde(rename = "unequip")]
    Unequip { slot_type: String, target_slot: Option<u8> },

    #[serde(rename = "dropItem")]
    DropItem { slot_index: u8, quantity: u32 },

    #[serde(rename = "dropGold")]
    DropGold { amount: i32 },

    #[serde(rename = "swapSlots")]
    SwapSlots { from_slot: u8, to_slot: u8 },

    #[serde(rename = "shopBuy")]
    ShopBuy { npc_id: String, item_id: String, quantity: u32 },

    #[serde(rename = "shopSell")]
    ShopSell { npc_id: String, item_id: String, quantity: u32 },
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
            ClientMessage::Face { direction } => {
                data.insert("direction".into(), Value::Integer((*direction as i64).into()));
                "face"
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
            ClientMessage::RequestChunk { chunk_x, chunk_y } => {
                data.insert("chunkX".into(), Value::Integer((*chunk_x as i64).into()));
                data.insert("chunkY".into(), Value::Integer((*chunk_y as i64).into()));
                "requestChunk"
            }
            ClientMessage::Interact { npc_id } => {
                data.insert("npc_id".into(), Value::String(npc_id.clone().into()));
                "interact"
            }
            ClientMessage::DialogueChoice { quest_id, choice_id } => {
                data.insert("quest_id".into(), Value::String(quest_id.clone().into()));
                data.insert("choice_id".into(), Value::String(choice_id.clone().into()));
                "dialogueChoice"
            }
            ClientMessage::AcceptQuest { quest_id } => {
                data.insert("quest_id".into(), Value::String(quest_id.clone().into()));
                "acceptQuest"
            }
            ClientMessage::AbandonQuest { quest_id } => {
                data.insert("quest_id".into(), Value::String(quest_id.clone().into()));
                "abandonQuest"
            }
            ClientMessage::Craft { recipe_id } => {
                data.insert("recipe_id".into(), Value::String(recipe_id.clone().into()));
                "craft"
            }
            ClientMessage::Equip { slot_index } => {
                data.insert("slot_index".into(), Value::Integer((*slot_index as i64).into()));
                "equip"
            }
            ClientMessage::Unequip { slot_type, target_slot } => {
                data.insert("slot_type".into(), Value::String(slot_type.clone().into()));
                if let Some(slot) = target_slot {
                    data.insert("target_slot".into(), Value::Integer((*slot as i64).into()));
                }
                "unequip"
            }
            ClientMessage::DropItem { slot_index, quantity } => {
                data.insert("slot_index".into(), Value::Integer((*slot_index as i64).into()));
                data.insert("quantity".into(), Value::Integer((*quantity as i64).into()));
                "dropItem"
            }
            ClientMessage::DropGold { amount } => {
                data.insert("amount".into(), Value::Integer((*amount as i64).into()));
                "dropGold"
            }
            ClientMessage::SwapSlots { from_slot, to_slot } => {
                data.insert("from_slot".into(), Value::Integer((*from_slot as i64).into()));
                data.insert("to_slot".into(), Value::Integer((*to_slot as i64).into()));
                "swapSlots"
            }
            ClientMessage::ShopBuy { npc_id, item_id, quantity } => {
                data.insert("npcId".into(), Value::String(npc_id.clone().into()));
                data.insert("itemId".into(), Value::String(item_id.clone().into()));
                data.insert("quantity".into(), Value::Integer((*quantity as i64).into()));
                "shopBuy"
            }
            ClientMessage::ShopSell { npc_id, item_id, quantity } => {
                data.insert("npcId".into(), Value::String(npc_id.clone().into()));
                data.insert("itemId".into(), Value::String(item_id.clone().into()));
                data.insert("quantity".into(), Value::Integer((*quantity as i64).into()));
                "shopSell"
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
