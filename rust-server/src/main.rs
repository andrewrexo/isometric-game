use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use dashmap::DashMap;
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

mod db;
mod game;
mod item;
mod npc;
mod protocol;
mod tilemap;

use db::Database;
use game::{GameRoom, Player, PlayerUpdate};
use protocol::{ClientMessage, ServerMessage};

// ============================================================================
// App State
// ============================================================================

#[derive(Clone)]
struct AppState {
    rooms: Arc<DashMap<String, Arc<GameRoom>>>,
    // Session ID -> (Room ID, Player ID)
    sessions: Arc<DashMap<String, (String, String)>>,
    // Auth token -> (Username, Player DB ID)
    auth_sessions: AuthSessions,
    db: Arc<Database>,
}

impl AppState {
    async fn new() -> Self {
        // Initialize database
        let db = Database::new("sqlite:game.db?mode=rwc")
            .await
            .expect("Failed to initialize database");

        Self {
            rooms: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            auth_sessions: Arc::new(DashMap::new()),
            db: Arc::new(db),
        }
    }

    fn get_or_create_room(&self, room_name: &str) -> Arc<GameRoom> {
        // Check if a room with this name already exists
        for room in self.rooms.iter() {
            if room.name == room_name {
                return room.clone();
            }
        }

        // Create new room and store by its UUID
        let room = Arc::new(GameRoom::new(room_name));
        self.rooms.insert(room.id.clone(), room.clone());
        room
    }
}

// ============================================================================
// HTTP Handlers - Authentication
// ============================================================================

/// Auth sessions: token -> (username, player_id)
type AuthSessions = Arc<DashMap<String, (String, i64)>>;

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
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
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

            info!("Account registered: {} (id: {})", req.username, player_id);

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
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match state.db.verify_password(&req.username, &req.password).await {
        Some(player) => {
            // Create auth token
            let token = Uuid::new_v4().to_string();
            state.auth_sessions.insert(token.clone(), (req.username.clone(), player.id));

            // Update last login
            let _ = state.db.save_player(
                &req.username,
                player.x,
                player.y,
                player.hp,
                player.level,
                player.exp,
            ).await;

            info!("Player logged in: {}", req.username);

            Json(AuthResponse {
                success: true,
                token: Some(token),
                username: Some(req.username),
                error: None,
            })
        }
        None => {
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
    #[serde(rename = "sessionId")]
    session_id: String,
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
    Path(room_name): Path<String>,
    Json(options): Json<JoinOptions>,
) -> impl IntoResponse {
    let room = state.get_or_create_room(&room_name);
    let room_id = room.id.clone();

    // Create session for this player
    let session_id = Uuid::new_v4().to_string();
    let player_id = session_id.clone();
    let player_name = options
        .name
        .unwrap_or_else(|| format!("Player{}", &session_id[..4]));

    // Reserve the session
    state
        .sessions
        .insert(session_id.clone(), (room_id.clone(), player_id.clone()));

    // Pre-create player (will be activated on WebSocket connect)
    room.reserve_player(&player_id, &player_name).await;

    let client_count = room.player_count().await;

    info!(
        "Matchmaking: session={}, room={}, player={}",
        session_id, room_id, player_name
    );

    Json(MatchmakeResponse {
        room: RoomInfo {
            room_id,
            name: room_name,
            clients: client_count,
        },
        session_id,
    })
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
    #[serde(rename = "sessionId")]
    session_id: String,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let session_id = query.session_id;

    // Validate session
    let session_data = state.sessions.get(&session_id).map(|s| s.clone());

    match session_data {
        Some((expected_room_id, player_id)) if expected_room_id == room_id => {
            // Valid session, upgrade to WebSocket
            ws.on_upgrade(move |socket| {
                handle_socket(socket, state, room_id, player_id, session_id)
            })
        }
        _ => {
            warn!("Invalid session: {} for room {}", session_id, room_id);
            // Return error response
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

    // Send existing players to this client
    for existing_player in room.get_all_players().await {
        if existing_player.id != player_id {
            let msg = ServerMessage::PlayerJoined {
                id: existing_player.id.clone(),
                name: existing_player.name.clone(),
                x: existing_player.x,
                y: existing_player.y,
            };
            if let Ok(bytes) = protocol::encode_server_message(&msg) {
                let _ = sender.send(Message::Binary(bytes)).await;
            }
        }
    }

    // Notify others about this player
    let (x, y) = room.get_player_position(&player_id).await.unwrap_or((0, 0));
    room.broadcast(ServerMessage::PlayerJoined {
        id: player_id.clone(),
        name: player_name.clone(),
        x,
        y,
    })
    .await;

    // Create channel for sending messages to this client
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(32);

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

    // Cleanup
    info!("Player {} disconnected from room {}", player_id, room_id);
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
        _ => {
            // Other messages not yet implemented
        }
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
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 2567));
    info!("Game server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
