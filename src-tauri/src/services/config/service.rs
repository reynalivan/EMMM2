use crate::domain::errors::AppError;
use sqlx::SqlitePool;
use std::sync::Mutex;
use tauri::AppHandle;

use super::models::AppSettings;

pub struct ConfigService {
    pub(super) pool: SqlitePool,
    pub(super) settings: Mutex<AppSettings>,
}

impl ConfigService {
    /// Run an async future from a synchronous context.
    /// Use the async constructors from current-thread `#[tokio::test]`.
    pub(super) fn run_async<F: std::future::Future>(f: F) -> F::Output {
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

    pub fn save_settings(&self, mut new_settings: AppSettings) -> Result<(), AppError> {
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

    pub fn set_active_game(&self, game_id: Option<String>) -> Result<(), AppError> {
        let mut settings = self
            .settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        settings.active_game_id = game_id;
        self.save_settings(settings)
    }

    pub fn set_auto_close_launcher(&self, enabled: bool) -> Result<(), AppError> {
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
