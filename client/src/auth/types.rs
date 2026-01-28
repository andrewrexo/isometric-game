use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum AuthError {
    NetworkError(String),
    InvalidCredentials,
    UsernameTaken,
    CharacterNameTaken,
    CharacterLimitReached,
    Unauthorized,
    ServerError(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::NetworkError(e) => write!(f, "Network error: {}", e),
            AuthError::InvalidCredentials => write!(f, "Invalid username or password"),
            AuthError::UsernameTaken => write!(f, "Username already taken"),
            AuthError::CharacterNameTaken => write!(f, "Character name already taken"),
            AuthError::CharacterLimitReached => write!(f, "Character limit reached (max 3)"),
            AuthError::Unauthorized => write!(f, "Not logged in"),
            AuthError::ServerError(e) => write!(f, "Server error: {}", e),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub token: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub id: i64,
    pub name: String,
    pub level: i32,
    pub gender: String,
    pub skin: String,
    #[serde(rename = "hairStyle")]
    pub hair_style: Option<i32>,
    #[serde(rename = "hairColor")]
    pub hair_color: Option<i32>,
    #[serde(rename = "playedTime")]
    pub played_time: i64,
}

#[derive(Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub token: Option<String>,
    pub username: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct CharacterListResponse {
    pub success: bool,
    pub characters: Option<Vec<CharacterInfo>>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct CharacterCreateResponse {
    pub success: bool,
    pub character: Option<CharacterInfo>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct MatchmakeResponse {
    pub room: Option<RoomInfo>,
    #[serde(rename = "sessionToken")]
    pub session_token: Option<String>,
}

#[derive(Deserialize)]
pub struct RoomInfo {
    #[serde(rename = "roomId")]
    pub room_id: String,
}
