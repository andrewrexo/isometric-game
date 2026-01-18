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
    #[serde(rename = "playedTime")]
    pub played_time: i64,
}

#[derive(Deserialize)]
struct AuthResponse {
    success: bool,
    token: Option<String>,
    username: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct CharacterListResponse {
    success: bool,
    characters: Option<Vec<CharacterInfo>>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct CharacterCreateResponse {
    success: bool,
    character: Option<CharacterInfo>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct MatchmakeResponse {
    room: Option<RoomInfo>,
    #[serde(rename = "sessionToken")]
    session_token: Option<String>,
}

#[derive(Deserialize)]
struct RoomInfo {
    #[serde(rename = "roomId")]
    room_id: String,
}

pub struct AuthClient {
    base_url: String,
}

impl AuthClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    /// Register a new account
    pub fn register(&self, username: &str, password: &str) -> Result<AuthSession, AuthError> {
        let url = format!("{}/api/register", self.base_url);

        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(ureq::json!({
                "username": username,
                "password": password
            }))
            .map_err(|e| {
                let error_str = e.to_string();
                if error_str.contains("already exists") {
                    return AuthError::UsernameTaken;
                }
                AuthError::NetworkError(error_str)
            })?;

        let auth_resp: AuthResponse = response
            .into_json()
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if auth_resp.success {
            Ok(AuthSession {
                token: auth_resp.token.unwrap_or_default(),
                username: auth_resp.username.unwrap_or_default(),
            })
        } else {
            let error = auth_resp.error.unwrap_or_else(|| "Unknown error".to_string());
            if error.contains("already exists") {
                Err(AuthError::UsernameTaken)
            } else {
                Err(AuthError::ServerError(error))
            }
        }
    }

    /// Login to an existing account
    pub fn login(&self, username: &str, password: &str) -> Result<AuthSession, AuthError> {
        let url = format!("{}/api/login", self.base_url);

        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(ureq::json!({
                "username": username,
                "password": password
            }))
            .map_err(|e| {
                if let ureq::Error::Status(401, _) = &e {
                    return AuthError::InvalidCredentials;
                }
                AuthError::NetworkError(e.to_string())
            })?;

        let auth_resp: AuthResponse = response
            .into_json()
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if auth_resp.success {
            Ok(AuthSession {
                token: auth_resp.token.unwrap_or_default(),
                username: auth_resp.username.unwrap_or_default(),
            })
        } else {
            let error = auth_resp.error.unwrap_or_else(|| "Unknown error".to_string());
            if error.contains("Invalid") {
                Err(AuthError::InvalidCredentials)
            } else {
                Err(AuthError::ServerError(error))
            }
        }
    }

    /// Logout (invalidate token)
    pub fn logout(&self, token: &str) -> Result<(), AuthError> {
        let url = format!("{}/api/logout", self.base_url);

        ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Content-Type", "application/json")
            .send_json(ureq::json!({}))
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        Ok(())
    }

    /// Get list of characters for the logged in account
    pub fn get_characters(&self, token: &str) -> Result<Vec<CharacterInfo>, AuthError> {
        let url = format!("{}/api/characters", self.base_url);

        let response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .call()
            .map_err(|e| {
                if let ureq::Error::Status(401, _) = &e {
                    return AuthError::Unauthorized;
                }
                AuthError::NetworkError(e.to_string())
            })?;

        let resp: CharacterListResponse = response
            .into_json()
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if resp.success {
            Ok(resp.characters.unwrap_or_default())
        } else {
            Err(AuthError::ServerError(resp.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }

    /// Create a new character
    pub fn create_character(&self, token: &str, name: &str, gender: &str, skin: &str) -> Result<CharacterInfo, AuthError> {
        let url = format!("{}/api/characters", self.base_url);

        let response = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Content-Type", "application/json")
            .send_json(ureq::json!({
                "name": name,
                "gender": gender,
                "skin": skin
            }))
            .map_err(|e| {
                let error_str = e.to_string();
                if error_str.contains("already exists") {
                    return AuthError::CharacterNameTaken;
                }
                if error_str.contains("limit") {
                    return AuthError::CharacterLimitReached;
                }
                if error_str.contains("401") || error_str.contains("Unauthorized") {
                    return AuthError::Unauthorized;
                }
                AuthError::NetworkError(error_str)
            })?;

        let resp: CharacterCreateResponse = response
            .into_json()
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if resp.success {
            resp.character.ok_or_else(|| AuthError::ServerError("No character returned".to_string()))
        } else {
            let error = resp.error.unwrap_or_else(|| "Unknown error".to_string());
            if error.contains("already exists") {
                Err(AuthError::CharacterNameTaken)
            } else if error.contains("limit") {
                Err(AuthError::CharacterLimitReached)
            } else {
                Err(AuthError::ServerError(error))
            }
        }
    }

    /// Delete a character
    pub fn delete_character(&self, token: &str, character_id: i64) -> Result<(), AuthError> {
        let url = format!("{}/api/characters/{}", self.base_url, character_id);

        ureq::delete(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .call()
            .map_err(|e| {
                if let ureq::Error::Status(401, _) = &e {
                    return AuthError::Unauthorized;
                }
                AuthError::NetworkError(e.to_string())
            })?;

        Ok(())
    }

    /// Request matchmaking with a specific character
    pub fn matchmake(&self, token: &str, character_id: i64, room_type: &str) -> Result<(String, String), AuthError> {
        let url = format!("{}/matchmake/joinOrCreate/{}", self.base_url, room_type);

        let response = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Content-Type", "application/json")
            .send_json(ureq::json!({
                "characterId": character_id
            }))
            .map_err(|e| {
                if let ureq::Error::Status(401, _) = &e {
                    return AuthError::Unauthorized;
                }
                AuthError::NetworkError(e.to_string())
            })?;

        let resp: MatchmakeResponse = response
            .into_json()
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        let room = resp.room.ok_or_else(|| AuthError::ServerError("No room returned".to_string()))?;
        let session_token = resp.session_token.ok_or_else(|| AuthError::ServerError("No session returned".to_string()))?;

        Ok((room.room_id, session_token))
    }
}
