use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use dashmap::DashMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{broadcast, mpsc, RwLock};
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};
use uuid::Uuid;

mod chunk;
mod crafting;
mod data;
mod db;
mod entity;
mod game;
mod item;
mod npc;
mod protocol;
mod quest;
mod tilemap;
mod world;

use crafting::CraftingRegistry;
use data::ItemRegistry;
use db::Database;
use entity::EntityRegistry;
use quest::QuestRegistry;
use game::{GameRoom, Player, PlayerUpdate};
use protocol::{ClientMessage, ServerMessage};

// ============================================================================
// App State
// ============================================================================

/// Game session data for a connected player
#[derive(Clone)]
struct GameSession {
    room_id: String,
    player_id: String,
    username: String,      // For DB persistence
    db_player_id: i64,     // Database player ID (from auth)
    auth_token: String,    // Token used for this session (for validation)
}

#[derive(Clone)]
struct AppState {
    rooms: Arc<DashMap<String, Arc<GameRoom>>>,
    // Session ID -> GameSession
    sessions: Arc<DashMap<String, GameSession>>,
    // Auth token -> (Username, Player DB ID)
    auth_sessions: AuthSessions,
    db: Arc<Database>,
    auth_rate_limiter: RateLimiter,
    matchmake_rate_limiter: RateLimiter,
    // SECURITY: Signed session token generator
    token_signer: SessionTokenSigner,
    // Entity and item registries (loaded from TOML at startup)
    entity_registry: Arc<EntityRegistry>,
    item_registry: Arc<ItemRegistry>,
    quest_registry: Arc<QuestRegistry>,
    crafting_registry: Arc<CraftingRegistry>,
}

impl AppState {
    async fn new() -> Self {
        // Initialize database
        let db = Database::new("sqlite:game.db?mode=rwc")
            .await
            .expect("Failed to initialize database");

        // Load entity registry from TOML files
        let mut entity_registry = EntityRegistry::new();
        let data_dir = std::path::Path::new("data");
        if let Err(e) = entity_registry.load_from_directory(data_dir) {
            error!("Failed to load entity registry: {}", e);
        }

        // Load item registry from TOML files
        let mut item_registry = ItemRegistry::new();
        if let Err(e) = item_registry.load_from_directory(data_dir) {
            error!("Failed to load item registry: {}", e);
        }

        // Load quest registry from TOML files
        let quest_registry = Arc::new(QuestRegistry::new(data_dir));
        if let Err(e) = quest_registry.load_all().await {
            error!("Failed to load quest registry: {}", e);
        }

        // Load crafting registry from TOML files
        let mut crafting_registry = CraftingRegistry::new();
        if let Err(e) = crafting_registry.load_from_directory(data_dir) {
            error!("Failed to load crafting registry: {}", e);
        }

        // Start hot-reload watcher for quest files (dev mode)
        #[cfg(debug_assertions)]
        {
            match quest_registry.start_file_watcher() {
                Ok(mut rx) => {
                    // Spawn task to log reload events
                    tokio::spawn(async move {
                        while let Some(event) = rx.recv().await {
                            match event {
                                quest::HotReloadEvent::Reloaded(path) => {
                                    info!("Quest hot-reload: {}", path);
                                }
                                quest::HotReloadEvent::Error(e) => {
                                    error!("Quest hot-reload error: {}", e);
                                }
                            }
                        }
                    });
                    info!("Quest hot-reload enabled");
                }
                Err(e) => {
                    warn!("Failed to start quest hot-reload: {}", e);
                }
            }
        }

        Self {
            rooms: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            auth_sessions: Arc::new(DashMap::new()),
            db: Arc::new(db),
            // Auth: 10 attempts per 60 seconds per IP
            auth_rate_limiter: RateLimiter::new(10, 60),
            // Matchmaking: 20 attempts per 60 seconds per IP
            matchmake_rate_limiter: RateLimiter::new(20, 60),
            // SECURITY: Token signer for session tokens
            token_signer: SessionTokenSigner::new(),
            entity_registry: Arc::new(entity_registry),
            item_registry: Arc::new(item_registry),
            quest_registry,
            crafting_registry: Arc::new(crafting_registry),
        }
    }

    async fn get_or_create_room(&self, room_name: &str) -> Arc<GameRoom> {
        // Check if a room with this name already exists
        for room in self.rooms.iter() {
            if room.name == room_name {
                return room.clone();
            }
        }

        // Create new room and store by its UUID
        let room = Arc::new(GameRoom::new(
            room_name,
            self.entity_registry.clone(),
            self.quest_registry.clone(),
            self.crafting_registry.clone(),
            self.item_registry.clone(),
        ).await);
        self.rooms.insert(room.id.clone(), room.clone());
        room
    }
}

// ============================================================================
// HTTP Handlers - Authentication
// ============================================================================

/// Auth sessions: token -> (username, player_id)
type AuthSessions = Arc<DashMap<String, (String, i64)>>;

/// Rate limiter entry: (request_count, window_start_time)
type RateLimitEntry = (u32, std::time::Instant);

// ============================================================================
// Signed Session Tokens (Security Hardening)
// ============================================================================

type HmacSha256 = Hmac<Sha256>;

/// Session token validity duration
const SESSION_TOKEN_EXPIRY_SECS: u64 = 300; // 5 minutes

/// Signed session token generator/validator
#[derive(Clone)]
struct SessionTokenSigner {
    /// Secret key for HMAC signing (generated at startup)
    secret: Vec<u8>,
}

impl SessionTokenSigner {
    fn new() -> Self {
        // Generate a random 32-byte secret at startup
        use rand::RngCore;
        let mut secret = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret);
        Self { secret }
    }

    /// Create a signed session token
    /// Format: base64(session_id:room_id:expiry_ts:signature)
    fn create_token(&self, session_id: &str, room_id: &str) -> String {
        use base64::Engine;

        let expiry = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() + SESSION_TOKEN_EXPIRY_SECS;

        let payload = format!("{}:{}:{}", session_id, room_id, expiry);

        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());
        let signature = mac.finalize().into_bytes();

        let token_data = format!("{}:{}", payload, base64::engine::general_purpose::STANDARD.encode(signature));
        base64::engine::general_purpose::URL_SAFE.encode(token_data)
    }

    /// Validate a signed session token
    /// Returns Some((session_id, room_id)) if valid, None if invalid/expired
    fn validate_token(&self, token: &str) -> Option<(String, String)> {
        use base64::Engine;

        // Decode base64
        let token_data = base64::engine::general_purpose::URL_SAFE.decode(token).ok()?;
        let token_str = String::from_utf8(token_data).ok()?;

        // Parse: session_id:room_id:expiry:signature
        let parts: Vec<&str> = token_str.splitn(4, ':').collect();
        if parts.len() != 4 {
            return None;
        }

        let session_id = parts[0];
        let room_id = parts[1];
        let expiry: u64 = parts[2].parse().ok()?;
        let signature_b64 = parts[3];

        // Check expiry
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now > expiry {
            warn!("Session token expired: {} > {}", now, expiry);
            return None;
        }

        // Verify signature
        let payload = format!("{}:{}:{}", session_id, room_id, expiry);
        let expected_sig = base64::engine::general_purpose::STANDARD.decode(signature_b64).ok()?;

        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());

        if mac.verify_slice(&expected_sig).is_err() {
            warn!("Session token signature invalid");
            return None;
        }

        Some((session_id.to_string(), room_id.to_string()))
    }
}

/// Simple IP-based rate limiter
#[derive(Clone)]
struct RateLimiter {
    /// IP -> (request_count, window_start)
    entries: Arc<DashMap<String, RateLimitEntry>>,
    /// Max requests per window
    max_requests: u32,
    /// Window duration
    window_duration: Duration,
}

impl RateLimiter {
    fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            max_requests,
            window_duration: Duration::from_secs(window_secs),
        }
    }

    /// Check if request is allowed. Returns true if allowed, false if rate limited.
    fn check(&self, ip: &str) -> bool {
        let now = std::time::Instant::now();

        let mut entry = self.entries.entry(ip.to_string()).or_insert((0, now));
        let (count, window_start) = entry.value_mut();

        // Reset window if expired
        if now.duration_since(*window_start) > self.window_duration {
            *count = 0;
            *window_start = now;
        }

        // Check limit
        if *count >= self.max_requests {
            return false;
        }

        *count += 1;
        true
    }

    /// Record a failed login attempt (for stricter limiting on failures)
    fn record_failure(&self, ip: &str) {
        let now = std::time::Instant::now();
        let mut entry = self.entries.entry(ip.to_string()).or_insert((0, now));
        let (count, _) = entry.value_mut();
        // Add extra penalty for failures
        *count = (*count).saturating_add(2);
    }
}

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    success: bool,
    token: Option<String>,
    username: Option<String>,
    error: Option<String>,
}

async fn register_account(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let client_ip = addr.ip().to_string();

    if !state.auth_rate_limiter.check(&client_ip) {
        warn!("Rate limit exceeded for registration from {}", client_ip);
        return Json(AuthResponse {
            success: false,
            token: None,
            username: None,
            error: Some("Too many requests. Please try again later.".to_string()),
        });
    }

    // Validate input
    if req.username.len() < 3 {
        return Json(AuthResponse {
            success: false,
            token: None,
            username: None,
            error: Some("Username must be at least 3 characters".to_string()),
        });
    }
    if req.password.len() < 6 {
        return Json(AuthResponse {
            success: false,
            token: None,
            username: None,
            error: Some("Password must be at least 6 characters".to_string()),
        });
    }

    match state.db.create_player(&req.username, &req.password).await {
        Ok(player_id) => {
            // Create auth token
            let token = Uuid::new_v4().to_string();
            state.auth_sessions.insert(token.clone(), (req.username.clone(), player_id));

            info!("Account registered: {} (id: {}) from {}", req.username, player_id, client_ip);

            Json(AuthResponse {
                success: true,
                token: Some(token),
                username: Some(req.username),
                error: None,
            })
        }
        Err(e) => {
            Json(AuthResponse {
                success: false,
                token: None,
                username: None,
                error: Some(e),
            })
        }
    }
}

async fn login_account(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let client_ip = addr.ip().to_string();

    if !state.auth_rate_limiter.check(&client_ip) {
        warn!("Rate limit exceeded for login from {}", client_ip);
        return Json(AuthResponse {
            success: false,
            token: None,
            username: None,
            error: Some("Too many login attempts. Please try again later.".to_string()),
        });
    }

    match state.db.verify_password(&req.username, &req.password).await {
        Some(player) => {
            // Create auth token
            let token = Uuid::new_v4().to_string();
            state.auth_sessions.insert(token.clone(), (req.username.clone(), player.id));

            // Update last login (save existing data with updated timestamp)
            let _ = state.db.save_player(
                &req.username,
                player.x,
                player.y,
                player.hp,
                player.max_hp,
                player.level,
                player.exp,
                player.exp_to_next_level,
                player.gold,
                &player.inventory_json,
                player.equipped_head.as_deref(),
                player.equipped_body.as_deref(),
                player.equipped_weapon.as_deref(),
                player.equipped_back.as_deref(),
                player.equipped_feet.as_deref(),
                player.equipped_ring.as_deref(),
                player.equipped_gloves.as_deref(),
                player.equipped_necklace.as_deref(),
                player.equipped_belt.as_deref(),
            ).await;

            info!("Player logged in: {} from {}", req.username, client_ip);

            Json(AuthResponse {
                success: true,
                token: Some(token),
                username: Some(req.username),
                error: None,
            })
        }
        None => {
            state.auth_rate_limiter.record_failure(&client_ip);
            warn!("Failed login attempt for '{}' from {}", req.username, client_ip);

            Json(AuthResponse {
                success: false,
                token: None,
                username: None,
                error: Some("Invalid username or password".to_string()),
            })
        }
    }
}

async fn logout_account(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if let Some(auth) = headers.get("Authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                state.auth_sessions.remove(token);
            }
        }
    }
    Json(serde_json::json!({ "success": true }))
}

// ============================================================================
// HTTP Handlers - Matchmaking
// ============================================================================

#[derive(Deserialize)]
struct JoinOptions {
    name: Option<String>,
}

#[derive(Serialize)]
struct MatchmakeResponse {
    room: RoomInfo,
    /// Signed session token for WebSocket upgrade (expires in 5 minutes)
    #[serde(rename = "sessionToken")]
    session_token: String,
}

#[derive(Serialize)]
struct RoomInfo {
    #[serde(rename = "roomId")]
    room_id: String,
    name: String,
    clients: usize,
}

async fn matchmake_join_or_create(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(room_name): Path<String>,
    headers: axum::http::HeaderMap,
    Json(_options): Json<JoinOptions>,
) -> impl IntoResponse {
    let client_ip = addr.ip().to_string();

    if !state.matchmake_rate_limiter.check(&client_ip) {
        warn!("Rate limit exceeded for matchmaking from {}", client_ip);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": "Too many requests. Please try again later." }))
        ).into_response();
    }

    let auth_token = match headers.get("Authorization") {
        Some(auth_header) => {
            match auth_header.to_str() {
                Ok(auth_str) => {
                    match auth_str.strip_prefix("Bearer ") {
                        Some(token) => token.to_string(),
                        None => {
                            warn!("Matchmaking rejected: Invalid Authorization format");
                            return (
                                StatusCode::UNAUTHORIZED,
                                Json(serde_json::json!({
                                    "error": "Invalid authorization format. Use 'Bearer <token>'"
                                }))
                            ).into_response();
                        }
                    }
                }
                Err(_) => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({ "error": "Invalid authorization header" }))
                    ).into_response();
                }
            }
        }
        None => {
            warn!("Matchmaking rejected: No Authorization header");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Authorization required. Please login first." }))
            ).into_response();
        }
    };

    // Validate token and get authenticated user info
    let (username, db_player_id) = match state.auth_sessions.get(&auth_token) {
        Some(auth_data) => auth_data.clone(),
        None => {
            warn!("Matchmaking rejected: Invalid or expired token");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Invalid or expired token. Please login again." }))
            ).into_response();
        }
    };

    let room = state.get_or_create_room(&room_name).await;
    let room_id = room.id.clone();

    // Create session for this player - use a separate session token from player_id
    let session_id = Uuid::new_v4().to_string();
    let player_id = format!("player_{}", db_player_id);

    // Try to load existing player data from database using authenticated username
    let player_data = state.db.get_player_by_username(&username).await.ok().flatten();

    // Reserve the session with authenticated user info
    state.sessions.insert(
        session_id.clone(),
        GameSession {
            room_id: room_id.clone(),
            player_id: player_id.clone(),
            username: username.clone(),
            db_player_id,
            auth_token: auth_token.clone(),
        },
    );

    // Pre-create player with saved data or defaults
    if let Some(data) = player_data {
        // Load saved player
        info!("Loading saved player: {} (db_id: {}) at ({}, {}) as {} {}",
            username, db_player_id, data.x, data.y, data.gender, data.skin);
        room.reserve_player_with_data(
            &player_id,
            &username,
            data.x as i32,
            data.y as i32,
            data.hp,
            data.max_hp,
            data.level,
            data.exp,
            data.exp_to_next_level,
            data.gold,
            &data.inventory_json,
            &data.gender,
            &data.skin,
            data.equipped_head.clone(),
            data.equipped_body.clone(),
            data.equipped_weapon.clone(),
            data.equipped_back.clone(),
            data.equipped_feet.clone(),
            data.equipped_ring.clone(),
            data.equipped_gloves.clone(),
            data.equipped_necklace.clone(),
            data.equipped_belt.clone(),
            data.is_admin,
        ).await;
    } else {
        // New player with defaults (shouldn't happen - auth creates player)
        room.reserve_player(&player_id, &username, "male", "tan").await;
    }

    // Load quest state from database
    match state.db.load_quest_state(db_player_id).await {
        Ok(quest_state) => {
            let active_count = quest_state.active_quests.len();
            let completed_count = quest_state.completed_quests.len();
            room.set_player_quest_state(&player_id, quest_state).await;
            if active_count > 0 || completed_count > 0 {
                info!("Loaded quest state for {}: {} active, {} completed", username, active_count, completed_count);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load quest state for {}: {}", username, e);
            // Continue with empty quest state (default)
        }
    }

    let client_count = room.player_count().await;

    // Generate signed session token for WebSocket upgrade
    let session_token = state.token_signer.create_token(&session_id, &room_id);

    info!(
        "Matchmaking: room={}, player={} (db_id: {})",
        room_id, username, db_player_id
    );

    Json(MatchmakeResponse {
        room: RoomInfo {
            room_id,
            name: room_name,
            clients: client_count,
        },
        session_token,
    }).into_response()
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().timestamp_millis()
    }))
}

// ============================================================================
// WebSocket Handler
// ============================================================================

#[derive(Deserialize)]
struct WsQuery {
    /// Signed session token
    #[serde(rename = "sessionToken")]
    session_token: String,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Validate signed session token
    let session_id = match state.token_signer.validate_token(&query.session_token) {
        Some((sid, rid)) => {
            if rid != room_id {
                warn!("WebSocket rejected: Token room_id mismatch ({} != {})", rid, room_id);
                return (StatusCode::FORBIDDEN, "Invalid session token: room mismatch").into_response();
            }
            sid
        }
        None => {
            warn!("WebSocket rejected: Invalid or expired session token");
            return (StatusCode::UNAUTHORIZED, "Invalid or expired session token").into_response();
        }
    };

    // Validate session exists in our store
    let session_data = state.sessions.get(&session_id).map(|s| s.clone());

    match session_data {
        Some(session) if session.room_id == room_id => {
            // Verify the auth token is still valid
            if !state.auth_sessions.contains_key(&session.auth_token) {
                warn!("WebSocket rejected: Auth token expired for session {}", session_id);
                return (StatusCode::UNAUTHORIZED, "Auth token expired. Please login again.").into_response();
            }

            // Valid session, upgrade to WebSocket
            let player_id = session.player_id.clone();
            let username = session.username.clone();
            let db_player_id = session.db_player_id;
            ws.on_upgrade(move |socket| {
                handle_socket(socket, state, room_id, player_id, session_id, username, db_player_id)
            })
        }
        _ => {
            warn!("Invalid session: {} for room {}", session_id, room_id);
            (StatusCode::FORBIDDEN, "Invalid session").into_response()
        }
    }
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    room_id: String,
    player_id: String,
    session_id: String,
    username: String,
    _db_player_id: i64,  // Used for future persistence binding
) {
    let (mut sender, mut receiver) = socket.split();

    // Get the room
    let room = match state.rooms.get(&room_id) {
        Some(r) => r.clone(),
        None => {
            error!("Room not found: {}", room_id);
            return;
        }
    };

    // Activate the player
    let player_name = room.activate_player(&player_id).await;
    info!("Player {} ({}) connected to room {}", player_name, player_id, room_id);

    // Subscribe to room broadcasts
    let mut broadcast_rx = room.subscribe();

    // Send welcome message
    let welcome = ServerMessage::Welcome {
        player_id: player_id.clone(),
    };
    if let Ok(bytes) = protocol::encode_server_message(&welcome) {
        let _ = sender.send(Message::Binary(bytes)).await;
    }

    // Send entity definitions
    let entity_defs = room.get_entity_definitions();
    if let Ok(bytes) = protocol::encode_server_message(&entity_defs) {
        let _ = sender.send(Message::Binary(bytes)).await;
    }

    // Send item definitions
    let item_defs = state.item_registry.to_client_definitions();
    if let Ok(bytes) = protocol::encode_server_message(&item_defs) {
        let _ = sender.send(Message::Binary(bytes)).await;
    }

    // Send recipe definitions
    let recipe_defs = state.crafting_registry.to_client_definitions();
    if let Ok(bytes) = protocol::encode_server_message(&recipe_defs) {
        let _ = sender.send(Message::Binary(bytes)).await;
    }

    // Get player's position and send nearby chunks
    if let Some((px, py)) = room.get_player_position(&player_id).await {
        let player_chunk = chunk::ChunkCoord::from_world(px, py);

        // Preload and send chunks in a 3x3 area around the player
        for dy in -1..=1 {
            for dx in -1..=1 {
                let coord = chunk::ChunkCoord::new(player_chunk.x + dx, player_chunk.y + dy);
                if let Some(chunk_msg) = room.handle_chunk_request(coord.x, coord.y).await {
                    if let Ok(bytes) = protocol::encode_server_message(&chunk_msg) {
                        let _ = sender.send(Message::Binary(bytes)).await;
                    }
                }
            }
        }
    }

    // Send existing players to this client
    for existing_player in room.get_all_players().await {
        if existing_player.id != player_id {
            let msg = ServerMessage::PlayerJoined {
                id: existing_player.id.clone(),
                name: existing_player.name.clone(),
                x: existing_player.x,
                y: existing_player.y,
                gender: existing_player.gender.clone(),
                skin: existing_player.skin.clone(),
            };
            if let Ok(bytes) = protocol::encode_server_message(&msg) {
                let _ = sender.send(Message::Binary(bytes)).await;
            }
        }
    }

    // Send active quests to this client (from saved state)
    for quest_msg in room.get_active_quest_messages(&player_id).await {
        if let Ok(bytes) = protocol::encode_server_message(&quest_msg) {
            let _ = sender.send(Message::Binary(bytes)).await;
        }
    }

    // Send initial inventory to this client
    if let Some(inv_msg) = room.get_player_inventory_update(&player_id).await {
        if let Ok(bytes) = protocol::encode_server_message(&inv_msg) {
            let _ = sender.send(Message::Binary(bytes)).await;
        }
    }

    // Notify others about this player
    let (x, y) = room.get_player_position(&player_id).await.unwrap_or((0, 0));
    let (gender, skin) = room.get_player_appearance(&player_id).await.unwrap_or_else(|| ("male".to_string(), "tan".to_string()));
    room.broadcast(ServerMessage::PlayerJoined {
        id: player_id.clone(),
        name: player_name.clone(),
        x,
        y,
        gender,
        skin,
    })
    .await;

    // Create channel for sending messages to this client
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(32);

    // SECURITY: Register this player's sender for unicast messages
    room.register_player_sender(&player_id, tx).await;

    // Spawn task to forward messages to WebSocket
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle direct messages to this client
                Some(msg) = rx.recv() => {
                    if sender.send(Message::Binary(msg)).await.is_err() {
                        break;
                    }
                }
                // Handle broadcast messages
                Ok(msg) = broadcast_rx.recv() => {
                    if let Ok(bytes) = protocol::encode_server_message(&msg) {
                        if sender.send(Message::Binary(bytes)).await.is_err() {
                            break;
                        }
                    }
                }
                else => break,
            }
        }
    });

    // Handle incoming messages
    let room_clone = room.clone();
    let player_id_clone = player_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Binary(data) => {
                    if let Err(e) = handle_client_message(&room_clone, &player_id_clone, &data).await
                    {
                        warn!("Error handling message: {}", e);
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    // Cleanup - save player data before removing
    info!("Player {} disconnected from room {}", player_id, room_id);

    let should_save = state.sessions.get(&session_id)
        .map(|s| state.auth_sessions.contains_key(&s.auth_token))
        .unwrap_or(false);

    if should_save {
        // Get db_player_id from session
        let db_player_id = state.sessions.get(&session_id)
            .map(|s| s.db_player_id)
            .unwrap_or(0);

        // Save player state to database
        if let Some(save_data) = room.get_player_save_data(&player_id).await {
            if let Err(e) = state.db.save_player(
                &username,
                save_data.x,
                save_data.y,
                save_data.hp,
                save_data.max_hp,
                save_data.level,
                save_data.exp,
                save_data.exp_to_next_level,
                save_data.gold,
                &save_data.inventory_json,
                save_data.equipped_head.as_deref(),
                save_data.equipped_body.as_deref(),
                save_data.equipped_weapon.as_deref(),
                save_data.equipped_back.as_deref(),
                save_data.equipped_feet.as_deref(),
                save_data.equipped_ring.as_deref(),
                save_data.equipped_gloves.as_deref(),
                save_data.equipped_necklace.as_deref(),
                save_data.equipped_belt.as_deref(),
            ).await {
                error!("Failed to save player {} on disconnect: {}", username, e);
            } else {
                info!("Saved player {} to database on disconnect", username);
            }
        }

        // Save quest state to database
        if db_player_id > 0 {
            if let Some(quest_state) = room.get_player_quest_state(&player_id).await {
                if let Err(e) = state.db.save_quest_state(db_player_id, &quest_state).await {
                    error!("Failed to save quest state for {} on disconnect: {}", username, e);
                } else if !quest_state.active_quests.is_empty() || !quest_state.completed_quests.is_empty() {
                    info!("Saved quest state for {}: {} active, {} completed",
                        username, quest_state.active_quests.len(), quest_state.completed_quests.len());
                }
            }
        }
    } else {
        warn!("Skipping save for {} on disconnect: invalid auth", username);
    }

    // SECURITY: Unregister player sender before cleanup
    room.unregister_player_sender(&player_id).await;

    state.sessions.remove(&session_id);
    room.remove_player(&player_id).await;

    // Notify others
    room.broadcast(ServerMessage::PlayerLeft {
        id: player_id.clone(),
    })
    .await;
}

async fn handle_client_message(
    room: &GameRoom,
    player_id: &str,
    data: &[u8],
) -> Result<(), String> {
    let msg = protocol::decode_client_message(data)?;

    match msg {
        ClientMessage::Move { dx, dy } => {
            room.handle_move(player_id, dx, dy).await;
        }
        ClientMessage::Face { direction } => {
            room.handle_face(player_id, direction).await;
        }
        ClientMessage::Chat { text } => {
            room.handle_chat(player_id, &text).await;
        }
        ClientMessage::Attack => {
            room.handle_attack(player_id).await;
        }
        ClientMessage::Target { entity_id } => {
            room.handle_target(player_id, &entity_id).await;
        }
        ClientMessage::Pickup { item_id } => {
            room.handle_pickup(player_id, &item_id).await;
        }
        ClientMessage::UseItem { slot_index } => {
            room.handle_use_item(player_id, slot_index).await;
        }
        ClientMessage::RequestChunk { chunk_x, chunk_y } => {
            // Chunk data is sent back via the broadcast channel for now
            // In a production system, you'd send directly to requesting client
            if let Some(chunk_msg) = room.handle_chunk_request(chunk_x, chunk_y).await {
                room.broadcast(chunk_msg).await;
            }
        }
        ClientMessage::Interact { npc_id } => {
            room.handle_npc_interact(player_id, &npc_id).await;
        }
        ClientMessage::DialogueChoiceMsg { quest_id, choice_id } => {
            room.handle_dialogue_choice(player_id, &quest_id, &choice_id).await;
        }
        ClientMessage::AcceptQuest { quest_id: _ } => {
            // Quest acceptance is handled through dialogue choices
            // This is a fallback if client sends direct accept
        }
        ClientMessage::AbandonQuest { quest_id: _ } => {
            // TODO: Implement quest abandonment
        }
        ClientMessage::Craft { recipe_id } => {
            room.handle_craft(player_id, &recipe_id).await;
        }
        ClientMessage::Equip { slot_index } => {
            room.handle_equip(player_id, slot_index).await;
        }
        ClientMessage::Unequip { slot_type } => {
            room.handle_unequip(player_id, &slot_type).await;
        }
        ClientMessage::DropItem { slot_index, quantity } => {
            room.handle_drop_item(player_id, slot_index, quantity).await;
        }
        ClientMessage::SwapSlots { from_slot, to_slot } => {
            room.handle_swap_slots(player_id, from_slot, to_slot).await;
        }
        // Auth and Register are handled via HTTP endpoints, not WebSocket
        ClientMessage::Auth { .. } | ClientMessage::Register { .. } => {}
    }

    Ok(())
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("isometric_server=info".parse().unwrap()),
        )
        .init();

    let state = AppState::new().await;

    // Spawn game tick loop
    let tick_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(50)); // 20 Hz
        loop {
            interval.tick().await;
            for room in tick_state.rooms.iter() {
                room.tick().await;
            }
        }
    });

    // Spawn auto-save loop (every 30 seconds)
    let save_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            let mut saved_count = 0;
            // Iterate through all active sessions and save their players
            for session in save_state.sessions.iter() {
                let session_data = session.value().clone();
                let room_id = &session_data.room_id;
                let player_id = &session_data.player_id;
                let username = &session_data.username;
                let auth_token = &session_data.auth_token;

                if !save_state.auth_sessions.contains_key(auth_token) {
                    warn!("Auto-save skipped for {}: auth token no longer valid", username);
                    continue;
                }

                // Get the room and player save data
                if let Some(room) = save_state.rooms.get(room_id) {
                    if let Some(save_data) = room.get_player_save_data(player_id).await {
                        if let Err(e) = save_state.db.save_player(
                            username,
                            save_data.x,
                            save_data.y,
                            save_data.hp,
                            save_data.max_hp,
                            save_data.level,
                            save_data.exp,
                            save_data.exp_to_next_level,
                            save_data.gold,
                            &save_data.inventory_json,
                            save_data.equipped_head.as_deref(),
                            save_data.equipped_body.as_deref(),
                            save_data.equipped_weapon.as_deref(),
                            save_data.equipped_back.as_deref(),
                            save_data.equipped_feet.as_deref(),
                            save_data.equipped_ring.as_deref(),
                            save_data.equipped_gloves.as_deref(),
                            save_data.equipped_necklace.as_deref(),
                            save_data.equipped_belt.as_deref(),
                        ).await {
                            warn!("Auto-save failed for player {}: {}", username, e);
                        } else {
                            saved_count += 1;
                        }
                    }

                    // Also save quest state
                    let db_player_id = session_data.db_player_id;
                    if let Some(quest_state) = room.get_player_quest_state(player_id).await {
                        let _ = save_state.db.save_quest_state(db_player_id, &quest_state).await;
                    }
                }
            }

            if saved_count > 0 {
                info!("Auto-saved {} player(s) to database", saved_count);
            }
        }
    });

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // Authentication
        .route("/api/register", post(register_account))
        .route("/api/login", post(login_account))
        .route("/api/logout", post(logout_account))
        // Matchmaking
        .route("/matchmake/joinOrCreate/:room", post(matchmake_join_or_create))
        // WebSocket
        .route("/:room_id", get(ws_handler))
        // In development, you may want CorsLayer::permissive()
        // For production, specify allowed origins explicitly
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any) // TODO: Replace with specific origins in production
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION])
        )
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 2567));
    info!("Game server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}
