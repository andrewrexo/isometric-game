use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use serde::{Deserialize, Serialize};
use crate::game::{GameState, ConnectionStatus, Player, Direction, ChatMessage, DamageEvent, LevelUpEvent, GroundItem, ItemType, InventorySlot};
use crate::game::npc::{Npc, NpcType, NpcState};
use super::messages::ClientMessage;
use super::protocol::{self, DecodedMessage, extract_string, extract_f32, extract_i32, extract_u64, extract_array, extract_u8};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchmakeResponse {
    room: RoomInfo,
    session_id: String,
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

#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    Disconnected,
    Matchmaking,
    Connecting,
    Connected,
}

pub struct NetworkClient {
    sender: Option<WsSender>,
    receiver: Option<WsReceiver>,
    base_url: String,
    player_name: String,
    connection_state: ConnectionState,
    reconnect_timer: f32,
    room_id: Option<String>,
    session_id: Option<String>,
}

impl NetworkClient {
    pub fn new(base_url: &str) -> Self {
        let mut client = Self {
            sender: None,
            receiver: None,
            base_url: base_url.to_string(),
            player_name: format!("Player{}", rand::random::<u16>() % 10000),
            connection_state: ConnectionState::Disconnected,
            reconnect_timer: 0.0,
            room_id: None,
            session_id: None,
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

        let matchmake_url = format!("{}/matchmake/joinOrCreate/game", http_url);
        log::info!("Matchmaking: POST {}", matchmake_url);

        let options = JoinOptions {
            name: self.player_name.clone(),
        };

        match ureq::post(&matchmake_url)
            .set("Content-Type", "application/json")
            .send_json(&options)
        {
            Ok(response) => {
                match response.into_json::<MatchmakeResponse>() {
                    Ok(data) => {
                        log::info!("Matchmaking success: room={}, session={}",
                            data.room.room_id, data.session_id);
                        self.room_id = Some(data.room.room_id);
                        self.session_id = Some(data.session_id);
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
        let session_id = quad_storage::STORAGE.lock().unwrap().get("sessionId");

        if let (Some(rid), Some(sid)) = (room_id, session_id) {
            log::info!("WASM: Connecting with roomId={}, sessionId={}", rid, sid);
            self.room_id = Some(rid);
            self.session_id = Some(sid);
            self.connect_websocket();
        } else {
            log::error!("WASM: Missing roomId or sessionId in localStorage. JavaScript should matchmake first.");
            self.connection_state = ConnectionState::Disconnected;
        }
    }

    fn connect_websocket(&mut self) {
        let room_id = match &self.room_id {
            Some(id) => id,
            None => return,
        };
        let session_id = match &self.session_id {
            Some(id) => id,
            None => return,
        };

        let ws_url = format!("{}/{}?sessionId={}", self.base_url, room_id, session_id);
        log::info!("Connecting WebSocket: {}", ws_url);

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
                self.reconnect_timer += 1.0 / 60.0;
                if self.reconnect_timer > 2.0 {
                    self.reconnect_timer = 0.0;
                    self.start_matchmaking();
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

                    if let Some(session_id) = &self.session_id {
                        state.local_player_id = Some(session_id.clone());
                    }
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
            self.session_id = None;
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

                    log::info!("Player joined: {} at ({}, {})", name, x, y);
                    let player = Player::new(id.clone(), name, x, y);
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

                            if let Some(player) = state.players.get_mut(&id) {
                                if let (Some(x), Some(y)) = (x, y) {
                                    // Set server target - client will smoothly interpolate
                                    player.set_server_position(x as f32, y as f32);
                                }
                                if let Some(dir) = direction {
                                    player.direction = Direction::from_u8(dir as u8);
                                }
                                if let Some(hp) = hp {
                                    player.hp = hp;
                                }
                            }
                        }
                    }

                    // Update NPCs (grid positions from server, converted to f32 for interpolation)
                    if let Some(npcs) = extract_array(value, "npcs") {
                        for npc_value in npcs {
                            let id = extract_string(npc_value, "id").unwrap_or_default();
                            let npc_type = extract_u8(npc_value, "npc_type").unwrap_or(0);
                            // Server sends i32 grid positions
                            let x = extract_i32(npc_value, "x").unwrap_or(0) as f32;
                            let y = extract_i32(npc_value, "y").unwrap_or(0) as f32;
                            let direction = extract_u8(npc_value, "direction").unwrap_or(0);
                            let hp = extract_i32(npc_value, "hp").unwrap_or(50);
                            let max_hp = extract_i32(npc_value, "max_hp").unwrap_or(50);
                            let level = extract_i32(npc_value, "level").unwrap_or(1);
                            let npc_state = extract_u8(npc_value, "state").unwrap_or(0);

                            if let Some(npc) = state.npcs.get_mut(&id) {
                                // Update existing NPC - interpolate toward new grid position
                                npc.set_server_position(x, y);
                                npc.direction = Direction::from_u8(direction);
                                npc.hp = hp;
                                npc.max_hp = max_hp;
                                npc.state = NpcState::from_u8(npc_state);
                            } else {
                                // New NPC - add to state
                                let mut npc = Npc::new(id.clone(), NpcType::from_u8(npc_type), x, y);
                                npc.direction = Direction::from_u8(direction);
                                npc.hp = hp;
                                npc.max_hp = max_hp;
                                npc.level = level;
                                npc.state = NpcState::from_u8(npc_state);
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

                    state.ui_state.chat_messages.push(ChatMessage {
                        sender_name,
                        text,
                        timestamp,
                    });

                    if state.ui_state.chat_messages.len() > 100 {
                        state.ui_state.chat_messages.remove(0);
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
                    let item_type = extract_u8(value, "item_type").unwrap_or(2);
                    let x = extract_f32(value, "x").unwrap_or(0.0);
                    let y = extract_f32(value, "y").unwrap_or(0.0);
                    let quantity = extract_i32(value, "quantity").unwrap_or(1);

                    log::debug!("Item dropped: {} at ({}, {})", id, x, y);

                    let item = GroundItem::new(id.clone(), ItemType::from_u8(item_type), x, y, quantity);
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
                if let Some(value) = data {
                    // Clear current inventory
                    for slot in state.inventory.slots.iter_mut() {
                        *slot = None;
                    }

                    // Update slots
                    if let Some(slots) = extract_array(value, "slots") {
                        for slot_value in slots {
                            let slot_idx = extract_u8(slot_value, "slot").unwrap_or(0) as usize;
                            let item_type = extract_u8(slot_value, "item_type").unwrap_or(0);
                            let quantity = extract_i32(slot_value, "quantity").unwrap_or(0);

                            if slot_idx < state.inventory.slots.len() {
                                state.inventory.slots[slot_idx] = Some(InventorySlot {
                                    item_type: ItemType::from_u8(item_type),
                                    quantity,
                                });
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
                if let Some(value) = data {
                    let slot = extract_u8(value, "slot").unwrap_or(0);
                    let item_type = extract_u8(value, "item_type").unwrap_or(0);
                    let effect = extract_string(value, "effect").unwrap_or_default();
                    log::debug!("Item used: slot {} type {} effect {}", slot, item_type, effect);
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
