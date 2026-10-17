use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{Emitter, Manager};

use crate::modules::automation::application::hotkeys::manager::HotkeyGameContext;
use crate::modules::automation::application::hotkeys::notice::{
    unix_time_ms, HotkeyActionOutcome, HotkeyNotice, HOTKEY_NOTICE_LIFETIME_MS,
};
use crate::modules::reconciliation::api::disk_reconcile::types::{
    RuntimeSyncPhase, RuntimeSyncStatus,
};
use crate::modules::settings::application::config::{AppSettings, ConfigService};
use crate::modules::system::api::{focused_game_window, GameWindowGeometry};
use crate::shared::errors::AppError;
use crate::shared::sync::lock;

const NOTICE_EVENT: &str = "hotkey:notice";
const NOTICE_WINDOW_LABEL: &str = "hotkey-notice";
const NOTICE_WIDTH: f64 = 620.0;
const NOTICE_HEIGHT: f64 = 420.0;
const MAX_HIDE_RETRIES: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeHideOutcome {
    Failed,
    Hidden,
    Gone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowLifecycle {
    creation_token: String,
    ready: bool,
}

#[derive(Default)]
struct NoticeState {
    current: Option<HotkeyNotice>,
    shown_id: Option<String>,
    native_window: Option<NativeWindowLifecycle>,
    runtime_status: Option<RuntimeSyncStatus>,
}

#[derive(Default)]
pub struct HotkeyNoticeController {
    sequence: AtomicU64,
    state: Mutex<NoticeState>,
}

impl HotkeyNoticeController {
    pub(crate) fn build_notice(
        &self,
        context: &HotkeyGameContext,
        settings: &AppSettings,
        outcome: HotkeyActionOutcome,
        reload_binding: Option<String>,
    ) -> HotkeyNotice {
        let created_at_ms = unix_time_ms();
        let runtime_status = outcome.runtime_generation.and_then(|generation| {
            lock(&self.state)
                .runtime_status
                .as_ref()
                .filter(|status| {
                    status.game_id == context.game_id() && status.generation == generation
                })
                .cloned()
        });
        HotkeyNotice {
            id: uuid::Uuid::new_v4().to_string(),
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed) + 1,
            settings_revision: settings.revision,
            game_id: context.game_id().to_string(),
            game_name: context.game_name().to_string(),
            mods_path: context.mods_path().to_string_lossy().to_string(),
            root_identity: context.root_identity().to_string(),
            created_at_ms,
            expires_at_ms: created_at_ms.saturating_add(HOTKEY_NOTICE_LIFETIME_MS),
            kind: outcome.kind,
            previous_collection: outcome.previous_collection,
            collection: outcome.collection,
            safe_mode: outcome.safe_mode,
            message: outcome.message,
            runtime_generation: outcome.runtime_generation,
            runtime_status,
            reload_binding,
            hotkeys: settings.hotkeys.clone(),
            keyviewer_enabled: settings.keyviewer.enabled,
            language: settings.language.clone(),
            theme: settings.theme.clone(),
        }
    }

    pub(crate) fn publish(&self, app: &tauri::AppHandle, notice: HotkeyNotice) {
        let Some(config) = app.try_state::<ConfigService>() else {
            log::warn!("Hotkey notice was not published because settings are unavailable");
            return;
        };
        let settings = config.get_settings();
        if !notice_scope_is_current(&settings, &notice) {
            log::warn!(
                "Hotkey notice was not published because its selected game or mods root is no longer current"
            );
            return;
        }

        let previous_shown = {
            let mut state = lock(&self.state);
            let previous = state.shown_id.clone();
            state.current = Some(notice.clone());
            previous
        };
        if let Some(id) = previous_shown {
            hide_window_if_token(app.clone(), id);
        }
        if let Err(error) = app.emit_to("main", NOTICE_EVENT, &notice) {
            log::warn!("Could not emit hotkey notice to the main window: {error}");
        }

        if native_notice_geometry(&settings, &notice).is_none() {
            return;
        }

        if app.get_webview_window(NOTICE_WINDOW_LABEL).is_some() {
            if let Err(error) = app.emit_to(NOTICE_WINDOW_LABEL, NOTICE_EVENT, &notice) {
                log::warn!("Could not emit hotkey notice to the native overlay: {error}");
            }
            return;
        }
        create_notice_window(app.clone(), notice.id.clone());
    }

    fn current_valid(&self, settings: &AppSettings) -> Option<HotkeyNotice> {
        let notice = lock(&self.state).current.clone()?;
        notice_scope_is_current(settings, &notice).then_some(notice)
    }

    fn mark_shown(&self, id: &str) -> bool {
        let mut state = lock(&self.state);
        if state.current.as_ref().is_some_and(|notice| notice.id == id) {
            state.shown_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    fn record_native_hide_outcome(&self, id: &str, outcome: NativeHideOutcome) -> bool {
        let mut state = lock(&self.state);
        if state.shown_id.as_deref() != Some(id) || outcome == NativeHideOutcome::Failed {
            return false;
        }
        state.shown_id = None;
        if outcome == NativeHideOutcome::Gone {
            state.native_window = None;
        }
        true
    }

    fn is_shown(&self, id: &str) -> bool {
        lock(&self.state).shown_id.as_deref() == Some(id)
    }

    fn begin_native_window_creation(&self) -> String {
        let creation_token = uuid::Uuid::new_v4().to_string();
        lock(&self.state).native_window = Some(NativeWindowLifecycle {
            creation_token: creation_token.clone(),
            ready: false,
        });
        creation_token
    }

    fn mark_native_window_ready(&self) -> bool {
        let mut state = lock(&self.state);
        let Some(lifecycle) = state.native_window.as_mut() else {
            return false;
        };
        lifecycle.ready = true;
        true
    }

    fn clear_native_window_if(&self, creation_token: &str) -> bool {
        let mut state = lock(&self.state);
        let should_clear = state
            .native_window
            .as_ref()
            .is_some_and(|lifecycle| lifecycle.creation_token == creation_token);
        if should_clear {
            state.native_window = None;
        }
        should_clear
    }

    fn native_window_is_unready(&self, creation_token: &str) -> bool {
        lock(&self.state)
            .native_window
            .as_ref()
            .is_some_and(|lifecycle| lifecycle.creation_token == creation_token && !lifecycle.ready)
    }

    fn update_runtime(
        &self,
        settings: &AppSettings,
        status: &RuntimeSyncStatus,
    ) -> Option<HotkeyNotice> {
        if settings.active_game_id.as_deref() != Some(status.game_id.as_str()) {
            return None;
        }
        let mut state = lock(&self.state);
        let should_store = state.runtime_status.as_ref().is_none_or(|current| {
            current.game_id != status.game_id
                || status.generation > current.generation
                || (status.generation == current.generation
                    && runtime_phase_rank(&status.phase) >= runtime_phase_rank(&current.phase))
        });
        if !should_store {
            return None;
        }
        state.runtime_status = Some(status.clone());
        let notice = state.current.as_mut()?;
        if notice.game_id != status.game_id
            || notice.runtime_generation != Some(status.generation)
            || !notice_scope_is_current(settings, notice)
        {
            return None;
        }
        notice.runtime_status = Some(status.clone());
        Some(notice.clone())
    }
}

fn runtime_phase_rank(phase: &RuntimeSyncPhase) -> u8 {
    match phase {
        RuntimeSyncPhase::Queued => 0,
        RuntimeSyncPhase::Running => 1,
        RuntimeSyncPhase::Succeeded
        | RuntimeSyncPhase::NeedsManualReload
        | RuntimeSyncPhase::Failed => 2,
    }
}

pub(crate) fn update_runtime_status(app: &tauri::AppHandle, status: &RuntimeSyncStatus) {
    let Some(controller) = app.try_state::<HotkeyNoticeController>() else {
        return;
    };
    let Some(config) = app.try_state::<ConfigService>() else {
        return;
    };
    let Some(notice) = controller.update_runtime(&config.get_settings(), status) else {
        return;
    };
    if let Err(error) = app.emit_to("main", NOTICE_EVENT, &notice) {
        log::warn!("Could not enrich main hotkey notice with runtime status: {error}");
    }
    if app.get_webview_window(NOTICE_WINDOW_LABEL).is_some() {
        if let Err(error) = app.emit_to(NOTICE_WINDOW_LABEL, NOTICE_EVENT, &notice) {
            log::warn!("Could not enrich native hotkey notice with runtime status: {error}");
        }
    }
}

fn notice_scope_is_current(settings: &AppSettings, notice: &HotkeyNotice) -> bool {
    if notice.expires_at_ms <= unix_time_ms() {
        return false;
    }
    let Some(game) = settings.active_game() else {
        return false;
    };
    if game.id != notice.game_id || game.mod_path.to_string_lossy() != notice.mods_path {
        return false;
    }
    crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&game.mod_path)
        .is_ok_and(|proof| proof.identity() == notice.root_identity)
}

fn native_notice_geometry(
    settings: &AppSettings,
    notice: &HotkeyNotice,
) -> Option<GameWindowGeometry> {
    if !settings.hotkeys.enabled
        || !settings.hotkeys.game_notification_overlay_enabled
        || !notice_scope_is_current(settings, notice)
    {
        return None;
    }
    let game = settings.active_game()?;
    let process = crate::modules::games::api::executable_profiles::resolve_process_executable(
        game.game_type,
        game.game_exe.as_deref(),
    );
    focused_game_window(process.as_deref())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NoticeRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn notice_rect(game: GameWindowGeometry) -> NoticeRect {
    let desired_width = (NOTICE_WIDTH * game.scale_factor).round().max(1.0) as u32;
    let desired_height = (NOTICE_HEIGHT * game.scale_factor).round().max(1.0) as u32;
    let width = desired_width.min(game.width);
    let height = desired_height.min(game.height);
    let x = game
        .x
        .saturating_add((game.width.saturating_sub(width) / 2) as i32);
    let top_padding = (24.0 * game.scale_factor).round().max(0.0) as u32;
    let y = game
        .y
        .saturating_add(top_padding.min(game.height.saturating_sub(height)) as i32);
    NoticeRect {
        x,
        y,
        width,
        height,
    }
}

fn create_notice_window(app: tauri::AppHandle, expected_id: String) {
    let dispatch = app.clone();
    if let Err(error) = dispatch.run_on_main_thread(move || {
        let Some(controller) = app.try_state::<HotkeyNoticeController>() else {
            return;
        };
        let Some(config) = app.try_state::<ConfigService>() else {
            return;
        };
        let settings = config.get_settings();
        let Some(notice) = controller.current_valid(&settings) else {
            return;
        };
        if notice.id != expected_id || native_notice_geometry(&settings, &notice).is_none() {
            return;
        }
        if app.get_webview_window(NOTICE_WINDOW_LABEL).is_some() {
            if let Err(error) = app.emit_to(NOTICE_WINDOW_LABEL, NOTICE_EVENT, &notice) {
                log::warn!("Could not emit hotkey notice to existing native overlay: {error}");
            }
            return;
        }
        let creation_token = controller.begin_native_window_creation();
        let result = tauri::WebviewWindowBuilder::new(
            &app,
            NOTICE_WINDOW_LABEL,
            tauri::WebviewUrl::App("hotkey-notice.html".into()),
        )
        .title("EMMM hotkey notice")
        .inner_size(NOTICE_WIDTH, NOTICE_HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .always_on_top(true)
        .visible(false)
        .build();
        match result {
            Ok(window) => {
                let configured = configure_nonactivating_window(&window).and_then(|()| {
                    window
                        .set_ignore_cursor_events(true)
                        .map_err(|error| AppError::Internal(error.to_string()))
                });
                if let Err(error) = configured {
                    log::warn!(
                        "Native hotkey notice remains hidden because nonactivation/input pass-through could not be guaranteed: {error}"
                    );
                    match destroy_window_verified(&app, &window) {
                        Ok(()) => {
                            controller.clear_native_window_if(&creation_token);
                        }
                        Err(destroy_error) => {
                            log::error!(
                                "Could not destroy rejected native hotkey notice window: {destroy_error}; ownership is retained for retry"
                            );
                            start_unready_window_retirement(
                                app.clone(),
                                creation_token,
                                0,
                                Duration::from_millis(100),
                            );
                        }
                    }
                } else {
                    start_readiness_watchdog(app.clone(), creation_token);
                }
            }
            Err(error) => {
                controller.clear_native_window_if(&creation_token);
                log::warn!("Could not create native hotkey notice window: {error}");
            }
        }
    }) {
        log::warn!("Could not schedule native hotkey notice creation: {error}");
    }
}

fn start_readiness_watchdog(app: tauri::AppHandle, creation_token: String) {
    start_unready_window_retirement(
        app,
        creation_token,
        0,
        Duration::from_millis(HOTKEY_NOTICE_LIFETIME_MS),
    );
}

fn start_unready_window_retirement(
    app: tauri::AppHandle,
    creation_token: String,
    attempt: u8,
    delay: Duration,
) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        let dispatch = app.clone();
        if let Err(error) = dispatch.run_on_main_thread(move || {
            let Some(controller) = app.try_state::<HotkeyNoticeController>() else {
                return;
            };
            if !controller.native_window_is_unready(&creation_token) {
                return;
            }
            let Some(window) = app.get_webview_window(NOTICE_WINDOW_LABEL) else {
                controller.clear_native_window_if(&creation_token);
                return;
            };
            match destroy_window_verified(&app, &window) {
                Ok(()) => {
                    controller.clear_native_window_if(&creation_token);
                }
                Err(error) => {
                    log::error!(
                        "Could not destroy unready native hotkey notice window '{creation_token}': {error}; ownership is retained for retry"
                    );
                    if attempt < MAX_HIDE_RETRIES {
                        start_unready_window_retirement(
                            app.clone(),
                            creation_token.clone(),
                            attempt + 1,
                            Duration::from_millis(100),
                        );
                    }
                }
            }
        }) {
            log::warn!("Could not schedule unready native hotkey notice cleanup: {error}");
        }
    });
}

#[cfg(target_os = "windows")]
fn configure_nonactivating_window(window: &tauri::WebviewWindow) -> Result<(), AppError> {
    use windows_sys::Win32::Foundation::{GetLastError, SetLastError};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT,
    };
    let hwnd = window
        .hwnd()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let raw = hwnd.0 as windows_sys::Win32::Foundation::HWND;
    unsafe {
        let styles = GetWindowLongPtrW(raw, GWL_EXSTYLE);
        let required = WS_EX_NOACTIVATE as isize | WS_EX_TRANSPARENT as isize;
        SetLastError(0);
        let previous = SetWindowLongPtrW(raw, GWL_EXSTYLE, styles | required);
        let last_error = GetLastError();
        if previous == 0 && last_error != 0 {
            return Err(AppError::Internal(format!(
                "SetWindowLongPtrW failed: {}",
                std::io::Error::from_raw_os_error(last_error as i32)
            )));
        }
        let applied = GetWindowLongPtrW(raw, GWL_EXSTYLE);
        if applied & required != required {
            return Err(AppError::Internal(
                "Native notice window did not retain no-activate/click-through styles".to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn configure_nonactivating_window(_window: &tauri::WebviewWindow) -> Result<(), AppError> {
    Ok(())
}

fn show_notice_window(app: tauri::AppHandle, notice: HotkeyNotice) {
    let id = notice.id.clone();
    let dispatch = app.clone();
    if let Err(error) = dispatch.run_on_main_thread(move || {
        let Some(controller) = app.try_state::<HotkeyNoticeController>() else {
            return;
        };
        let Some(config) = app.try_state::<ConfigService>() else {
            return;
        };
        let settings = config.get_settings();
        let Some(current) = controller.current_valid(&settings) else {
            return;
        };
        if current.id != id {
            return;
        }
        let Some(geometry) = native_notice_geometry(&settings, &current) else {
            return;
        };
        let rect = notice_rect(geometry);
        if !controller.mark_shown(&id) {
            return;
        }
        let Some(window) = app.get_webview_window(NOTICE_WINDOW_LABEL) else {
            controller.record_native_hide_outcome(&id, NativeHideOutcome::Gone);
            return;
        };
        if let Err(error) = position_and_show_without_activation(&window, rect) {
            log::warn!("Could not position/show native hotkey notice: {error}");
            hide_window_if_token(app.clone(), id.clone());
            return;
        }
        start_visibility_timer(app.clone(), id.clone());
    }) {
        log::warn!("Could not schedule native hotkey notice display: {error}");
    }
}

fn start_visibility_timer(timer_app: tauri::AppHandle, timer_id: String) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let keep_showing = timer_app
                .try_state::<HotkeyNoticeController>()
                .zip(timer_app.try_state::<ConfigService>())
                .is_some_and(|(controller, config)| {
                    let settings = config.get_settings();
                    controller.current_valid(&settings).is_some_and(|current| {
                        current.id == timer_id
                            && native_notice_geometry(&settings, &current).is_some()
                    })
                });
            if !keep_showing {
                hide_window_if_token(timer_app.clone(), timer_id.clone());
                break;
            }
        }
    });
}

#[cfg(target_os = "windows")]
fn position_and_show_without_activation(
    window: &tauri::WebviewWindow,
    rect: NoticeRect,
) -> Result<(), AppError> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    };
    let hwnd = window
        .hwnd()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let shown = unsafe {
        SetWindowPos(
            hwnd.0 as windows_sys::Win32::Foundation::HWND,
            HWND_TOPMOST,
            rect.x,
            rect.y,
            rect.width.try_into().unwrap_or(i32::MAX),
            rect.height.try_into().unwrap_or(i32::MAX),
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    };
    if shown == 0 {
        return Err(AppError::Internal(format!(
            "SetWindowPos failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn position_and_show_without_activation(
    window: &tauri::WebviewWindow,
    rect: NoticeRect,
) -> Result<(), AppError> {
    window
        .set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
            rect.width,
            rect.height,
        )))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
            rect.x, rect.y,
        )))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    window
        .show()
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[cfg(target_os = "windows")]
fn hide_window_verified(window: &tauri::WebviewWindow) -> Result<NativeHideOutcome, AppError> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsWindow, IsWindowVisible, ShowWindow, SW_HIDE,
    };
    let hwnd = window
        .hwnd()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let raw = hwnd.0 as windows_sys::Win32::Foundation::HWND;
    unsafe {
        if IsWindow(raw) == 0 {
            return Ok(NativeHideOutcome::Gone);
        }
        ShowWindow(raw, SW_HIDE);
        if IsWindowVisible(raw) != 0 {
            return Err(AppError::Internal(
                "SW_HIDE returned but the native notice window is still visible".to_string(),
            ));
        }
    }
    Ok(NativeHideOutcome::Hidden)
}

#[cfg(not(target_os = "windows"))]
fn hide_window_verified(window: &tauri::WebviewWindow) -> Result<NativeHideOutcome, AppError> {
    window
        .hide()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    match window.is_visible() {
        Ok(false) => Ok(NativeHideOutcome::Hidden),
        Ok(true) => Err(AppError::Internal(
            "Native notice window remained visible after hide".to_string(),
        )),
        Err(error) => Err(AppError::Internal(error.to_string())),
    }
}

#[cfg(target_os = "windows")]
fn destroy_window_verified(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
) -> Result<(), AppError> {
    use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;
    let hwnd = window
        .hwnd()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let raw = hwnd.0 as windows_sys::Win32::Foundation::HWND;
    window
        .destroy()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let retired =
        unsafe { IsWindow(raw) == 0 } && app.get_webview_window(NOTICE_WINDOW_LABEL).is_none();
    if retired {
        Ok(())
    } else {
        Err(AppError::Internal(
            "Native notice window remained registered after destroy".to_string(),
        ))
    }
}

#[cfg(not(target_os = "windows"))]
fn destroy_window_verified(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
) -> Result<(), AppError> {
    window
        .destroy()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    if app.get_webview_window(NOTICE_WINDOW_LABEL).is_none() {
        Ok(())
    } else {
        Err(AppError::Internal(
            "Native notice window remained registered after destroy".to_string(),
        ))
    }
}

fn hide_window_if_token(app: tauri::AppHandle, id: String) {
    hide_window_if_token_attempt(app, id, 0);
}

fn hide_window_if_token_attempt(app: tauri::AppHandle, id: String, attempt: u8) {
    let dispatch = app.clone();
    if let Err(error) = dispatch.run_on_main_thread(move || {
        let Some(controller) = app.try_state::<HotkeyNoticeController>() else {
            return;
        };
        if !controller.is_shown(&id) {
            return;
        }
        let Some(window) = app.get_webview_window(NOTICE_WINDOW_LABEL) else {
            controller.record_native_hide_outcome(&id, NativeHideOutcome::Gone);
            return;
        };
        match hide_window_verified(&window) {
            Ok(outcome) => {
                controller.record_native_hide_outcome(&id, outcome);
            }
            Err(hide_error) => match destroy_window_verified(&app, &window) {
                Ok(()) => {
                    controller.record_native_hide_outcome(&id, NativeHideOutcome::Gone);
                }
                Err(destroy_error) => {
                    controller.record_native_hide_outcome(&id, NativeHideOutcome::Failed);
                    log::error!(
                        "Native hotkey notice '{id}' could not be hidden ({hide_error}) or destroyed ({destroy_error}); ownership is retained for retry"
                    );
                    if attempt < MAX_HIDE_RETRIES {
                        let retry_app = app.clone();
                        let retry_id = id.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            hide_window_if_token_attempt(retry_app, retry_id, attempt + 1);
                        });
                    }
                }
            }
        }
    }) {
        log::warn!("Could not schedule native hotkey notice hide: {error}");
    }
}

#[specta::specta]
#[tauri::command]
pub fn get_hotkey_notice(
    controller: tauri::State<'_, HotkeyNoticeController>,
    config: tauri::State<'_, ConfigService>,
) -> Result<Option<HotkeyNotice>, AppError> {
    Ok(controller.current_valid(&config.get_settings()))
}

#[specta::specta]
#[tauri::command]
pub fn hotkey_notice_ready(
    id: String,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    controller: tauri::State<'_, HotkeyNoticeController>,
    config: tauri::State<'_, ConfigService>,
) -> Result<(), AppError> {
    if window.label() != NOTICE_WINDOW_LABEL {
        return Err(AppError::Validation(
            "Hotkey notice readiness is restricted to the notice window".to_string(),
        ));
    }
    let settings = config.get_settings();
    let notice = controller
        .current_valid(&settings)
        .filter(|notice| notice.id == id)
        .ok_or_else(|| AppError::Validation("Hotkey notice is no longer current".to_string()))?;
    if controller.is_shown(&id) {
        return Ok(());
    }
    native_notice_geometry(&settings, &notice)
        .ok_or_else(|| AppError::Validation("Hotkey notice cannot be shown now".to_string()))?;
    if !controller.mark_native_window_ready() {
        return Err(AppError::Validation(
            "Hotkey notice window does not have an active creation token".to_string(),
        ));
    }
    show_notice_window(app, notice);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        notice_rect, notice_scope_is_current, GameWindowGeometry, HotkeyNoticeController,
        NativeHideOutcome, NoticeRect,
    };
    use crate::modules::automation::application::hotkeys::manager::HotkeyGameContext;
    use crate::modules::automation::application::hotkeys::notice::{
        HotkeyActionOutcome, HotkeyNoticeKind,
    };
    use crate::modules::games::domain::models::GameType;
    use crate::modules::reconciliation::api::disk_reconcile::types::{
        RuntimeSyncPhase, RuntimeSyncReloadStatus, RuntimeSyncStatus,
    };
    use crate::modules::settings::application::config::{AppSettings, GameConfig};
    use crate::shared::sync::lock;
    use std::path::PathBuf;

    fn test_settings(mods_path: PathBuf) -> AppSettings {
        AppSettings {
            games: vec![GameConfig {
                id: "game".to_string(),
                name: "Test Game".to_string(),
                game_type: GameType::Gimi,
                instance_path: mods_path.clone(),
                mod_path: mods_path,
                ready_to_move_path: None,
                launch_mode: Default::default(),
                game_exe: Some(PathBuf::from("C:/Games/Test.exe")),
                loader_exe: None,
                xxmi_launcher_exe: None,
                launch_args: None,
                warnings: Vec::new(),
            }],
            active_game_id: Some("game".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn notice_geometry_is_scaled_centered_and_bounded_by_the_game_client() {
        assert_eq!(
            notice_rect(GameWindowGeometry {
                x: 100,
                y: 40,
                width: 1920,
                height: 1080,
                scale_factor: 1.5,
            }),
            NoticeRect {
                x: 595,
                y: 76,
                width: 930,
                height: 630,
            }
        );
        assert_eq!(
            notice_rect(GameWindowGeometry {
                x: -800,
                y: 0,
                width: 480,
                height: 240,
                scale_factor: 1.0,
            }),
            NoticeRect {
                x: -800,
                y: 0,
                width: 480,
                height: 240,
            }
        );
    }

    #[test]
    fn transition_payload_retains_original_scope_and_same_collection_application() {
        let root = tempfile::tempdir().unwrap();
        let settings = test_settings(root.path().to_path_buf());
        let context = HotkeyGameContext::capture(&settings).unwrap();
        let controller = HotkeyNoticeController::default();
        let notice = controller.build_notice(
            &context,
            &settings,
            HotkeyActionOutcome {
                kind: HotkeyNoticeKind::CollectionApplied,
                previous_collection: Some("Only preset".to_string()),
                collection: Some("Only preset".to_string()),
                safe_mode: true,
                message: None,
                runtime_generation: Some(42),
                log_summary: "applied".to_string(),
            },
            Some("Ctrl+F10".to_string()),
        );

        assert_eq!(notice.kind, HotkeyNoticeKind::CollectionApplied);
        assert_eq!(notice.previous_collection, notice.collection);
        assert_eq!(notice.runtime_generation, Some(42));
        assert_eq!(notice.game_id, "game");
        assert_eq!(notice.mods_path, root.path().to_string_lossy());
        assert_eq!(notice.reload_binding.as_deref(), Some("Ctrl+F10"));
        assert_eq!(notice.sequence, 1);
        assert_eq!(notice.settings_revision, settings.revision);
    }

    #[test]
    fn initial_publication_scope_rejects_a_recreated_mods_root() {
        let parent = tempfile::tempdir().unwrap();
        let mods_path = parent.path().join("Mods");
        std::fs::create_dir(&mods_path).unwrap();
        let mut settings = test_settings(mods_path.clone());
        let context = HotkeyGameContext::capture(&settings).unwrap();
        let controller = HotkeyNoticeController::default();
        let notice = controller.build_notice(
            &context,
            &settings,
            HotkeyActionOutcome {
                kind: HotkeyNoticeKind::CollectionApplied,
                previous_collection: Some("Before".to_string()),
                collection: Some("After".to_string()),
                safe_mode: false,
                message: None,
                runtime_generation: Some(7),
                log_summary: "applied".to_string(),
            },
            None,
        );

        settings.hotkeys.enabled = false;
        assert!(notice_scope_is_current(&settings, &notice));

        std::fs::rename(&mods_path, parent.path().join("Old Mods")).unwrap();
        std::fs::create_dir(&mods_path).unwrap();
        assert!(!notice_scope_is_current(&settings, &notice));
    }

    #[test]
    fn getter_guard_filters_expired_foreign_and_superseded_receipts() {
        let first_root = tempfile::tempdir().unwrap();
        let second_root = tempfile::tempdir().unwrap();
        let settings = test_settings(first_root.path().to_path_buf());
        let context = HotkeyGameContext::capture(&settings).unwrap();
        let controller = HotkeyNoticeController::default();
        let outcome = HotkeyActionOutcome {
            kind: HotkeyNoticeKind::NoPresets,
            previous_collection: None,
            collection: None,
            safe_mode: false,
            message: Some("none".to_string()),
            runtime_generation: None,
            log_summary: "none".to_string(),
        };
        let first = controller.build_notice(&context, &settings, outcome.clone(), None);
        let mut second = controller.build_notice(&context, &settings, outcome, None);
        assert!(second.sequence > first.sequence);
        lock(&controller.state).current = Some(second.clone());
        assert_eq!(
            controller.current_valid(&settings).map(|notice| notice.id),
            Some(second.id.clone())
        );

        second.expires_at_ms = 0;
        lock(&controller.state).current = Some(second);
        assert!(controller.current_valid(&settings).is_none());

        let foreign = test_settings(second_root.path().to_path_buf());
        lock(&controller.state).current = Some(first);
        assert!(controller.current_valid(&foreign).is_none());
    }

    #[test]
    fn failed_hide_retains_ownership_until_success_and_stale_ids_cannot_retire_successors() {
        let controller = HotkeyNoticeController::default();
        {
            let mut state = lock(&controller.state);
            state.shown_id = Some("old".to_string());
        }
        assert!(!controller.record_native_hide_outcome("old", NativeHideOutcome::Failed));
        assert_eq!(lock(&controller.state).shown_id.as_deref(), Some("old"));
        assert!(controller.record_native_hide_outcome("old", NativeHideOutcome::Hidden));
        assert!(lock(&controller.state).shown_id.is_none());

        {
            let mut state = lock(&controller.state);
            state.shown_id = Some("new".to_string());
        }
        assert!(!controller.record_native_hide_outcome("old", NativeHideOutcome::Hidden));
        assert_eq!(lock(&controller.state).shown_id.as_deref(), Some("new"));
    }

    #[test]
    fn unready_creation_can_expire_and_allow_the_next_creation() {
        let controller = HotkeyNoticeController::default();
        let failed = controller.begin_native_window_creation();
        assert!(controller.native_window_is_unready(&failed));
        assert!(controller.clear_native_window_if(&failed));

        let successor = controller.begin_native_window_creation();
        assert_ne!(successor, failed);
        assert!(controller.mark_native_window_ready());
        assert!(!controller.native_window_is_unready(&successor));
    }

    #[test]
    fn stale_creation_watchdog_cannot_close_a_ready_successor() {
        let controller = HotkeyNoticeController::default();
        let stale = controller.begin_native_window_creation();
        let successor = controller.begin_native_window_creation();
        assert!(controller.mark_native_window_ready());

        assert!(!controller.native_window_is_unready(&stale));
        assert!(!controller.native_window_is_unready(&successor));
    }

    #[test]
    fn runtime_completion_before_publish_is_attached_and_stale_generations_are_ignored() {
        let root = tempfile::tempdir().unwrap();
        let settings = test_settings(root.path().to_path_buf());
        let context = HotkeyGameContext::capture(&settings).unwrap();
        let controller = HotkeyNoticeController::default();
        let completed = RuntimeSyncStatus {
            game_id: "game".to_string(),
            generation: 42,
            phase: RuntimeSyncPhase::Succeeded,
            cause: "collection_applied".to_string(),
            message: None,
            reload_status: Some(RuntimeSyncReloadStatus::Sent),
            reload_binding: Some("F10".to_string()),
        };
        assert!(controller.update_runtime(&settings, &completed).is_none());
        let outcome = HotkeyActionOutcome {
            kind: HotkeyNoticeKind::CollectionApplied,
            previous_collection: Some("Before".to_string()),
            collection: Some("After".to_string()),
            safe_mode: false,
            message: None,
            runtime_generation: Some(42),
            log_summary: "applied".to_string(),
        };
        let notice = controller.build_notice(&context, &settings, outcome, None);
        assert_eq!(notice.runtime_status, Some(completed.clone()));
        lock(&controller.state).current = Some(notice.clone());

        let stale = RuntimeSyncStatus {
            generation: 41,
            phase: RuntimeSyncPhase::Failed,
            message: Some("stale".to_string()),
            reload_status: None,
            reload_binding: None,
            ..completed.clone()
        };
        assert!(controller.update_runtime(&settings, &stale).is_none());
        assert_eq!(
            lock(&controller.state)
                .current
                .as_ref()
                .and_then(|notice| notice.runtime_status.clone()),
            Some(completed)
        );
    }
}
