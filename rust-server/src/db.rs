use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use crate::quest::state::{PlayerQuestState, QuestProgress, QuestStatus, ObjectiveProgress};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

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
    pub gender: String,         // "male" or "female"
    pub skin: String,           // "tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"
    pub equipped_body: Option<String>, // Item ID of equipped body armor
}

// Available appearance options
pub const GENDERS: &[&str] = &["male", "female"];
pub const SKINS: &[&str] = &["tan", "pale", "brown", "purple", "orc", "ghost", "skeleton"];

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
        let _ = sqlx::query("ALTER TABLE players ADD COLUMN gender TEXT DEFAULT 'male'")
            .execute(pool).await;
        let _ = sqlx::query("ALTER TABLE players ADD COLUMN skin TEXT DEFAULT 'tan'")
            .execute(pool).await;
        let _ = sqlx::query("ALTER TABLE players ADD COLUMN equipped_body TEXT")
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

    /// Create a new player with hashed password and random appearance
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

        // Assign random appearance
        let gender_idx = rand::random::<usize>() % GENDERS.len();
        let skin_idx = rand::random::<usize>() % SKINS.len();
        let gender = GENDERS[gender_idx];
        let skin = SKINS[skin_idx];

        let result = sqlx::query(
            "INSERT INTO players (username, password_hash, gender, skin) VALUES (?, ?, ?, ?)",
        )
        .bind(username)
        .bind(&password_hash)
        .bind(gender)
        .bind(skin)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                "Username already exists".to_string()
            } else {
                format!("Database error: {}", e)
            }
        })?;

        tracing::info!("Created player: {} (id: {}) with appearance: {} {}",
            username, result.last_insert_rowid(), gender, skin);
        Ok(result.last_insert_rowid())
    }

    pub async fn get_player_by_username(&self, username: &str) -> Result<Option<PlayerData>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, username, password_hash, x, y, hp, max_hp, level, exp, exp_to_next_level, gold, inventory_json, gender, skin, equipped_body FROM players WHERE username = ?",
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
            gender: r.try_get("gender").unwrap_or_else(|_| "male".to_string()),
            skin: r.try_get("skin").unwrap_or_else(|_| "tan".to_string()),
            equipped_body: r.try_get::<String, _>("equipped_body").ok().filter(|s| !s.is_empty()),
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
        equipped_body: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE players SET
                x = ?, y = ?, hp = ?, max_hp = ?, level = ?, exp = ?,
                exp_to_next_level = ?, gold = ?, inventory_json = ?,
                equipped_body = ?,
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
        .bind(equipped_body)
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

    /// Load quest state for a player from database
    pub async fn load_quest_state(&self, db_player_id: i64) -> Result<PlayerQuestState, sqlx::Error> {
        let mut state = PlayerQuestState::new();

        // Load quests from player_quests table
        let quest_rows = sqlx::query(
            "SELECT quest_id, state, objectives_json, started_at, completed_at FROM player_quests WHERE player_id = ?"
        )
        .bind(db_player_id)
        .fetch_all(&self.pool)
        .await?;

        for row in quest_rows {
            let quest_id: String = row.get("quest_id");
            let state_str: String = row.get("state");
            let objectives_json: String = row.get("objectives_json");
            let started_at: Option<String> = row.get("started_at");
            let completed_at: Option<String> = row.get("completed_at");

            let status = QuestStatus::from_str(&state_str).unwrap_or(QuestStatus::Active);

            // Parse timestamps
            let started_at_dt = started_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
            let completed_at_dt = completed_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));

            // Deserialize objectives
            let objectives = QuestProgress::objectives_from_json(&objectives_json);

            let progress = QuestProgress {
                quest_id: quest_id.clone(),
                status,
                objectives,
                started_at: started_at_dt,
                completed_at: completed_at_dt,
            };

            match status {
                QuestStatus::Completed => {
                    state.completed_quests.push(quest_id);
                }
                QuestStatus::Active | QuestStatus::ReadyToComplete => {
                    state.active_quests.insert(quest_id, progress);
                }
                _ => {
                    // Failed/Abandoned quests are stored but not active
                }
            }
        }

        // Load available quests
        let available_rows = sqlx::query(
            "SELECT quest_id FROM player_quest_availability WHERE player_id = ?"
        )
        .bind(db_player_id)
        .fetch_all(&self.pool)
        .await?;

        for row in available_rows {
            let quest_id: String = row.get("quest_id");
            // Only add if not already active or completed
            if !state.active_quests.contains_key(&quest_id) && !state.completed_quests.contains(&quest_id) {
                state.available_quests.push(quest_id);
            }
        }

        // Load flags
        let flag_rows = sqlx::query(
            "SELECT flag_name, flag_value FROM player_flags WHERE player_id = ?"
        )
        .bind(db_player_id)
        .fetch_all(&self.pool)
        .await?;

        for row in flag_rows {
            let flag_name: String = row.get("flag_name");
            let flag_value: Option<String> = row.get("flag_value");
            if let Some(value) = flag_value {
                state.flags.insert(flag_name, value);
            }
        }

        tracing::debug!("Loaded quest state for player {}: {} active, {} completed, {} available",
            db_player_id,
            state.active_quests.len(),
            state.completed_quests.len(),
            state.available_quests.len()
        );

        Ok(state)
    }

    /// Save quest state for a player to database
    pub async fn save_quest_state(&self, db_player_id: i64, state: &PlayerQuestState) -> Result<(), sqlx::Error> {
        // Save active quests
        for (quest_id, progress) in &state.active_quests {
            let objectives_json = progress.objectives_to_json();
            let started_at = progress.started_at.map(|dt| dt.to_rfc3339());
            let completed_at = progress.completed_at.map(|dt| dt.to_rfc3339());

            sqlx::query(
                r#"INSERT INTO player_quests (player_id, quest_id, state, objectives_json, started_at, completed_at)
                   VALUES (?, ?, ?, ?, ?, ?)
                   ON CONFLICT(player_id, quest_id) DO UPDATE SET
                       state = excluded.state,
                       objectives_json = excluded.objectives_json,
                       started_at = excluded.started_at,
                       completed_at = excluded.completed_at"#
            )
            .bind(db_player_id)
            .bind(quest_id)
            .bind(progress.status.as_str())
            .bind(&objectives_json)
            .bind(&started_at)
            .bind(&completed_at)
            .execute(&self.pool)
            .await?;
        }

        // Save completed quests
        for quest_id in &state.completed_quests {
            // Only insert if not exists - completed quests are immutable
            sqlx::query(
                r#"INSERT OR IGNORE INTO player_quests (player_id, quest_id, state, completed_at)
                   VALUES (?, ?, 'completed', CURRENT_TIMESTAMP)"#
            )
            .bind(db_player_id)
            .bind(quest_id)
            .execute(&self.pool)
            .await?;
        }

        // Save available quests
        for quest_id in &state.available_quests {
            sqlx::query(
                r#"INSERT OR IGNORE INTO player_quest_availability (player_id, quest_id)
                   VALUES (?, ?)"#
            )
            .bind(db_player_id)
            .bind(quest_id)
            .execute(&self.pool)
            .await?;
        }

        // Save flags
        for (flag_name, flag_value) in &state.flags {
            sqlx::query(
                r#"INSERT INTO player_flags (player_id, flag_name, flag_value)
                   VALUES (?, ?, ?)
                   ON CONFLICT(player_id, flag_name) DO UPDATE SET
                       flag_value = excluded.flag_value"#
            )
            .bind(db_player_id)
            .bind(flag_name)
            .bind(flag_value)
            .execute(&self.pool)
            .await?;
        }

        tracing::debug!("Saved quest state for player {}: {} active, {} completed",
            db_player_id,
            state.active_quests.len(),
            state.completed_quests.len()
        );

        Ok(())
    }
}
