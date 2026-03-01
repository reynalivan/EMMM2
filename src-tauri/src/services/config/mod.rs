pub mod models;
pub mod pin_guard;

pub use models::*;

use crate::database::{game_repo, settings_repo};
use pin_guard::{validate_pin_format, PinGuardState, PinVerifyStatus};
use sqlx::SqlitePool;
use std::sync::Mutex;
use tauri::AppHandle;

pub struct ConfigService {
    pool: SqlitePool,
    settings: Mutex<AppSettings>,
    pin_guard: Mutex<PinGuardState>,
}

impl ConfigService {
    /// Run an async future from a synchronous context.
    /// Works both inside Tauri's runtime and inside `#[tokio::test]`.
    fn run_async<F: std::future::Future>(f: F) -> F::Output {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(f)),
            Err(_) => tauri::async_runtime::block_on(f),
        }
    }

    /// Initialize from Tauri AppHandle. Runs migration and loads from DB.
    pub fn init(_app_handle: &AppHandle, pool: SqlitePool) -> Self {
        // 1. Run our table creation (idempotent, so safe even if the
        //    tauri_plugin_sql migration already ran).
        Self::run_async(async {
            Self::ensure_tables(&pool).await;
        });

        // 2. Load current settings from DB
        let settings = Self::run_async(async { Self::load_from_db(&pool).await });

        let mut pin_guard = PinGuardState::default();
        if let Some(failed) = settings.safe_mode.failed_attempts {
            pin_guard = PinGuardState::new(failed, settings.safe_mode.lockout_until_ts);
        }

        Self {
            pool,
            settings: Mutex::new(settings),
            pin_guard: Mutex::new(pin_guard),
        }
    }

    /// Constructor for tests: takes a pool directly, no legacy migration.
    pub fn new_for_test(pool: SqlitePool) -> Self {
        Self::run_async(async {
            Self::ensure_tables(&pool).await;
        });

        let settings = Self::run_async(async { Self::load_from_db(&pool).await });

        let mut pin_guard = PinGuardState::default();
        if let Some(failed) = settings.safe_mode.failed_attempts {
            pin_guard = PinGuardState::new(failed, settings.safe_mode.lockout_until_ts);
        }

        Self {
            pool,
            settings: Mutex::new(settings),
            pin_guard: Mutex::new(pin_guard),
        }
    }

    /// Create tables and apply ad-hoc schema patches if they don't exist (idempotent).
    async fn ensure_tables(pool: &SqlitePool) {
        // Games table (matches 001_init.sql + 012 ALTER extensions)
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS games (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                game_type TEXT NOT NULL,
                path TEXT NOT NULL,
                launcher_path TEXT,
                launch_args TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                mod_path TEXT,
                game_exe TEXT,
                loader_exe TEXT
            )",
        )
        .execute(pool)
        .await;

        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await;

        // Apply fallback schema patches.
        // If `tauri_plugin_sql` migrations failed due to duplicate column errors
        // (e.g. users migrating from older SQLx tracked versions to new Tauri plugin versions),
        // we explicitly add required columns here and safely ignore "duplicate column" errors.
        let patches = [
            "ALTER TABLE games ADD COLUMN mod_path TEXT;",
            "ALTER TABLE games ADD COLUMN game_exe TEXT;",
            "ALTER TABLE games ADD COLUMN loader_exe TEXT;",
            "ALTER TABLE collections ADD COLUMN is_safe_context BOOLEAN DEFAULT 0;",
            "ALTER TABLE collections ADD COLUMN is_favorite BOOLEAN DEFAULT 0;",
            "ALTER TABLE objects ADD COLUMN is_pinned BOOLEAN DEFAULT 0;",
            "ALTER TABLE objects ADD COLUMN is_auto_sync BOOLEAN NOT NULL DEFAULT 0;",
            "ALTER TABLE mods ADD COLUMN last_status_sfw BOOLEAN;",
            "ALTER TABLE mods ADD COLUMN last_status_nsfw BOOLEAN;",
        ];

        for patch in patches {
            let _ = sqlx::query(patch).execute(pool).await;
        }
    }

    /// Load AppSettings from the SQLite database.
    async fn load_from_db(pool: &SqlitePool) -> AppSettings {
        let kv = match settings_repo::get_all_settings(pool).await {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to load settings from DB: {e}");
                return AppSettings::default();
            }
        };

        let games = match game_repo::get_all_games(pool).await {
            Ok(rows) => rows.into_iter().map(game_row_to_config).collect(),
            Err(e) => {
                log::error!("Failed to load games from DB: {e}");
                Vec::new()
            }
        };

        let theme = kv.get("theme").cloned().unwrap_or_else(|| "dark".into());
        let language = kv.get("language").cloned().unwrap_or_else(|| "en".into());
        let active_game_id = kv.get("active_game_id").cloned();

        let mut safe_mode: SafeModeConfig = kv
            .get("safe_mode")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

        // Epic 7 Requirement: App ALWAYS starts in SFW mode regardless of previous session.
        safe_mode.enabled = true;

        let ai: AiConfig = kv
            .get("ai")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

        let auto_close_launcher = kv
            .get("auto_close_launcher")
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let hotkeys = kv
            .get("hotkeys")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

        let keyviewer = kv
            .get("keyviewer")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

        AppSettings {
            theme,
            language,
            games,
            active_game_id,
            safe_mode,
            ai,
            auto_close_launcher,
            hotkeys,
            keyviewer,
        }
    }

    /// Write the full AppSettings to the database in a single transaction.
    pub(crate) async fn write_settings_to_db(
        pool: &SqlitePool,
        settings: &AppSettings,
    ) -> Result<(), String> {
        settings_repo::set_setting(pool, "theme", &settings.theme)
            .await
            .map_err(|e| e.to_string())?;
        settings_repo::set_setting(pool, "language", &settings.language)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(ref id) = settings.active_game_id {
            settings_repo::set_setting(pool, "active_game_id", id)
                .await
                .map_err(|e| e.to_string())?;
        }

        settings_repo::set_setting(
            pool,
            "auto_close_launcher",
            &settings.auto_close_launcher.to_string(),
        )
        .await
        .map_err(|e| e.to_string())?;

        let safe_mode_json =
            serde_json::to_string(&settings.safe_mode).map_err(|e| e.to_string())?;
        settings_repo::set_setting(pool, "safe_mode", &safe_mode_json)
            .await
            .map_err(|e| e.to_string())?;

        let ai_json = serde_json::to_string(&settings.ai).map_err(|e| e.to_string())?;
        settings_repo::set_setting(pool, "ai", &ai_json)
            .await
            .map_err(|e| e.to_string())?;

        let hotkeys_json = serde_json::to_string(&settings.hotkeys).map_err(|e| e.to_string())?;
        settings_repo::set_setting(pool, "hotkeys", &hotkeys_json)
            .await
            .map_err(|e| e.to_string())?;

        let keyviewer_json =
            serde_json::to_string(&settings.keyviewer).map_err(|e| e.to_string())?;
        settings_repo::set_setting(pool, "keyviewer", &keyviewer_json)
            .await
            .map_err(|e| e.to_string())?;

        // Persist games
        for game in &settings.games {
            let row = config_to_game_row(game);
            game_repo::upsert_game(pool, &row)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn get_settings(&self) -> AppSettings {
        self.settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn save_settings(&self, mut new_settings: AppSettings) -> Result<(), String> {
        new_settings.safe_mode.keywords = normalize_keywords(&new_settings.safe_mode.keywords);

        // Write to DB synchronously
        let pool = self.pool.clone();
        Self::run_async(async { Self::write_settings_to_db(&pool, &new_settings).await })?;

        // Update in-memory state
        *self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = new_settings;
        Ok(())
    }

    pub fn verify_pin_status(&self, pin: &str) -> PinVerifyStatus {
        let (pin_hash, original_snapshot) = {
            let settings = self
                .settings
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let safe_mode = &settings.safe_mode;
            let snapshot = (
                safe_mode.failed_attempts.unwrap_or(0),
                safe_mode.lockout_until_ts,
            );
            (safe_mode.pin_hash.clone(), snapshot)
        };

        let new_status = {
            let mut guard = self
                .pin_guard
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.verify(pin, pin_hash.as_deref())
        };

        let current_snapshot = self
            .pin_guard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .snapshot();
        if current_snapshot != original_snapshot {
            let mut updated_settings = self
                .settings
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            updated_settings.safe_mode.failed_attempts = Some(current_snapshot.0);
            updated_settings.safe_mode.lockout_until_ts = current_snapshot.1;
            let _ = self.save_settings(updated_settings);
        }

        new_status
    }

    pub fn verify_pin(&self, pin: &str) -> bool {
        self.verify_pin_status(pin).valid
    }

    pub fn set_pin(&self, pin: &str) -> Result<(), String> {
        validate_pin_format(pin)?;

        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(pin.as_bytes(), &salt)
            .map_err(|e| e.to_string())?
            .to_string();

        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.safe_mode.pin_hash = Some(password_hash);

        self.save_settings(settings)?;
        self.pin_guard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .reset();

        Ok(())
    }

    pub fn set_active_game(&self, game_id: Option<String>) -> Result<(), String> {
        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.active_game_id = game_id;
        self.save_settings(settings)
    }

    pub fn set_safe_mode_enabled(&self, enabled: bool) -> Result<(), String> {
        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.safe_mode.enabled = enabled;
        self.save_settings(settings)
    }

    pub fn set_auto_close_launcher(&self, enabled: bool) -> Result<(), String> {
        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.auto_close_launcher = enabled;
        self.save_settings(settings)
    }

    /// Get a reference to the pool (for use in commands that need direct DB access).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// ── Helpers ──────────────────────────────────────────

fn normalize_keywords(keywords: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::new();
    for keyword in keywords {
        let next = keyword.trim().to_lowercase();
        if next.is_empty() || normalized.contains(&next) {
            continue;
        }
        normalized.push(next);
    }

    normalized
}
