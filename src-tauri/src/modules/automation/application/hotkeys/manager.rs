//! HotkeyManager — bridges OS-level global hotkeys to action planners.
//!
//! Owns global shortcut registration lifecycle through `tauri-plugin-global-shortcut`,
//! and dispatches events to action planners.
//!
//! **Threading model:**
//! - Registration/unregistration happens through Tauri plugin APIs.
//! - Event listening is callback-driven via plugin handler (configured in `lib.rs`).
//! - `HotkeyState` (debounce/switch_lock) is protected by `Mutex`.

use crate::shared::errors::AppError;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::modules::settings::application::config::{AppSettings, ConfigService};
use crate::shared::sync::lock;

use super::cycle_preset::{execute_cycle_preset, CycleDirection};
use super::focus;
use super::notice::{HotkeyActionOutcome, HotkeyNoticeKind};
use super::{get_key_string, HotkeyAction, HotkeyConfig, HotkeyState};

// ─── Key Parsing ─────────────────────────────────────────────────────────────

/// Parse and normalize a user-facing key string (e.g. "F5", "Shift+F6").
pub fn parse_hotkey(key_str: &str) -> Result<String, AppError> {
    Ok(parse_os_shortcut(key_str)?.to_string())
}

fn parse_os_shortcut(key_str: &str) -> Result<Shortcut, AppError> {
    let normalized = super::validate_3dmigoto_binding(key_str)?;
    if normalized.is_empty() {
        return Err(AppError::Internal("Hotkey cannot be empty".to_string()));
    }

    // `normalize_shortcut` already stripped every space, so a token is empty
    // only when the string is all separators ("+", "++", …).
    if normalized.split('+').all(str::is_empty) {
        return Err(AppError::Internal(format!("Invalid hotkey '{key_str}'")));
    }

    if normalized
        .split('+')
        .any(|token| matches!(token, "no_ctrl" | "no_shift" | "no_alt"))
    {
        return Err(AppError::Validation(
            "OS hotkeys cannot use NO_CTRL, NO_SHIFT, or NO_ALT modifiers".to_string(),
        ));
    }

    normalized
        .parse::<Shortcut>()
        .map_err(|error| AppError::Validation(format!("Invalid OS hotkey '{key_str}': {error}")))
}

/// The one spelling of "how a shortcut string is canonicalized". Registration
/// and keystroke replay (`reload.rs`) must agree, or we register a shortcut we
/// cannot send.
pub fn normalize_shortcut(key_str: &str) -> String {
    key_str.trim().replace(' ', "").to_ascii_lowercase()
}

/// `Some` for the two preset-cycling actions, `None` for everything else.
fn preset_cycle_direction(action: HotkeyAction) -> Option<CycleDirection> {
    match action {
        HotkeyAction::NextPreset => Some(CycleDirection::Next),
        HotkeyAction::PrevPreset => Some(CycleDirection::Previous),
        _ => None,
    }
}

async fn publish_action_notice(
    app: &tauri::AppHandle,
    context: &HotkeyGameContext,
    press_settings: &AppSettings,
    result: &Result<HotkeyActionOutcome, AppError>,
) {
    let settings = app
        .try_state::<ConfigService>()
        .map(|config| config.get_settings())
        .unwrap_or_else(|| press_settings.clone());
    let outcome = match result {
        Ok(outcome) => outcome.clone(),
        Err(error) => HotkeyActionOutcome {
            kind: HotkeyNoticeKind::Failed,
            previous_collection: None,
            collection: None,
            safe_mode: settings.safety.runtime_safe_mode_for(context.game_id()),
            message: Some(error.to_string()),
            runtime_generation: None,
            log_summary: String::new(),
        },
    };
    let game = settings
        .games
        .iter()
        .find(|game| game.id == context.game_id() && game.mod_path == context.mods_path())
        .cloned();
    let reload_binding = match tokio::task::spawn_blocking(move || {
        game.and_then(|game| {
            crate::modules::automation::application::keyviewer::generator::discover_reload_key_for_game(&game)
                .ok()
                .map(|config| config.reload_config_key)
        })
    })
    .await
    {
        Ok(binding) => binding,
        Err(error) => {
            log::warn!("Could not resolve reload binding for hotkey notice: {error}");
            None
        }
    };

    let Some(controller) = app.try_state::<
        crate::modules::automation::adapters::tauri::hotkey_notice::HotkeyNoticeController,
    >() else {
        log::warn!("Hotkey notice controller is unavailable");
        return;
    };
    let notice = controller.build_notice(context, &settings, outcome, reload_binding);
    controller.publish(app, notice);
}

// ─── Registration Map ────────────────────────────────────────────────────────

type HotkeyMap = HashMap<u32, (Shortcut, HotkeyAction)>;

pub(crate) trait ShortcutRegistrationSink {
    fn unregister_all(&mut self) -> Result<(), AppError>;
    fn register(&mut self, shortcut: Shortcut) -> Result<(), AppError>;
    fn unregister(&mut self, shortcut: Shortcut) -> Result<(), AppError>;
}

pub(crate) struct TauriShortcutRegistrationSink<'a> {
    app: &'a tauri::AppHandle,
}

impl<'a> TauriShortcutRegistrationSink<'a> {
    pub(crate) fn new(app: &'a tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl ShortcutRegistrationSink for TauriShortcutRegistrationSink<'_> {
    fn unregister_all(&mut self) -> Result<(), AppError> {
        self.app
            .global_shortcut()
            .unregister_all()
            .map_err(Into::into)
    }

    fn register(&mut self, shortcut: Shortcut) -> Result<(), AppError> {
        self.app
            .global_shortcut()
            .register(shortcut)
            .map_err(Into::into)
    }

    fn unregister(&mut self, shortcut: Shortcut) -> Result<(), AppError> {
        self.app
            .global_shortcut()
            .unregister(shortcut)
            .map_err(Into::into)
    }
}

/// Build a map of (shortcut string, HotkeyAction) from the user config.
fn build_registration(config: &HotkeyConfig) -> Result<Vec<(Shortcut, HotkeyAction)>, AppError> {
    let entries: Vec<(Shortcut, HotkeyAction)> = HotkeyAction::OS_ACTIONS
        .into_iter()
        .map(|action| Ok((parse_os_shortcut(get_key_string(config, action))?, action)))
        .collect::<Result<_, AppError>>()?;
    Ok(entries)
}

#[derive(Clone)]
pub(crate) struct HotkeyGameContext {
    game_id: String,
    game_name: String,
    mods_path: PathBuf,
    root_proof: crate::platform::fs::file_utils::FilesystemIdentityProof,
}

impl HotkeyGameContext {
    pub(crate) fn capture(
        settings: &crate::modules::settings::application::config::AppSettings,
    ) -> Result<Self, AppError> {
        let game = settings
            .active_game()
            .ok_or_else(|| AppError::Validation("No active game selected".to_string()))?;
        Ok(Self {
            game_id: game.id.clone(),
            game_name: game.name.clone(),
            mods_path: game.mod_path.clone(),
            root_proof: crate::platform::fs::file_utils::FilesystemIdentityProof::capture(
                &game.mod_path,
            )?,
        })
    }

    pub(crate) fn game_id(&self) -> &str {
        &self.game_id
    }

    pub(crate) fn mods_path(&self) -> &Path {
        &self.mods_path
    }

    pub(crate) fn game_name(&self) -> &str {
        &self.game_name
    }

    pub(crate) fn root_identity(&self) -> &str {
        self.root_proof.identity()
    }

    pub(super) fn validate(
        &self,
        settings: &crate::modules::settings::application::config::AppSettings,
    ) -> Result<(), AppError> {
        let current = settings.active_game().ok_or_else(|| {
            AppError::Validation("Hotkey canceled because the selected game changed".to_string())
        })?;
        if !settings.hotkeys.enabled
            || current.id != self.game_id
            || current.mod_path != self.mods_path
        {
            return Err(AppError::Validation(
                "Hotkey canceled because hotkeys were disabled or the selected game changed"
                    .to_string(),
            ));
        }
        self.root_proof.validate(&self.mods_path)
    }
}

pub(crate) fn validate_overlay_replay_settings(
    original: &AppSettings,
    current: &AppSettings,
) -> Result<(), AppError> {
    let same_game = original
        .active_game()
        .zip(current.active_game())
        .is_some_and(|(before, after)| {
            before.id == after.id
                && before.game_type == after.game_type
                && before.game_exe == after.game_exe
        });
    if !current.hotkeys.enabled
        || !same_game
        || parse_os_shortcut(&original.hotkeys.toggle_overlay)?
            != parse_os_shortcut(&current.hotkeys.toggle_overlay)?
    {
        return Err(AppError::Validation(
            "Overlay canceled because its game or hotkey configuration changed".into(),
        ));
    }
    Ok(())
}

// ─── HotkeyManager ──────────────────────────────────────────────────────────

/// Managed Tauri state — owns OS hotkey lifecycle.
pub struct HotkeyManager {
    registration_lifecycle: Mutex<()>,
    /// Map from normalized shortcut string → action enum. Non-empty exactly
    /// while shortcuts are registered, so it also answers `is_enabled`.
    key_map: Mutex<HotkeyMap>,
    /// Debounce / switch-lock state.
    state: Mutex<HotkeyState>,
    /// One-shot release signals for OS shortcuts that will later synthesize a
    /// 3DMigoto reload chord. The signal is armed before the async work starts
    /// so a quick key release cannot be lost.
    release_waiters: Mutex<HashMap<u32, Arc<tokio::sync::Notify>>>,
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyManager {
    /// Create a new HotkeyManager.
    pub fn new() -> Self {
        Self {
            registration_lifecycle: Mutex::new(()),
            key_map: Mutex::new(HashMap::new()),
            state: Mutex::new(HotkeyState::new()),
            release_waiters: Mutex::new(HashMap::new()),
        }
    }

    /// Update shortcuts after settings change, restoring the old registration
    /// if the OS rejects any part of the replacement set.
    pub fn update_bindings(
        &self,
        app: &tauri::AppHandle,
        config: &HotkeyConfig,
    ) -> Result<(), AppError> {
        let lifecycle = self.registration_lifecycle();
        let mut sink = TauriShortcutRegistrationSink::new(app);
        self.update_bindings_with_sink(&lifecycle, &mut sink, config)
    }

    pub(crate) fn registration_lifecycle(&self) -> MutexGuard<'_, ()> {
        lock(&self.registration_lifecycle)
    }

    pub(crate) fn update_bindings_with_sink(
        &self,
        _lifecycle: &MutexGuard<'_, ()>,
        sink: &mut impl ShortcutRegistrationSink,
        config: &HotkeyConfig,
    ) -> Result<(), AppError> {
        validate_binding_configuration(config)?;
        let entries = config
            .enabled
            .then(|| build_registration(config))
            .transpose()?;
        let previous: Vec<(Shortcut, HotkeyAction)> =
            lock(&self.key_map).values().copied().collect();
        sink.unregister_all()?;

        let registration = entries.as_deref().unwrap_or_default();
        let mut registered = HashMap::new();
        for (shortcut, action) in registration {
            if let Err(error) = sink.register(*shortcut) {
                let _ = sink.unregister_all();
                let mut restored = HashMap::new();
                for (old_shortcut, old_action) in &previous {
                    if let Err(restore_error) = sink.register(*old_shortcut) {
                        log::error!(
                            "Could not restore global hotkey '{old_shortcut}' after failed update: {restore_error}"
                        );
                        continue;
                    }
                    restored.insert(old_shortcut.id(), (*old_shortcut, *old_action));
                }
                *lock(&self.key_map) = restored;
                return Err(error);
            }
            registered.insert(shortcut.id(), (*shortcut, *action));
        }
        *lock(&self.key_map) = registered;

        log::info!("Registered {} global shortcuts", registration.len());

        Ok(())
    }

    /// Check if the manager is currently enabled and listening.
    pub fn is_enabled(&self) -> bool {
        !lock(&self.key_map).is_empty()
    }

    /// Look up which action corresponds to a shortcut string.
    pub fn lookup_action(&self, shortcut: &str) -> Option<HotkeyAction> {
        let shortcut = parse_os_shortcut(shortcut).ok()?;
        lock(&self.key_map)
            .get(&shortcut.id())
            .map(|(_, action)| *action)
    }

    /// Temporarily release one OS shortcut before replaying the same key into
    /// the game. Without this pause, the synthetic KeyViewer toggle would be
    /// intercepted by this manager again instead of reaching 3DMigoto.
    fn suspend_shortcut_with_sink(
        &self,
        _lifecycle: &MutexGuard<'_, ()>,
        sink: &mut impl ShortcutRegistrationSink,
        shortcut: &str,
    ) -> Result<(), AppError> {
        sink.unregister(parse_os_shortcut(shortcut)?)
    }

    pub(crate) fn replay_overlay_with_sink(
        &self,
        lifecycle: &MutexGuard<'_, ()>,
        sink: &mut impl ShortcutRegistrationSink,
        original: &AppSettings,
        current: impl Fn() -> AppSettings,
        replay: impl FnOnce(&AppSettings, &str) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        let settings = current();
        validate_overlay_replay_settings(original, &settings)?;
        let binding = &original.hotkeys.toggle_overlay;
        self.suspend_shortcut_with_sink(lifecycle, sink, binding)?;
        let replay_result = replay(&settings, binding);
        let restore_result = self.update_bindings_with_sink(lifecycle, sink, &current().hotkeys);
        match (replay_result, restore_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Err(replay), Err(restore)) => Err(AppError::Internal(format!(
                "Overlay replay failed ({replay}); hotkey restoration also failed ({restore})"
            ))),
        }
    }

    /// Try to acquire the action lock (debounce + switch_lock).
    pub fn try_acquire(&self) -> bool {
        lock(&self.state).try_acquire()
    }

    /// Release the action lock after an action completes.
    pub fn release(&self) {
        lock(&self.state).release();
    }

    fn arm_shortcut_release(&self, shortcut: &str) -> Arc<tokio::sync::Notify> {
        let waiter = Arc::new(tokio::sync::Notify::new());
        if let Ok(shortcut) = parse_os_shortcut(shortcut) {
            lock(&self.release_waiters).insert(shortcut.id(), Arc::clone(&waiter));
        }
        waiter
    }

    /// Called by the global-shortcut handler for a release event.
    pub fn on_shortcut_released(&self, shortcut: &str) {
        let Ok(shortcut) = parse_os_shortcut(shortcut) else {
            return;
        };
        if let Some(waiter) = lock(&self.release_waiters).remove(&shortcut.id()) {
            // `notify_one` keeps one permit if the task has not started
            // awaiting yet, avoiding a race with a fast key release.
            waiter.notify_one();
        }
    }

    async fn wait_for_shortcut_release(waiter: Arc<tokio::sync::Notify>) -> Result<(), AppError> {
        tokio::time::timeout(Duration::from_millis(1_000), waiter.notified())
            .await
            .map_err(|_| {
                AppError::Validation(
                    "NeedsManualReload: triggering shortcut was not released".to_string(),
                )
            })
    }

    /// Called by plugin event handler when a shortcut is pressed.
    pub fn on_shortcut_pressed(&self, app: &tauri::AppHandle, shortcut: &str) {
        if !self.is_enabled() {
            return;
        }

        let action = match self.lookup_action(shortcut) {
            Some(action) => action,
            None => return,
        };

        let Some(config_state) = app.try_state::<ConfigService>() else {
            log::warn!("Hotkey ignored: ConfigService is unavailable");
            return;
        };

        let settings = config_state.get_settings();
        let telemetry =
            crate::modules::system::application::telemetry::capture_background_telemetry(app);
        if !settings.hotkeys.enabled {
            return;
        }

        // Safe Mode and presets mutate the selected game's workspace globally.
        // Overlay toggling replays input, so only that action requires game focus.
        if action == HotkeyAction::ToggleOverlay && !focus::is_active_game_focused(&settings) {
            return;
        }
        if let Some(direction) = preset_cycle_direction(action) {
            let context = match HotkeyGameContext::capture(&settings) {
                Ok(context) => context,
                Err(error) => {
                    log::warn!("Preset cycle hotkey ignored: {error}");
                    return;
                }
            };
            if !self.try_acquire() {
                log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
                return;
            }

            let app_handle = app.clone();
            let notice_settings = settings.clone();
            let release_waiter = self.arm_shortcut_release(shortcut);
            tauri::async_runtime::spawn(async move {
                let result = match Self::wait_for_shortcut_release(release_waiter).await {
                    Ok(()) => execute_cycle_preset(&app_handle, direction, &context).await,
                    Err(error) => Err(error),
                };
                publish_action_notice(&app_handle, &context, &notice_settings, &result).await;
                match result {
                    Ok(outcome) => log::info!("Hotkey {:?} → {}", action, outcome.log_summary),
                    Err(error) => {
                        crate::modules::system::application::telemetry::record_background_failure(
                            &telemetry, &error,
                        )
                        .await;
                        log::error!("Preset cycle hotkey {:?} failed: {error}", action);
                    }
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });

            return;
        }

        if action == HotkeyAction::ToggleSafeMode {
            let context = match HotkeyGameContext::capture(&settings) {
                Ok(context) => context,
                Err(error) => {
                    log::warn!("Safe Mode hotkey ignored: {error}");
                    return;
                }
            };
            if !self.try_acquire() {
                log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
                return;
            }

            let app_handle = app.clone();
            let notice_settings = settings.clone();
            let release_waiter = self.arm_shortcut_release(shortcut);
            tauri::async_runtime::spawn(async move {
                let result = match Self::wait_for_shortcut_release(release_waiter).await {
                    Ok(()) => {
                        super::safe_mode::execute_toggle_safe_mode(&app_handle, &context).await
                    }
                    Err(error) => Err(error),
                };
                publish_action_notice(&app_handle, &context, &notice_settings, &result).await;
                match result {
                    Ok(outcome) => log::info!("Hotkey {:?} → {}", action, outcome.log_summary),
                    Err(error) => {
                        crate::modules::system::application::telemetry::record_background_failure(
                            &telemetry, &error,
                        )
                        .await;
                        log::error!("Safe Mode hotkey failed: {error}");
                    }
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });
            return;
        }

        if action == HotkeyAction::ToggleOverlay {
            if !self.try_acquire() {
                log::debug!("Hotkey {:?} dropped (debounce/lock)", action);
                return;
            }

            let app_handle = app.clone();
            let release_waiter = self.arm_shortcut_release(shortcut);
            let binding =
                get_key_string(&settings.hotkeys, HotkeyAction::ToggleOverlay).to_string();
            tauri::async_runtime::spawn(async move {
                let released = async {
                    Self::wait_for_shortcut_release(release_waiter).await?;
                    super::reload::wait_for_input_release(&binding).await
                }
                .await;
                let result = match released {
                    Ok(()) => {
                        let suspend_result = match app_handle.try_state::<HotkeyManager>() {
                            Some(hotkey_manager) => {
                                let lifecycle = hotkey_manager.inner().registration_lifecycle();
                                let mut sink = TauriShortcutRegistrationSink::new(&app_handle);
                                match app_handle.try_state::<ConfigService>() {
                                    Some(config_state) => {
                                        hotkey_manager.inner().replay_overlay_with_sink(
                                            &lifecycle,
                                            &mut sink,
                                            &settings,
                                            || config_state.get_settings(),
                                            super::reload::trigger_binding_while_focused,
                                        )
                                    }
                                    None => Err(AppError::Internal(
                                        "ConfigService is unavailable".into(),
                                    )),
                                }
                            }
                            None => Err(AppError::Internal(
                                "HotkeyManager is unavailable".to_string(),
                            )),
                        };
                        suspend_result
                    }
                    Err(error) => Err(error),
                };
                match result {
                    Ok(()) => log::info!("Hotkey {:?} replayed to the active game", action),
                    Err(error) => {
                        crate::modules::system::application::telemetry::record_background_failure(
                            &telemetry, &error,
                        )
                        .await;
                        log::error!("Overlay hotkey failed: {error}");
                    }
                }

                if let Some(hotkey_manager) = app_handle.try_state::<HotkeyManager>() {
                    hotkey_manager.inner().release();
                }
            });
        }
    }
}

/// Validate every binding exposed in Settings and registered with the OS.
pub(crate) fn validate_binding_configuration(config: &HotkeyConfig) -> Result<(), AppError> {
    let mut configured = HashSet::new();
    for action in HotkeyAction::ALL {
        let shortcut = parse_os_shortcut(get_key_string(config, action))?;
        if !configured.insert(shortcut.id()) {
            return Err(AppError::Validation(
                "Hotkey bindings must not use the same shortcut".to_string(),
            ));
        }
    }
    Ok(())
}
