use sapp_jsutils::JsObject;

use super::types::*;

extern "C" {
    fn http_request(method: JsObject, url: JsObject, headers: JsObject, body: JsObject) -> i32;
    fn http_poll(request_id: i32) -> i32;
    fn http_response_body(request_id: i32) -> JsObject;
    fn http_response_status(request_id: i32) -> i32;
    fn http_cleanup(request_id: i32);
}

pub enum AuthResult {
    Login(Result<AuthSession, AuthError>),
    Register(Result<AuthSession, AuthError>),
    Characters(Result<Vec<CharacterInfo>, AuthError>),
    CharacterCreated(Result<CharacterInfo, AuthError>),
    CharacterDeleted(Result<(), AuthError>),
    Matchmake(Result<(String, String), AuthError>),
    HealthCheck(bool),
}

enum PendingRequestKind {
    Login,
    Register,
    GetCharacters,
    CreateCharacter,
    DeleteCharacter,
    Matchmake,
    HealthCheck,
}

struct PendingRequest {
    id: i32,
    kind: PendingRequestKind,
}

pub struct AuthClient {
    base_url: String,
    pending_request: Option<PendingRequest>,
}

fn make_headers_json(token: Option<&str>) -> String {
    match token {
        Some(t) => format!(
            r#"{{"Authorization":"Bearer {}","Content-Type":"application/json"}}"#,
            t
        ),
        None => r#"{"Content-Type":"application/json"}"#.to_string(),
    }
}

fn fire_request(method: &str, url: &str, headers_json: &str, body: Option<&str>) -> i32 {
    unsafe {
        let method_js = JsObject::string(method);
        let url_js = JsObject::string(url);
        let headers_js = JsObject::string(headers_json);
        let body_js = match body {
            Some(b) => JsObject::string(b),
            None => JsObject::string(""),
        };
        http_request(method_js, url_js, headers_js, body_js)
    }
}

fn get_response_body(request_id: i32) -> String {
    unsafe {
        let obj = http_response_body(request_id);
        if obj.is_nil() {
            return String::new();
        }
        let mut buf = String::new();
        obj.to_string(&mut buf);
        buf
    }
}

impl AuthClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            pending_request: None,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.pending_request.is_some()
    }

    pub fn start_login(&mut self, username: &str, password: &str) {
        let url = format!("{}/api/login", self.base_url);
        let body = format!(r#"{{"username":"{}","password":"{}"}}"#, username, password);
        let headers = make_headers_json(None);
        let id = fire_request("POST", &url, &headers, Some(&body));
        self.pending_request = Some(PendingRequest {
            id,
            kind: PendingRequestKind::Login,
        });
    }

    pub fn start_register(&mut self, username: &str, password: &str) {
        let url = format!("{}/api/register", self.base_url);
        let body = format!(r#"{{"username":"{}","password":"{}"}}"#, username, password);
        let headers = make_headers_json(None);
        let id = fire_request("POST", &url, &headers, Some(&body));
        self.pending_request = Some(PendingRequest {
            id,
            kind: PendingRequestKind::Register,
        });
    }

    pub fn start_get_characters(&mut self, token: &str) {
        let url = format!("{}/api/characters", self.base_url);
        let headers = make_headers_json(Some(token));
        let id = fire_request("GET", &url, &headers, None);
        self.pending_request = Some(PendingRequest {
            id,
            kind: PendingRequestKind::GetCharacters,
        });
    }

    pub fn start_create_character(
        &mut self,
        token: &str,
        name: &str,
        gender: &str,
        skin: &str,
        hair_style: Option<i32>,
        hair_color: Option<i32>,
    ) {
        let url = format!("{}/api/characters", self.base_url);
        let headers = make_headers_json(Some(token));

        let mut body = format!(
            r#"{{"name":"{}","gender":"{}","skin":"{}""#,
            name, gender, skin
        );
        if let Some(style) = hair_style {
            body.push_str(&format!(r#","hair_style":{}"#, style));
        }
        if let Some(color) = hair_color {
            body.push_str(&format!(r#","hair_color":{}"#, color));
        }
        body.push('}');

        let id = fire_request("POST", &url, &headers, Some(&body));
        self.pending_request = Some(PendingRequest {
            id,
            kind: PendingRequestKind::CreateCharacter,
        });
    }

    pub fn start_delete_character(&mut self, token: &str, character_id: i64) {
        let url = format!("{}/api/characters/{}", self.base_url, character_id);
        let headers = make_headers_json(Some(token));
        let id = fire_request("DELETE", &url, &headers, None);
        self.pending_request = Some(PendingRequest {
            id,
            kind: PendingRequestKind::DeleteCharacter,
        });
    }

    pub fn start_matchmake(&mut self, token: &str, character_id: i64, room_type: &str) {
        let url = format!("{}/matchmake/joinOrCreate/{}", self.base_url, room_type);
        let headers = make_headers_json(Some(token));
        let body = format!(r#"{{"characterId":{}}}"#, character_id);
        let id = fire_request("POST", &url, &headers, Some(&body));
        self.pending_request = Some(PendingRequest {
            id,
            kind: PendingRequestKind::Matchmake,
        });
    }

    pub fn start_health_check(&mut self) {
        let url = format!("{}/health", self.base_url);
        let headers = r#"{"Content-Type":"application/json"}"#.to_string();
        let id = fire_request("GET", &url, &headers, None);
        self.pending_request = Some(PendingRequest {
            id,
            kind: PendingRequestKind::HealthCheck,
        });
    }

    pub fn poll(&mut self) -> Option<AuthResult> {
        let pending = self.pending_request.as_ref()?;
        let status = unsafe { http_poll(pending.id) };

        if status == 0 {
            return None; // still pending
        }

        let request_id = pending.id;
        let http_status = unsafe { http_response_status(request_id) };
        let body_text = get_response_body(request_id);

        // Take ownership of the pending request
        let pending = self.pending_request.take().unwrap();
        unsafe { http_cleanup(request_id) };

        if status == 2 {
            // Network error
            let err = AuthError::NetworkError(body_text);
            return Some(match pending.kind {
                PendingRequestKind::Login => AuthResult::Login(Err(err)),
                PendingRequestKind::Register => AuthResult::Register(Err(err)),
                PendingRequestKind::GetCharacters => AuthResult::Characters(Err(err)),
                PendingRequestKind::CreateCharacter => AuthResult::CharacterCreated(Err(err)),
                PendingRequestKind::DeleteCharacter => AuthResult::CharacterDeleted(Err(err)),
                PendingRequestKind::Matchmake => AuthResult::Matchmake(Err(err)),
                PendingRequestKind::HealthCheck => AuthResult::HealthCheck(false),
            });
        }

        // status == 1, success (HTTP response received, may still be an error status code)
        Some(match pending.kind {
            PendingRequestKind::Login => AuthResult::Login(self.parse_auth_response(&body_text, http_status)),
            PendingRequestKind::Register => AuthResult::Register(self.parse_auth_response(&body_text, http_status)),
            PendingRequestKind::GetCharacters => AuthResult::Characters(self.parse_characters_response(&body_text, http_status)),
            PendingRequestKind::CreateCharacter => AuthResult::CharacterCreated(self.parse_create_character_response(&body_text, http_status)),
            PendingRequestKind::DeleteCharacter => AuthResult::CharacterDeleted(self.parse_delete_response(&body_text, http_status)),
            PendingRequestKind::Matchmake => AuthResult::Matchmake(self.parse_matchmake_response(&body_text, http_status)),
            PendingRequestKind::HealthCheck => AuthResult::HealthCheck(http_status == 200),
        })
    }

    fn parse_auth_response(&self, body: &str, http_status: i32) -> Result<AuthSession, AuthError> {
        if http_status == 401 {
            return Err(AuthError::InvalidCredentials);
        }

        let resp: AuthResponse =
            serde_json::from_str(body).map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if resp.success {
            Ok(AuthSession {
                token: resp.token.unwrap_or_default(),
                username: resp.username.unwrap_or_default(),
            })
        } else {
            let error = resp.error.unwrap_or_else(|| "Unknown error".to_string());
            if error.contains("Invalid") {
                Err(AuthError::InvalidCredentials)
            } else if error.contains("already exists") {
                Err(AuthError::UsernameTaken)
            } else {
                Err(AuthError::ServerError(error))
            }
        }
    }

    fn parse_characters_response(
        &self,
        body: &str,
        http_status: i32,
    ) -> Result<Vec<CharacterInfo>, AuthError> {
        if http_status == 401 {
            return Err(AuthError::Unauthorized);
        }

        let resp: CharacterListResponse =
            serde_json::from_str(body).map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if resp.success {
            Ok(resp.characters.unwrap_or_default())
        } else {
            Err(AuthError::ServerError(
                resp.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    fn parse_create_character_response(
        &self,
        body: &str,
        http_status: i32,
    ) -> Result<CharacterInfo, AuthError> {
        if http_status == 401 {
            return Err(AuthError::Unauthorized);
        }
        if http_status == 409 {
            return Err(AuthError::CharacterNameTaken);
        }

        let resp: CharacterCreateResponse =
            serde_json::from_str(body).map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if resp.success {
            resp.character
                .ok_or_else(|| AuthError::ServerError("No character returned".to_string()))
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

    fn parse_delete_response(&self, body: &str, http_status: i32) -> Result<(), AuthError> {
        if http_status == 401 {
            return Err(AuthError::Unauthorized);
        }
        if http_status >= 200 && http_status < 300 {
            Ok(())
        } else {
            let resp: Option<DeleteResponse> = serde_json::from_str(body).ok();
            let error = resp
                .and_then(|r| r.error)
                .unwrap_or_else(|| format!("HTTP {}", http_status));
            Err(AuthError::ServerError(error))
        }
    }

    fn parse_matchmake_response(
        &self,
        body: &str,
        http_status: i32,
    ) -> Result<(String, String), AuthError> {
        if http_status == 401 {
            return Err(AuthError::Unauthorized);
        }

        let resp: MatchmakeResponse =
            serde_json::from_str(body).map_err(|e| AuthError::NetworkError(e.to_string()))?;

        let room = resp
            .room
            .ok_or_else(|| AuthError::ServerError("No room returned".to_string()))?;
        let session_token = resp
            .session_token
            .ok_or_else(|| AuthError::ServerError("No session returned".to_string()))?;

        Ok((room.room_id, session_token))
    }
}
