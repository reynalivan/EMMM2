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

/// Strongly-typed diagnostic payload emitted via `mod_watch:event`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WatchEventPayload {
    Created {
        path: String,
    },
    Modified {
        path: String,
    },
    Removed {
        path: String,
    },
    Renamed {
        from: String,
        to: String,
    },
    StatusChanged {
        path: String,
        from: String,
        to: String,
        from_status: String,
        to_status: String,
    },
    Error {
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_count: Option<u32>,
    },
}

impl WatchEventPayload {
    pub fn from_event(event: &ModWatchEvent) -> Self {
        match event {
            ModWatchEvent::Created(path) => Self::Created { path: path.clone() },
            ModWatchEvent::Modified(path) => Self::Modified { path: path.clone() },
            ModWatchEvent::Removed(path) => Self::Removed { path: path.clone() },
            ModWatchEvent::Renamed { from, to } => Self::Renamed {
                from: from.clone(),
                to: to.clone(),
            },
            ModWatchEvent::StatusChanged {
                from,
                to,
                from_status,
                to_status,
            } => Self::StatusChanged {
                path: to.clone(),
                from: from.clone(),
                to: to.clone(),
                from_status: from_status.to_string(),
                to_status: to_status.to_string(),
            },
            ModWatchEvent::Error(error) => Self::Error {
                error: error.clone(),
                path: None,
                retry_count: None,
            },
        }
    }
}
