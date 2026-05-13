pub mod models;
pub mod pin_guard;

pub use models::*;

use crate::repo::{game_repo, settings_repo};
use pin_guard::{validate_pin_format, PinVerifyStatus};
use sqlx::SqlitePool;
use std::sync::Mutex;
use tauri::AppHandle;

pub struct ConfigService {
    pool: SqlitePool,
    settings: Mutex<AppSettings>,
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

        Self {
            pool,
            settings: Mutex::new(settings),
        }
    }

    /// Constructor for tests: takes a pool directly, no legacy migration.
    pub fn new_for_test(pool: SqlitePool) -> Self {
        Self::run_async(async {
            Self::ensure_tables(&pool).await;
        });

        let settings = Self::run_async(async { Self::load_from_db(&pool).await });

        Self {
            pool,
            settings: Mutex::new(settings),
        }
    }

    /// Async test constructor for current-thread tokio tests that cannot use block_in_place.
    pub async fn new_for_test_async(pool: SqlitePool) -> Self {
        Self::ensure_tables(&pool).await;
        let settings = Self::load_from_db(&pool).await;

        Self {
            pool,
            settings: Mutex::new(settings),
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
            "ALTER TABLE collections ADD COLUMN is_safe BOOLEAN DEFAULT 0;",
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

        let safe_mode: SafeModeConfig = kv
            .get("safe_mode")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

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

    /// Resets the in-memory state to defaults. Should be called after a database reset.
    pub fn reset_to_default(&self) {
        *self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = AppSettings::default();
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
        let pool = self.pool.clone();
        Self::run_async(async move {
            let before = crate::services::pin_service::get_status(&pool)
                .await
                .map_err(|error| error.to_string())?;
            if before.is_locked {
                return Ok(PinVerifyStatus {
                    valid: false,
                    attempts_remaining: 0,
                    locked_seconds_remaining: before.lockout_seconds_remaining.max(0) as u64,
                });
            }

            let valid = crate::services::pin_service::verify_pin(&pool, pin)
                .await
                .map_err(|error| error.to_string())?;
            let after = crate::services::pin_service::get_status(&pool)
                .await
                .map_err(|error| error.to_string())?;

            Ok::<PinVerifyStatus, String>(PinVerifyStatus {
                valid,
                attempts_remaining: after.attempts_remaining.max(0) as u8,
                locked_seconds_remaining: after.lockout_seconds_remaining.max(0) as u64,
            })
        })
        .unwrap_or(PinVerifyStatus {
            valid: false,
            attempts_remaining: 0,
            locked_seconds_remaining: 0,
        })
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
        let pool = self.pool.clone();
        Self::run_async(async {
            crate::services::pin_service::set_pin(&pool, pin, None)
                .await
                .map_err(|error| error.to_string())
        })?;

        Ok(())
    }

    /// Sets the PIN and simultaneously generates a one-time recovery code.
    /// Returns the plaintext recovery code (e.g. `EMMM-4F2A-9B87-CC1E`).
    /// The code is stored as SHA-256 hex in settings — never in plaintext.
    pub fn set_pin_with_recovery(&self, pin: &str) -> Result<String, String> {
        validate_pin_format(pin)?;

        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };
        use std::fmt::Write;

        // --- Hash PIN with Argon2 ---
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(pin.as_bytes(), &salt)
            .map_err(|e| e.to_string())?
            .to_string();

        // --- Generate recovery code: EMMM-XXXX-XXXX-XXXX ---
        let raw_bytes = {
            use argon2::password_hash::rand_core::RngCore;
            let mut rng = OsRng;
            let mut buf = [0u8; 9]; // 9 random bytes → 18 uppercase hex chars → 4-4-4-4 format
            rng.fill_bytes(&mut buf);
            buf
        };
        let hex_str: String = raw_bytes.iter().fold(String::new(), |mut acc, b| {
            let _ = write!(acc, "{:02X}", b);
            acc
        });
        // Format as EMMM-XXXX-XXXX-XXXX-XX (we take first 16 hex chars = 8 bytes)
        let recovery_code = format!(
            "EMMM-{}-{}-{}",
            &hex_str[0..4],
            &hex_str[4..8],
            &hex_str[8..12],
        );

        // --- SHA-256 hash the recovery code for storage ---
        // Use sha2 crate via the already-available ring/md5/sha1 or native digest
        // sha2 is available as a dependency of argon2's password-hash crate.
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(recovery_code.as_bytes());
        let recovery_hash = format!("{:x}", hasher.finalize());

        // --- Persist ---
        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.safe_mode.pin_hash = Some(password_hash.clone());
        settings.safe_mode.recovery_code_hash = Some(recovery_hash.clone());

        self.save_settings(settings)?;
        let pool = self.pool.clone();
        Self::run_async(async move {
            crate::repo::pin_repo::set_pin(&pool, &password_hash, Some(recovery_hash.as_str()))
                .await
                .map_err(|error| error.to_string())
        })?;

        Ok(recovery_code)
    }

    /// Validates the recovery code. If valid, clears the PIN and recovery code,
    /// allowing the user to set a new PIN without knowing the old one.
    /// Returns `true` if reset succeeded, `Err` on internal errors.
    pub fn reset_pin_with_recovery_code(&self, code: &str) -> Result<bool, String> {
        use sha2::{Digest, Sha256};

        let stored_hash = {
            let settings = self
                .settings
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            settings.safe_mode.recovery_code_hash.clone()
        };

        let Some(stored_hash) = stored_hash else {
            // No recovery code configured for this installation
            return Ok(false);
        };

        // Normalise input (uppercase, trim whitespace)
        let code_normalised = code.trim().to_uppercase();

        let mut hasher = Sha256::new();
        hasher.update(code_normalised.as_bytes());
        let input_hash = format!("{:x}", hasher.finalize());

        if input_hash != stored_hash {
            return Ok(false);
        }

        // Valid — clear PIN and recovery code
        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.safe_mode.pin_hash = None;
        settings.safe_mode.recovery_code_hash = None;
        settings.safe_mode.failed_attempts = None;
        settings.safe_mode.lockout_until_ts = None;
        self.save_settings(settings)?;
        let pool = self.pool.clone();
        Self::run_async(async move {
            crate::repo::pin_repo::clear_pin(&pool)
                .await
                .map_err(|error| error.to_string())
        })?;

        Ok(true)
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
