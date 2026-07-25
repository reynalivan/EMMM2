use serde::Serialize;

/// Internal events produced by the watcher closure.
/// Consumed by the lifecycle event loop for DB sync.
#[derive(Debug, Clone)]
pub enum ModWatchEvent {
    Created(String),
    Modified(String),
    Removed(String),
    Renamed {
        from: String,
        to: String,
    },
    StatusChanged {
        from: String,
        to: String,
        from_status: &'static str,
        to_status: &'static str,
    },
    Error(String),
}

/// Error payload emitted via `mod_watch:event` when the watcher or a
/// watcher-triggered reconcile fails. Data events reach the frontend through
/// `disk_reconcile:result` only.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WatchEventPayload {
    Error {
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
}
