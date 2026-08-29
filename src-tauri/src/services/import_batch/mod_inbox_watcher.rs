use crate::common::sync::lock;
use crate::domain::errors::AppError;
use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use tauri::Emitter;

type InboxWatcher = Debouncer<notify::RecommendedWatcher, RecommendedCache>;

struct ActiveInboxWatcher {
    game_id: String,
    _watcher: InboxWatcher,
}

pub struct ModInboxWatcherState {
    active: Mutex<Option<ActiveInboxWatcher>>,
}

impl ModInboxWatcherState {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
        }
    }
}

impl Default for ModInboxWatcherState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModInboxChangedPayload {
    pub game_id: String,
    pub root_path: String,
    pub error: Option<String>,
}

pub fn start(
    app: tauri::AppHandle,
    state: &ModInboxWatcherState,
    game_id: &str,
    root: &Path,
) -> Result<(), AppError> {
    if !root.is_dir() {
        return Err(AppError::Validation(format!(
            "Create the Mod Inbox folder before watching it: {}",
            root.display()
        )));
    }
    let canonical_root = root.canonicalize()?;
    let callback_game_id = game_id.to_string();
    let callback_root = canonical_root.to_string_lossy().into_owned();
    let callback_app = app;
    let mut watcher = notify_debouncer_full::new_debouncer(
        Duration::from_millis(500),
        None,
        move |result: DebounceEventResult| {
            let error = match result {
                Ok(events) if events.is_empty() => return,
                Ok(_) => None,
                Err(errors) => Some(
                    errors
                        .into_iter()
                        .map(|error| error.to_string())
                        .collect::<Vec<_>>()
                        .join("; "),
                ),
            };
            let _ = callback_app.emit(
                "mod-inbox://changed",
                ModInboxChangedPayload {
                    game_id: callback_game_id.clone(),
                    root_path: callback_root.clone(),
                    error,
                },
            );
        },
    )
    .map_err(|error| AppError::Io(format!("Could not start Mod Inbox watcher: {error}")))?;
    watcher
        .watch(&canonical_root, RecursiveMode::Recursive)
        .map_err(|error| AppError::Io(format!("Could not watch Mod Inbox: {error}")))?;

    *lock(&state.active) = Some(ActiveInboxWatcher {
        game_id: game_id.to_string(),
        _watcher: watcher,
    });
    Ok(())
}

pub fn stop(state: &ModInboxWatcherState, game_id: &str) {
    let mut active = lock(&state.active);
    if active
        .as_ref()
        .is_some_and(|watcher| watcher.game_id == game_id)
    {
        *active = None;
    }
}
