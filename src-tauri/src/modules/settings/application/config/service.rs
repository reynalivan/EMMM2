use crate::shared::errors::AppError;
use crate::shared::sync::lock;
use sqlx::SqlitePool;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::AppHandle;

use super::models::AppSettings;
use crate::modules::automation::application::hotkeys::{HotkeyConfig, KeyViewerConfig};

pub struct ConfigService {
    pub(super) pool: SqlitePool,
    pub(super) settings: Mutex<AppSettings>,
    pub(super) settings_authoritative: AtomicBool,
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
    /// `app::bootstrap::init_pool`, so this only loads current settings.
    pub fn init(_app_handle: &AppHandle, pool: SqlitePool) -> Self {
        let (settings, settings_authoritative) =
            match Self::run_async(async { Self::load_from_db(&pool).await }) {
                Ok(settings) => (settings, true),
                Err(error) => {
                    log::error!("Failed to load settings from DB: {error}");
                    (AppSettings::default(), false)
                }
            };

        Self {
            pool,
            settings: Mutex::new(settings),
            settings_authoritative: AtomicBool::new(settings_authoritative),
        }
    }

    /// Constructor for tests: takes an already-migrated pool directly.
    pub fn new_for_test(pool: SqlitePool) -> Self {
        let (settings, settings_authoritative) =
            match Self::run_async(async { Self::load_from_db(&pool).await }) {
                Ok(settings) => (settings, true),
                Err(error) => {
                    log::error!("Failed to load test settings from DB: {error}");
                    (AppSettings::default(), false)
                }
            };

        Self {
            pool,
            settings: Mutex::new(settings),
            settings_authoritative: AtomicBool::new(settings_authoritative),
        }
    }

    /// Async test constructor for current-thread tokio tests that cannot use block_in_place.
    pub async fn new_for_test_async(pool: SqlitePool) -> Self {
        let (settings, settings_authoritative) = match Self::load_from_db(&pool).await {
            Ok(settings) => (settings, true),
            Err(error) => {
                log::error!("Failed to load async test settings from DB: {error}");
                (AppSettings::default(), false)
            }
        };

        Self {
            pool,
            settings: Mutex::new(settings),
            settings_authoritative: AtomicBool::new(settings_authoritative),
        }
    }

    /// Reset persistent and in-memory settings under the same writer lock.
    pub fn reset_database(&self, app_data_dir: &std::path::Path) -> Result<(), AppError> {
        let mut current = lock(&self.settings);
        let next_revision = current.revision.checked_add(1).ok_or_else(|| {
            AppError::Internal("Settings revision counter overflowed during reset".to_string())
        })?;
        let pool = self.pool.clone();
        Self::run_async(async {
            crate::modules::system::application::app::app_service::reset_database_service(
                &pool,
                app_data_dir,
                Some(next_revision),
            )
            .await
        })?;
        let defaults = AppSettings {
            revision: next_revision,
            ..AppSettings::default()
        };
        *current = defaults;
        self.settings_authoritative.store(true, Ordering::Release);
        Ok(())
    }

    pub fn get_settings(&self) -> AppSettings {
        lock(&self.settings).clone()
    }

    pub fn set_ai_key_status(&self, has_api_key: bool) {
        lock(&self.settings).ai.has_api_key = has_api_key;
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

    /// Persist a complete settings snapshot without allowing it to bypass the
    /// disk-recovery activation boundary.
    ///
    /// Callers that own one field should use [`Self::update_settings`] so a
    /// snapshot captured before another settings write cannot erase it.
    pub fn save_settings(&self, new_settings: AppSettings) -> Result<AppSettings, AppError> {
        self.update_settings(move |current| {
            if new_settings.revision != current.revision {
                return Err(AppError::Validation(
                    "Settings changed since this screen was loaded. Refresh and retry your edit."
                        .to_string(),
                ));
            }
            if new_settings.active_game_id != current.active_game_id {
                return Err(AppError::Validation(
                    "Use the active-game command to change the active game safely".to_string(),
                ));
            }
            ensure_unique_game_ids(&new_settings)?;
            ensure_existing_mod_paths_unchanged(current, &new_settings)?;
            let has_api_key = current.ai.has_api_key;
            // Runtime Safe Mode is controlled by the mutation path, never by
            // a whole Settings payload captured in the frontend.
            let runtime_safe_mode_by_game = current.safety.runtime_safe_mode_by_game.clone();
            *current = new_settings;
            current.ai.has_api_key = has_api_key;
            current.safety.runtime_safe_mode_by_game = runtime_safe_mode_by_game;
            if current
                .active_game_id
                .as_ref()
                .is_some_and(|active_id| !current.games.iter().any(|game| &game.id == active_id))
            {
                current.active_game_id = None;
            }
            Ok(())
        })?;
        Ok(self.get_settings())
    }

    /// Atomically derive and persist a settings change from the latest
    /// in-memory snapshot. The lock intentionally spans the database write:
    /// settings writers are rare, and serializing their commit point prevents
    /// stale whole-struct writes from reverting unrelated fields.
    pub(crate) fn update_settings<R>(
        &self,
        update: impl FnOnce(&mut AppSettings) -> Result<R, AppError>,
    ) -> Result<R, AppError> {
        if !self.settings_authoritative.load(Ordering::Acquire) {
            return Err(AppError::Db(
                "Settings were not loaded authoritatively; restart or repair the database before saving"
                    .to_string(),
            ));
        }
        let mut current = lock(&self.settings);
        let mut next = current.clone();
        let output = update(&mut next)?;
        next.revision = current.revision.checked_add(1).ok_or_else(|| {
            AppError::Internal("Settings revision counter overflowed".to_string())
        })?;
        next.safety.keywords = normalize_keywords(&next.safety.keywords);
        let removed_game_ids: Vec<String> = current
            .games
            .iter()
            .filter(|existing| !next.games.iter().any(|game| game.id == existing.id))
            .map(|game| game.id.clone())
            .collect();

        let pool = self.pool.clone();
        Self::run_async(async {
            Self::write_settings_to_db(&pool, &next, &removed_game_ids).await
        })?;
        *current = next;
        Ok(output)
    }

    pub fn set_active_game(&self, game_id: Option<String>) -> Result<(), AppError> {
        self.update_settings(move |settings| {
            settings.active_game_id = game_id;
            Ok(())
        })
    }

    pub fn set_auto_close_launcher(&self, enabled: bool) -> Result<(), AppError> {
        self.update_settings(move |settings| {
            settings.auto_close_launcher = enabled;
            Ok(())
        })
    }

    /// Commit the four runtime control bindings and KeyViewer enablement as
    /// one settings revision. The caller registers OS bindings first and
    /// restores them if this durable write fails.
    pub fn set_hotkey_configuration(
        &self,
        expected_revision: u64,
        hotkeys: HotkeyConfig,
        keyviewer: KeyViewerConfig,
    ) -> Result<AppSettings, AppError> {
        crate::modules::automation::application::hotkeys::manager::validate_binding_configuration(
            &hotkeys,
        )?;
        self.update_settings(move |settings| {
            if settings.revision != expected_revision {
                return Err(AppError::Validation(
                    "Settings changed since this screen was loaded. Refresh and retry your edit."
                        .to_string(),
                ));
            }
            settings.hotkeys = hotkeys;
            settings.keyviewer = keyviewer;
            Ok(())
        })?;
        Ok(self.get_settings())
    }

    /// Persist the per-game Safe Mode decision without accepting a stale full
    /// Settings snapshot from the UI.
    pub fn set_runtime_safe_mode(
        &self,
        game_id: &str,
        enabled: bool,
    ) -> Result<AppSettings, AppError> {
        self.update_settings(|settings| {
            if !settings.games.iter().any(|game| game.id == game_id) {
                return Err(AppError::NotFound(format!("Game {game_id} not found")));
            }
            settings
                .safety
                .set_runtime_safe_mode(game_id.to_string(), enabled);
            Ok(())
        })?;
        Ok(self.get_settings())
    }

    /// Update the diagnostics consent without accepting a stale full settings snapshot.
    pub fn set_telemetry_enabled(&self, enabled: bool) -> Result<AppSettings, AppError> {
        self.update_settings(move |settings| {
            settings.diagnostics.telemetry_enabled = enabled;
            Ok(())
        })?;
        Ok(self.get_settings())
    }

    /// Store the optional Mod Viewer executable without accepting a complete
    /// settings snapshot, so unrelated stale Settings saves cannot race this
    /// integration update.
    pub fn set_mod_viewer_executable(
        &self,
        executable: Option<PathBuf>,
    ) -> Result<AppSettings, AppError> {
        if let Some(path) = executable.as_deref() {
            validate_mod_viewer_executable(path)?;
        }

        self.update_settings(move |settings| {
            settings.external_tools.mod_viewer_executable = executable;
            Ok(())
        })?;
        Ok(self.get_settings())
    }

    /// Get a reference to the pool (for use in commands that need direct DB access).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

pub(crate) fn validate_mod_viewer_executable(path: &Path) -> Result<(), AppError> {
    if !path.is_absolute() {
        return Err(AppError::Validation(
            "3DMigoto Mod Viewer executable path must be absolute".to_string(),
        ));
    }
    if !path.is_file() {
        return Err(AppError::NotFound(format!(
            "3DMigoto Mod Viewer executable not found at: {}",
            path.display()
        )));
    }
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return Err(AppError::Validation(format!(
            "3DMigoto Mod Viewer executable must be a .exe file: {}",
            path.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/service_tests.rs"]
mod tests;

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

fn ensure_existing_mod_paths_unchanged(
    current: &AppSettings,
    requested: &AppSettings,
) -> Result<(), AppError> {
    for existing in &current.games {
        let Some(updated) = requested.games.iter().find(|game| game.id == existing.id) else {
            continue;
        };
        let old_key =
            crate::shared::path_key::folder_path_key(&existing.mod_path.to_string_lossy(), None);
        let new_key =
            crate::shared::path_key::folder_path_key(&updated.mod_path.to_string_lossy(), None);
        if old_key != new_key {
            return Err(AppError::Validation(
                "Use source recovery to change an existing game's mods directory".to_string(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn ensure_unique_game_ids(settings: &AppSettings) -> Result<(), AppError> {
    let mut game_ids = std::collections::HashSet::with_capacity(settings.games.len());
    for game in &settings.games {
        if !game_ids.insert(game.id.as_str()) {
            return Err(AppError::Validation(format!(
                "Settings contain duplicate game id '{}'",
                game.id
            )));
        }
    }
    Ok(())
}
