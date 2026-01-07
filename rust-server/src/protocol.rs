use serde::{Deserialize, Serialize};

use crate::game::PlayerUpdate;
use crate::npc::NpcUpdate;

// ============================================================================
// Client -> Server Messages
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "move")]
    Move { dx: f32, dy: f32 },

    #[serde(rename = "chat")]
    Chat { text: String },

    #[serde(rename = "attack")]
    Attack,

    #[serde(rename = "target")]
    Target { entity_id: String },

    #[serde(rename = "pickup")]
    Pickup { item_id: String },

    #[serde(rename = "useItem")]
    UseItem { slot_index: u8 },

    #[serde(rename = "auth")]
    Auth { username: String, password: String },

    #[serde(rename = "register")]
    Register { username: String, password: String },
}

// ============================================================================
// Server -> Client Messages
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ServerMessage {
    Welcome {
        player_id: String,
    },
    PlayerJoined {
        id: String,
        name: String,
        x: i32,
        y: i32,
    },
    PlayerLeft {
        id: String,
    },
    StateSync {
        tick: u64,
        players: Vec<PlayerUpdate>,
        npcs: Vec<NpcUpdate>,
    },
    ChatMessage {
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "senderName")]
        sender_name: String,
        text: String,
        timestamp: u64,
    },
    TargetChanged {
        player_id: String,
        target_id: Option<String>,
    },
    DamageEvent {
        source_id: String,
        target_id: String,
        damage: i32,
        target_hp: i32,
        target_x: f32,
        target_y: f32,
    },
    AttackResult {
        success: bool,
        reason: Option<String>,
    },
    NpcDied {
        id: String,
        killer_id: String,
    },
    NpcRespawned {
        id: String,
        x: i32,
        y: i32,
    },
    PlayerDied {
        id: String,
        killer_id: String,
    },
    PlayerRespawned {
        id: String,
        x: i32,
        y: i32,
        hp: i32,
    },
    ExpGained {
        player_id: String,
        amount: i32,
        total_exp: i32,
        exp_to_next_level: i32,
    },
    LevelUp {
        player_id: String,
        new_level: i32,
        new_max_hp: i32,
    },
    ItemDropped {
        id: String,
        item_type: u8,
        x: f32,
        y: f32,
        quantity: i32,
    },
    ItemPickedUp {
        item_id: String,
        player_id: String,
    },
    ItemDespawned {
        item_id: String,
    },
    InventoryUpdate {
        slots: Vec<crate::item::InventorySlotUpdate>,
        gold: i32,
    },
    ItemUsed {
        slot: u8,
        item_type: u8,
        effect: String, // e.g., "heal:30"
    },
    Error {
        code: u32,
        message: String,
    },
}

impl ServerMessage {
    pub fn msg_type(&self) -> &'static str {
        match self {
            ServerMessage::Welcome { .. } => "welcome",
            ServerMessage::PlayerJoined { .. } => "playerJoined",
            ServerMessage::PlayerLeft { .. } => "playerLeft",
            ServerMessage::StateSync { .. } => "stateSync",
            ServerMessage::ChatMessage { .. } => "chatMessage",
            ServerMessage::TargetChanged { .. } => "targetChanged",
            ServerMessage::DamageEvent { .. } => "damageEvent",
            ServerMessage::AttackResult { .. } => "attackResult",
            ServerMessage::NpcDied { .. } => "npcDied",
            ServerMessage::NpcRespawned { .. } => "npcRespawned",
            ServerMessage::PlayerDied { .. } => "playerDied",
            ServerMessage::PlayerRespawned { .. } => "playerRespawned",
            ServerMessage::ExpGained { .. } => "expGained",
            ServerMessage::LevelUp { .. } => "levelUp",
            ServerMessage::ItemDropped { .. } => "itemDropped",
            ServerMessage::ItemPickedUp { .. } => "itemPickedUp",
            ServerMessage::ItemDespawned { .. } => "itemDespawned",
            ServerMessage::InventoryUpdate { .. } => "inventoryUpdate",
            ServerMessage::ItemUsed { .. } => "itemUsed",
            ServerMessage::Error { .. } => "error",
        }
    }
}

// ============================================================================
// Encoding/Decoding
// ============================================================================

/// Encode a server message to MessagePack format
/// Format: [13, "msg_type", {data}] (matching Colyseus ROOM_DATA protocol)
pub fn encode_server_message(msg: &ServerMessage) -> Result<Vec<u8>, String> {
    use rmpv::Value;

    let msg_type = msg.msg_type();

    // Convert message to rmpv::Value
    let data = match msg {
        ServerMessage::Welcome { player_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::PlayerJoined { id, name, x, y } => {
            let mut map = Vec::new();
            map.push((Value::String("id".into()), Value::String(id.clone().into())));
            map.push((
                Value::String("name".into()),
                Value::String(name.clone().into()),
            ));
            map.push((Value::String("x".into()), Value::Integer((*x as i64).into())));
            map.push((Value::String("y".into()), Value::Integer((*y as i64).into())));
            Value::Map(map)
        }
        ServerMessage::PlayerLeft { id } => {
            let mut map = Vec::new();
            map.push((Value::String("id".into()), Value::String(id.clone().into())));
            Value::Map(map)
        }
        ServerMessage::StateSync { tick, players, npcs } => {
            let mut map = Vec::new();
            map.push((Value::String("tick".into()), Value::Integer((*tick).into())));

            let player_values: Vec<Value> = players
                .iter()
                .map(|p| {
                    let mut pmap = Vec::new();
                    pmap.push((
                        Value::String("id".into()),
                        Value::String(p.id.clone().into()),
                    ));
                    pmap.push((Value::String("x".into()), Value::Integer((p.x as i64).into())));
                    pmap.push((Value::String("y".into()), Value::Integer((p.y as i64).into())));
                    pmap.push((
                        Value::String("direction".into()),
                        Value::Integer((p.direction as i64).into()),
                    ));
                    pmap.push((Value::String("hp".into()), Value::Integer((p.hp as i64).into())));
                    Value::Map(pmap)
                })
                .collect();
            map.push((Value::String("players".into()), Value::Array(player_values)));

            let npc_values: Vec<Value> = npcs
                .iter()
                .map(|n| {
                    let mut nmap = Vec::new();
                    nmap.push((
                        Value::String("id".into()),
                        Value::String(n.id.clone().into()),
                    ));
                    nmap.push((Value::String("npc_type".into()), Value::Integer((n.npc_type as i64).into())));
                    nmap.push((Value::String("x".into()), Value::Integer((n.x as i64).into())));
                    nmap.push((Value::String("y".into()), Value::Integer((n.y as i64).into())));
                    nmap.push((Value::String("direction".into()), Value::Integer((n.direction as i64).into())));
                    nmap.push((Value::String("hp".into()), Value::Integer((n.hp as i64).into())));
                    nmap.push((Value::String("max_hp".into()), Value::Integer((n.max_hp as i64).into())));
                    nmap.push((Value::String("level".into()), Value::Integer((n.level as i64).into())));
                    nmap.push((Value::String("state".into()), Value::Integer((n.state as i64).into())));
                    Value::Map(nmap)
                })
                .collect();
            map.push((Value::String("npcs".into()), Value::Array(npc_values)));

            Value::Map(map)
        }
        ServerMessage::ChatMessage {
            sender_id,
            sender_name,
            text,
            timestamp,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("senderId".into()),
                Value::String(sender_id.clone().into()),
            ));
            map.push((
                Value::String("senderName".into()),
                Value::String(sender_name.clone().into()),
            ));
            map.push((
                Value::String("text".into()),
                Value::String(text.clone().into()),
            ));
            map.push((
                Value::String("timestamp".into()),
                Value::Integer((*timestamp).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::TargetChanged {
            player_id,
            target_id,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            map.push((
                Value::String("target_id".into()),
                match target_id {
                    Some(id) => Value::String(id.clone().into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
        ServerMessage::DamageEvent {
            source_id,
            target_id,
            damage,
            target_hp,
            target_x,
            target_y,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("source_id".into()),
                Value::String(source_id.clone().into()),
            ));
            map.push((
                Value::String("target_id".into()),
                Value::String(target_id.clone().into()),
            ));
            map.push((
                Value::String("damage".into()),
                Value::Integer((*damage as i64).into()),
            ));
            map.push((
                Value::String("target_hp".into()),
                Value::Integer((*target_hp as i64).into()),
            ));
            map.push((
                Value::String("target_x".into()),
                Value::F64(*target_x as f64),
            ));
            map.push((
                Value::String("target_y".into()),
                Value::F64(*target_y as f64),
            ));
            Value::Map(map)
        }
        ServerMessage::AttackResult { success, reason } => {
            let mut map = Vec::new();
            map.push((
                Value::String("success".into()),
                Value::Boolean(*success),
            ));
            map.push((
                Value::String("reason".into()),
                match reason {
                    Some(r) => Value::String(r.clone().into()),
                    None => Value::Nil,
                },
            ));
            Value::Map(map)
        }
        ServerMessage::NpcDied { id, killer_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((
                Value::String("killer_id".into()),
                Value::String(killer_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::NpcRespawned { id, x, y } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((Value::String("x".into()), Value::Integer((*x as i64).into())));
            map.push((Value::String("y".into()), Value::Integer((*y as i64).into())));
            Value::Map(map)
        }
        ServerMessage::PlayerDied { id, killer_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((
                Value::String("killer_id".into()),
                Value::String(killer_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::PlayerRespawned { id, x, y, hp } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((Value::String("x".into()), Value::Integer((*x as i64).into())));
            map.push((Value::String("y".into()), Value::Integer((*y as i64).into())));
            map.push((Value::String("hp".into()), Value::Integer((*hp as i64).into())));
            Value::Map(map)
        }
        ServerMessage::ExpGained {
            player_id,
            amount,
            total_exp,
            exp_to_next_level,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            map.push((
                Value::String("amount".into()),
                Value::Integer((*amount as i64).into()),
            ));
            map.push((
                Value::String("total_exp".into()),
                Value::Integer((*total_exp as i64).into()),
            ));
            map.push((
                Value::String("exp_to_next_level".into()),
                Value::Integer((*exp_to_next_level as i64).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::LevelUp {
            player_id,
            new_level,
            new_max_hp,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            map.push((
                Value::String("new_level".into()),
                Value::Integer((*new_level as i64).into()),
            ));
            map.push((
                Value::String("new_max_hp".into()),
                Value::Integer((*new_max_hp as i64).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::ItemDropped {
            id,
            item_type,
            x,
            y,
            quantity,
        } => {
            let mut map = Vec::new();
            map.push((
                Value::String("id".into()),
                Value::String(id.clone().into()),
            ));
            map.push((
                Value::String("item_type".into()),
                Value::Integer((*item_type as i64).into()),
            ));
            map.push((Value::String("x".into()), Value::F64(*x as f64)));
            map.push((Value::String("y".into()), Value::F64(*y as f64)));
            map.push((
                Value::String("quantity".into()),
                Value::Integer((*quantity as i64).into()),
            ));
            Value::Map(map)
        }
        ServerMessage::ItemPickedUp { item_id, player_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("item_id".into()),
                Value::String(item_id.clone().into()),
            ));
            map.push((
                Value::String("player_id".into()),
                Value::String(player_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::ItemDespawned { item_id } => {
            let mut map = Vec::new();
            map.push((
                Value::String("item_id".into()),
                Value::String(item_id.clone().into()),
            ));
            Value::Map(map)
        }
        ServerMessage::InventoryUpdate { slots, gold } => {
            let mut map = Vec::new();

            let slot_values: Vec<Value> = slots.iter().map(|s| {
                let mut smap = Vec::new();
                smap.push((Value::String("slot".into()), Value::Integer((s.slot as i64).into())));
                smap.push((Value::String("item_type".into()), Value::Integer((s.item_type as i64).into())));
                smap.push((Value::String("quantity".into()), Value::Integer((s.quantity as i64).into())));
                Value::Map(smap)
            }).collect();

            map.push((Value::String("slots".into()), Value::Array(slot_values)));
            map.push((Value::String("gold".into()), Value::Integer((*gold as i64).into())));
            Value::Map(map)
        }
        ServerMessage::ItemUsed { slot, item_type, effect } => {
            let mut map = Vec::new();
            map.push((Value::String("slot".into()), Value::Integer((*slot as i64).into())));
            map.push((Value::String("item_type".into()), Value::Integer((*item_type as i64).into())));
            map.push((Value::String("effect".into()), Value::String(effect.clone().into())));
            Value::Map(map)
        }
        ServerMessage::Error { code, message } => {
            let mut map = Vec::new();
            map.push((
                Value::String("code".into()),
                Value::Integer((*code as i64).into()),
            ));
            map.push((
                Value::String("message".into()),
                Value::String(message.clone().into()),
            ));
            Value::Map(map)
        }
    };

    // Encode as [13, "msg_type", data] - matching Colyseus ROOM_DATA format
    let array = Value::Array(vec![
        Value::Integer(13.into()), // Protocol.RoomData
        Value::String(msg_type.into()),
        data,
    ]);

    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &array)
        .map_err(|e| format!("Failed to encode message: {}", e))?;

    Ok(buf)
}

/// Decode a client message from MessagePack format
/// Expected format: [13, "msg_type", {data}]
pub fn decode_client_message(data: &[u8]) -> Result<ClientMessage, String> {
    use rmpv::Value;
    use std::io::Cursor;

    let mut cursor = Cursor::new(data);
    let value = rmpv::decode::read_value(&mut cursor)
        .map_err(|e| format!("Failed to decode MessagePack: {}", e))?;

    let array = value
        .as_array()
        .ok_or("Expected array")?;

    if array.len() < 2 {
        return Err("Array too short".to_string());
    }

    let protocol = array[0]
        .as_u64()
        .ok_or("Protocol code must be integer")? as u8;

    if protocol != 13 {
        return Err(format!("Unexpected protocol code: {}", protocol));
    }

    let msg_type = array[1]
        .as_str()
        .ok_or("Message type must be string")?;

    let msg_data = if array.len() > 2 {
        &array[2]
    } else {
        &Value::Nil
    };

    match msg_type {
        "move" => {
            let dx = extract_f32(msg_data, "dx").unwrap_or(0.0);
            let dy = extract_f32(msg_data, "dy").unwrap_or(0.0);
            Ok(ClientMessage::Move { dx, dy })
        }
        "chat" => {
            let text = extract_string(msg_data, "text").unwrap_or_default();
            Ok(ClientMessage::Chat { text })
        }
        "attack" => Ok(ClientMessage::Attack),
        "target" => {
            let entity_id = extract_string(msg_data, "entity_id").unwrap_or_default();
            Ok(ClientMessage::Target { entity_id })
        }
        "pickup" => {
            let item_id = extract_string(msg_data, "item_id").unwrap_or_default();
            Ok(ClientMessage::Pickup { item_id })
        }
        "useItem" => {
            let slot_index = msg_data.as_map()
                .and_then(|map| map.iter().find(|(k, _)| k.as_str() == Some("slot_index")))
                .and_then(|(_, v)| v.as_u64().map(|u| u as u8))
                .unwrap_or(0);
            Ok(ClientMessage::UseItem { slot_index })
        }
        "auth" => {
            let username = extract_string(msg_data, "username").unwrap_or_default();
            let password = extract_string(msg_data, "password").unwrap_or_default();
            Ok(ClientMessage::Auth { username, password })
        }
        "register" => {
            let username = extract_string(msg_data, "username").unwrap_or_default();
            let password = extract_string(msg_data, "password").unwrap_or_default();
            Ok(ClientMessage::Register { username, password })
        }
        _ => Err(format!("Unknown message type: {}", msg_type)),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_string(value: &rmpv::Value, key: &str) -> Option<String> {
    value.as_map().and_then(|map| {
        map.iter()
            .find(|(k, _)| k.as_str() == Some(key))
            .and_then(|(_, v)| v.as_str().map(|s| s.to_string()))
    })
}

fn extract_f32(value: &rmpv::Value, key: &str) -> Option<f32> {
    value.as_map().and_then(|map| {
        map.iter()
            .find(|(k, _)| k.as_str() == Some(key))
            .and_then(|(_, v)| {
                v.as_f64()
                    .map(|f| f as f32)
                    .or_else(|| v.as_i64().map(|i| i as f32))
                    .or_else(|| v.as_u64().map(|u| u as f32))
            })
    })
}
