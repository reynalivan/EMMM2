use crate::common::sync::lock;
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

    /// Initialize from Tauri AppHandle. The pool is already migrated by
    /// `bootstrap::init_pool`, so this only loads current settings.
    pub fn init(_app_handle: &AppHandle, pool: SqlitePool) -> Self {
        let settings = Self::run_async(async { Self::load_from_db(&pool).await });

        Self {
            pool,
            settings: Mutex::new(settings),
        }
    }

    /// Constructor for tests: takes an already-migrated pool directly.
    pub fn new_for_test(pool: SqlitePool) -> Self {
        let settings = Self::run_async(async { Self::load_from_db(&pool).await });

        Self {
            pool,
            settings: Mutex::new(settings),
        }
    }

    /// Async test constructor for current-thread tokio tests that cannot use block_in_place.
    pub async fn new_for_test_async(pool: SqlitePool) -> Self {
        let settings = Self::load_from_db(&pool).await;

        Self {
            pool,
            settings: Mutex::new(settings),
        }
    }

    /// Resets the in-memory state to defaults. Should be called after a database reset.
    pub fn reset_to_default(&self) {
        *lock(&self.settings) = AppSettings::default();
    }

    pub fn get_settings(&self) -> AppSettings {
        lock(&self.settings).clone()
    }

    /// Read a projection of the settings without cloning the whole struct.
    ///
    /// `get_settings` deep-clones every `GameConfig`, keyword list, and hotkey
    /// binding — fine once per operation, wasteful when a caller only needs one
    /// field and runs per grid card or per bulk item.
    pub fn with_settings<R>(&self, read: impl FnOnce(&AppSettings) -> R) -> R {
        read(&lock(&self.settings))
    }

    /// The configured game whose mods root contains `path`, if any. Used by
    /// import/restore flows that receive a directory rather than a game id
    /// but must reconcile that game afterwards.
    pub fn game_id_for_path(&self, path: &std::path::Path) -> Option<String> {
        self.with_settings(|settings| {
            settings
                .games
                .iter()
                .find(|game| path.starts_with(&game.mod_path))
                .map(|game| game.id.clone())
        })
    }

    /// Whether the Safe Mode corridor is active.
    /// The corridor the app is operating in right now.
    ///
    /// The only place a `Corridor` value is born: commands derive it here and
    /// pass it down, so Safe Mode can never be supplied over IPC.
    pub fn current_corridor(&self) -> crate::domain::corridor::Corridor {
        crate::domain::corridor::Corridor::from_is_safe(self.safe_mode_enabled())
    }

    /// Corridor after an optional per-request PIN proof: a valid PIN widens
    /// Safe to Unsafe. With no PIN configured there is nothing to prove, so
    /// the corridor stays Safe.
    pub fn corridor_with_elevation(&self, pin: Option<&str>) -> crate::domain::corridor::Corridor {
        let corridor = self.current_corridor();
        if corridor.is_safe() && pin.is_some_and(|value| self.pin_grants_elevation(value)) {
            return crate::domain::corridor::Corridor::Unsafe;
        }
        corridor
    }

    pub fn safe_mode_enabled(&self) -> bool {
        self.with_settings(|settings| settings.safe_mode.enabled)
    }

    /// The configured mods root for a game, if it has one.
    pub fn mods_root_for(&self, game_id: &str) -> Option<std::path::PathBuf> {
        self.with_settings(|settings| {
            settings
                .games
                .iter()
                .find(|game| game.id == game_id)
                .map(|game| game.mod_path.clone())
        })
    }

    pub fn save_settings(&self, mut new_settings: AppSettings) -> Result<(), AppError> {
        new_settings.safe_mode.keywords = normalize_keywords(&new_settings.safe_mode.keywords);

        // Write to DB synchronously
        let pool = self.pool.clone();
        Self::run_async(async { Self::write_settings_to_db(&pool, &new_settings).await })?;

        // Update in-memory state
        *lock(&self.settings) = new_settings;
        Ok(())
    }

    pub fn set_active_game(&self, game_id: Option<String>) -> Result<(), AppError> {
        let mut settings = lock(&self.settings).clone();
        settings.active_game_id = game_id;
        self.save_settings(settings)
    }

    pub fn set_auto_close_launcher(&self, enabled: bool) -> Result<(), AppError> {
        let mut settings = lock(&self.settings).clone();
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
