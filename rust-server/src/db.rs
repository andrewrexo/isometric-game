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
    pub exp_to_next_level: i32,
    pub gold: i32,
    pub inventory_json: String, // JSON serialized inventory
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
        // Create players table
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
                exp_to_next_level INTEGER DEFAULT 100,
                gold INTEGER DEFAULT 0,
                inventory_json TEXT DEFAULT '[]',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_login DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Add new columns if they don't exist (for existing databases)
        let _ = sqlx::query("ALTER TABLE players ADD COLUMN exp_to_next_level INTEGER DEFAULT 100")
            .execute(pool).await;
        let _ = sqlx::query("ALTER TABLE players ADD COLUMN gold INTEGER DEFAULT 0")
            .execute(pool).await;
        let _ = sqlx::query("ALTER TABLE players ADD COLUMN inventory_json TEXT DEFAULT '[]'")
            .execute(pool).await;

        // Quest system tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS player_quests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                player_id INTEGER NOT NULL,
                quest_id TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'active',
                objectives_json TEXT DEFAULT '{}',
                started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                FOREIGN KEY(player_id) REFERENCES players(id),
                UNIQUE(player_id, quest_id)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS player_flags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                player_id INTEGER NOT NULL,
                flag_name TEXT NOT NULL,
                flag_value TEXT,
                FOREIGN KEY(player_id) REFERENCES players(id),
                UNIQUE(player_id, flag_name)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS player_quest_availability (
                player_id INTEGER NOT NULL,
                quest_id TEXT NOT NULL,
                unlocked_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(player_id, quest_id),
                FOREIGN KEY(player_id) REFERENCES players(id)
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
            "SELECT id, username, password_hash, x, y, hp, max_hp, level, exp, exp_to_next_level, gold, inventory_json FROM players WHERE username = ?",
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
            exp_to_next_level: r.get("exp_to_next_level"),
            gold: r.get("gold"),
            inventory_json: r.get("inventory_json"),
        }))
    }

    pub async fn save_player(
        &self,
        username: &str,
        x: f32,
        y: f32,
        hp: i32,
        max_hp: i32,
        level: i32,
        exp: i32,
        exp_to_next_level: i32,
        gold: i32,
        inventory_json: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE players SET
                x = ?, y = ?, hp = ?, max_hp = ?, level = ?, exp = ?,
                exp_to_next_level = ?, gold = ?, inventory_json = ?,
                last_login = CURRENT_TIMESTAMP
            WHERE username = ?"#,
        )
        .bind(x)
        .bind(y)
        .bind(hp)
        .bind(max_hp)
        .bind(level)
        .bind(exp)
        .bind(exp_to_next_level)
        .bind(gold)
        .bind(inventory_json)
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
