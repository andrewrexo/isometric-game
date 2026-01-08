use serde::Serialize;
use std::io::Cursor;

// Colyseus protocol codes (from colyseus/src/Protocol.ts)
#[repr(u8)]
#[allow(dead_code)]
pub enum Protocol {
    Handshake = 9,
    JoinRoom = 10,
    Error = 11,
    LeaveRoom = 12,
    RoomData = 13,
    RoomState = 14,
    RoomStatePatch = 15,
    RoomDataSchema = 16,
    RoomDataBytes = 17,
}

/// Encode a message for Colyseus in MessagePack format
/// Format: [Protocol.ROOM_DATA, message_type, message_data]
pub fn encode_message<T: Serialize>(message_type: &str, data: &T) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    // Colyseus expects: [13, "type", data]
    let message: (u8, &str, &T) = (Protocol::RoomData as u8, message_type, data);
    rmp_serde::to_vec(&message)
}

/// Decode a Colyseus message from MessagePack format
/// Returns (protocol_code, message_type, raw_data)
pub fn decode_message(data: &[u8]) -> Result<DecodedMessage, DecodeError> {
    let mut cursor = Cursor::new(data);
    let value = rmpv::decode::read_value(&mut cursor)
        .map_err(|e| DecodeError::MsgpackError(e.to_string()))?;

    let array = value.as_array()
        .ok_or(DecodeError::InvalidFormat("Expected array".into()))?;

    if array.is_empty() {
        return Err(DecodeError::InvalidFormat("Empty array".into()));
    }

    let protocol = array[0].as_u64()
        .ok_or(DecodeError::InvalidFormat("Protocol code must be integer".into()))? as u8;

    match protocol {
        9 => {
            // Handshake - not typically received
            Ok(DecodedMessage::Handshake)
        }
        11 => {
            // Error
            let code = array.get(1).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let message = array.get(2).and_then(|v| v.as_str()).unwrap_or("Unknown error").to_string();
            Ok(DecodedMessage::Error { code, message })
        }
        13 => {
            // RoomData - standard message
            if array.len() < 2 {
                return Err(DecodeError::InvalidFormat("RoomData missing type".into()));
            }

            let msg_type = array[1].as_str()
                .ok_or(DecodeError::InvalidFormat("Message type must be string".into()))?
                .to_string();

            let msg_data = if array.len() > 2 {
                Some(array[2].clone())
            } else {
                None
            };

            Ok(DecodedMessage::RoomData { msg_type, data: msg_data })
        }
        14 => {
            // RoomState - full state (Colyseus Schema binary)
            Ok(DecodedMessage::RoomState { data: data.to_vec() })
        }
        15 => {
            // RoomStatePatch - state delta (Colyseus Schema binary)
            Ok(DecodedMessage::RoomStatePatch { data: data.to_vec() })
        }
        _ => {
            Ok(DecodedMessage::Unknown { protocol, data: data.to_vec() })
        }
    }
}

#[derive(Debug)]
pub enum DecodedMessage {
    Handshake,
    Error { code: u32, message: String },
    RoomData { msg_type: String, data: Option<rmpv::Value> },
    RoomState { data: Vec<u8> },
    RoomStatePatch { data: Vec<u8> },
    Unknown { protocol: u8, data: Vec<u8> },
}

#[derive(Debug)]
pub enum DecodeError {
    MsgpackError(String),
    InvalidFormat(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::MsgpackError(e) => write!(f, "MessagePack error: {}", e),
            DecodeError::InvalidFormat(e) => write!(f, "Invalid format: {}", e),
        }
    }
}

/// Helper to extract typed data from a rmpv::Value
pub fn extract_string(value: &rmpv::Value, key: &str) -> Option<String> {
    value.as_map()
        .and_then(|map| {
            map.iter()
                .find(|(k, _)| k.as_str() == Some(key))
                .and_then(|(_, v)| v.as_str().map(|s| s.to_string()))
        })
}

pub fn extract_f32(value: &rmpv::Value, key: &str) -> Option<f32> {
    value.as_map()
        .and_then(|map| {
            map.iter()
                .find(|(k, _)| k.as_str() == Some(key))
                .and_then(|(_, v)| {
                    v.as_f64().map(|f| f as f32)
                        .or_else(|| v.as_i64().map(|i| i as f32))
                        .or_else(|| v.as_u64().map(|u| u as f32))
                })
        })
}

pub fn extract_i32(value: &rmpv::Value, key: &str) -> Option<i32> {
    value.as_map()
        .and_then(|map| {
            map.iter()
                .find(|(k, _)| k.as_str() == Some(key))
                .and_then(|(_, v)| v.as_i64().map(|i| i as i32)
                    .or_else(|| v.as_u64().map(|u| u as i32)))
        })
}

pub fn extract_u64(value: &rmpv::Value, key: &str) -> Option<u64> {
    value.as_map()
        .and_then(|map| {
            map.iter()
                .find(|(k, _)| k.as_str() == Some(key))
                .and_then(|(_, v)| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
        })
}

pub fn extract_array<'a>(value: &'a rmpv::Value, key: &str) -> Option<&'a Vec<rmpv::Value>> {
    value.as_map()
        .and_then(|map| {
            map.iter()
                .find(|(k, _)| k.as_str() == Some(key))
                .and_then(|(_, v)| v.as_array())
        })
}

pub fn extract_u8(value: &rmpv::Value, key: &str) -> Option<u8> {
    value.as_map()
        .and_then(|map| {
            map.iter()
                .find(|(k, _)| k.as_str() == Some(key))
                .and_then(|(_, v)| v.as_u64().map(|u| u as u8)
                    .or_else(|| v.as_i64().map(|i| i as u8)))
        })
}

pub fn extract_bool(value: &rmpv::Value, key: &str) -> Option<bool> {
    value.as_map()
        .and_then(|map| {
            map.iter()
                .find(|(k, _)| k.as_str() == Some(key))
                .and_then(|(_, v)| v.as_bool())
        })
}
