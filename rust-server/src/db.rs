use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct PlayerData {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub x: f32,
    pub y: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub level: i32,
    pub exp: i32,
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Run migrations
        Self::migrate(&pool).await?;

        Ok(Self { pool })
    }

    async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS players (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                x REAL DEFAULT 16.0,
                y REAL DEFAULT 16.0,
                hp INTEGER DEFAULT 100,
                max_hp INTEGER DEFAULT 100,
                level INTEGER DEFAULT 1,
                exp INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_login DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await?;

        tracing::info!("Database migrations complete");
        Ok(())
    }

    /// Create a new player with hashed password
    pub async fn create_player(
        &self,
        username: &str,
        password: &str,
    ) -> Result<i64, String> {
        // Hash the password with Argon2
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Failed to hash password: {}", e))?
            .to_string();

        let result = sqlx::query(
            "INSERT INTO players (username, password_hash) VALUES (?, ?)",
        )
        .bind(username)
        .bind(&password_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                "Username already exists".to_string()
            } else {
                format!("Database error: {}", e)
            }
        })?;

        tracing::info!("Created player: {} (id: {})", username, result.last_insert_rowid());
        Ok(result.last_insert_rowid())
    }

    pub async fn get_player_by_username(&self, username: &str) -> Result<Option<PlayerData>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, username, password_hash, x, y, hp, max_hp, level, exp FROM players WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| PlayerData {
            id: r.get("id"),
            username: r.get("username"),
            password_hash: r.get("password_hash"),
            x: r.get("x"),
            y: r.get("y"),
            hp: r.get("hp"),
            max_hp: r.get("max_hp"),
            level: r.get("level"),
            exp: r.get("exp"),
        }))
    }

    pub async fn save_player(&self, username: &str, x: f32, y: f32, hp: i32, level: i32, exp: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE players SET x = ?, y = ?, hp = ?, level = ?, exp = ?, last_login = CURRENT_TIMESTAMP WHERE username = ?",
        )
        .bind(x)
        .bind(y)
        .bind(hp)
        .bind(level)
        .bind(exp)
        .bind(username)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_player_position(&self, username: &str, x: f32, y: f32) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE players SET x = ?, y = ? WHERE username = ?",
        )
        .bind(x)
        .bind(y)
        .bind(username)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Verify password and return player data if valid
    pub async fn verify_password(&self, username: &str, password: &str) -> Option<PlayerData> {
        if let Ok(Some(player)) = self.get_player_by_username(username).await {
            // Parse the stored hash
            if let Ok(parsed_hash) = PasswordHash::new(&player.password_hash) {
                // Verify password against stored hash
                if Argon2::default()
                    .verify_password(password.as_bytes(), &parsed_hash)
                    .is_ok()
                {
                    return Some(player);
                }
            }
        }
        None
    }
}
