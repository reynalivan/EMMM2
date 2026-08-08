use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Typed error enums for each domain
// ---------------------------------------------------------------------------

/// Errors specific to corridor operations.
#[derive(Debug, Clone, Error, Serialize, Deserialize, specta::Type)]
pub enum CorridorError {
    #[error("Game '{game_id}' has no mods_path configured")]
    NoModsPath { game_id: String },

    #[error("Game '{game_id}' not found")]
    GameNotFound { game_id: String },

    #[error("Database error: {0}")]
    Db(String), // Converted sqlx::Error to String for Serde/Specta

    #[error("Collection error: {0}")]
    Collection(#[from] Box<CollectionError>),
}

impl From<sqlx::Error> for CorridorError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e.to_string())
    }
}

impl From<CollectionError> for AppError {
    fn from(error: CollectionError) -> Self {
        // Keep the lock classification at the top level: wrapping it in
        // `Collection(..)` would hide the discriminant the frontend matches on.
        match error {
            CollectionError::FileInUse { path, processes } => Self::FileInUse { path, processes },
            CollectionError::PathBusy { path } => Self::PathBusy { path },
            other => Self::Collection(other),
        }
    }
}

impl From<CollectionError> for CorridorError {
    fn from(e: CollectionError) -> Self {
        Self::Collection(Box::new(e))
    }
}

/// Errors specific to collection operations.
#[derive(Debug, Clone, Error, Serialize, Deserialize, specta::Type)]
pub enum CollectionError {
    #[error("Collection '{id}' not found")]
    NotFound { id: String },

    #[error("Collection name '{name}' already exists in this corridor")]
    DuplicateName { name: String },

    #[error("Missing mods on disk: {count} mod(s) not found")]
    MissingMods {
        #[specta(type = f64)]
        count: usize,
        paths: Vec<String>,
    },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Db(String), // Converted sqlx::Error to String for Serde/Specta

    #[error("Corridor error: {0}")]
    Corridor(#[from] CorridorError),

    #[error("IO error: {0}")]
    Io(String), // Converted std::io::Error to String for Serde/Specta

    // Carried structurally rather than flattened into `Io` so the apply path
    // surfaces the same actionable error the single-mod toggle does — the
    // frontend keys its retry dialog on the `FileInUse` discriminant.
    #[error("File in use by another process: {path}. Processes: {processes:?}")]
    FileInUse {
        path: String,
        processes: Vec<String>,
    },

    #[error("Path is busy and cannot be renamed right now: {path}")]
    PathBusy { path: String },
}

impl From<sqlx::Error> for CollectionError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e.to_string())
    }
}

impl From<std::io::Error> for CollectionError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Errors specific to Metadata operations.
#[derive(Debug, Clone, Error, Serialize, Deserialize, specta::Type)]
pub enum MetadataError {
    #[error("Path violation: {0}")]
    Security(String),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Database error: {0}")]
    Db(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<sqlx::Error> for MetadataError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e.to_string())
    }
}

impl From<std::io::Error> for MetadataError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Errors specific to Pin operations.
#[derive(Debug, Clone, Error, Serialize, Deserialize, specta::Type)]
pub enum PinError {
    #[error("Database error: {0}")]
    Db(String),
}

impl From<sqlx::Error> for PinError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e.to_string())
    }
}

/// Errors from the in-app browser: webview lifecycle, downloads, and the
/// import pipeline that turns a download into a placed mod.
#[derive(Debug, Clone, Error, Serialize, Deserialize, specta::Type)]
pub enum BrowserError {
    #[error("Browser window not available")]
    WindowUnavailable,

    #[error("Webview '{label}' not found")]
    WebviewNotFound { label: String },

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Download failed: {0}")]
    Download(String),

    #[error("Import job '{job_id}' is missing {field}")]
    JobIncomplete { job_id: String, field: String },

    #[error("Import failed: {0}")]
    Import(String),

    #[error("Background queue is closed")]
    QueueClosed,

    #[error("IO error: {0}")]
    Io(String),

    #[error("Database error: {0}")]
    Db(String),
}

impl From<sqlx::Error> for BrowserError {
    fn from(error: sqlx::Error) -> Self {
        Self::Db(error.to_string())
    }
}

impl From<std::io::Error> for BrowserError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<tauri::Error> for BrowserError {
    fn from(error: tauri::Error) -> Self {
        Self::Import(error.to_string())
    }
}

/// Errors from scanning the mods tree: walking folders, matching against the
/// MasterDB, duplicate detection, and committing a scan into the index.
#[derive(Debug, Clone, Error, Serialize, Deserialize, specta::Type)]
pub enum ScannerError {
    #[error("Path not found: {path}")]
    PathNotFound { path: String },

    #[error("Not a directory: {path}")]
    NotADirectory { path: String },

    #[error("Path escapes the mods directory: {path}")]
    PathEscape { path: String },

    #[error("Could not parse {what}: {detail}")]
    Parse { what: String, detail: String },

    #[error("Remote lookup failed: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Database error: {0}")]
    Db(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<sqlx::Error> for ScannerError {
    fn from(error: sqlx::Error) -> Self {
        Self::Db(error.to_string())
    }
}

impl From<std::io::Error> for ScannerError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<serde_json::Error> for ScannerError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parse {
            what: "JSON".to_string(),
            detail: error.to_string(),
        }
    }
}

impl From<tokio::task::JoinError> for ScannerError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::Io(format!("background scan task failed: {error}"))
    }
}

impl From<notify::Error> for ScannerError {
    fn from(error: notify::Error) -> Self {
        Self::Io(format!("filesystem watcher failed: {error}"))
    }
}

impl From<reqwest::Error> for ScannerError {
    fn from(error: reqwest::Error) -> Self {
        Self::Network(error.to_string())
    }
}

/// Unified error type for Tauri command boundaries.
/// Each domain error converts into this for consistent frontend handling.
#[derive(Debug, Clone, Error, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "payload")]
pub enum AppError {
    #[error("{0}")]
    Corridor(#[from] CorridorError),

    #[error("{0}")]
    Collection(CollectionError),

    #[error("{0}")]
    Pin(#[from] PinError),

    #[error("{0}")]
    Metadata(#[from] MetadataError),

    #[error("{0}")]
    Browser(#[from] BrowserError),

    #[error("{0}")]
    Scanner(#[from] ScannerError),

    #[error("Security violation: {0}")]
    Security(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Db(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Runtime path not found: {target}")]
    RuntimePathNotFound { target: String },

    #[error("Duplicate conflict for object: {0:?}")]
    DuplicateConflict(Vec<crate::domain::mods::DuplicateModInfo>),

    #[error("File in use by another process: {path}. Processes: {processes:?}")]
    FileInUse {
        path: String,
        processes: Vec<String>,
    },

    #[error("Path is busy and cannot be renamed right now: {path}")]
    PathBusy { path: String },

    #[error("Object has {0} mods")]
    ObjectHasMods(i32),

    /// The user cancelled a long-running operation.
    ///
    /// A variant rather than the `"ABORTED"` string the extractors used to
    /// return: callers matched on that message text, so any rewording of an
    /// error message could silently turn a cancel into a failure.
    #[error("Operation cancelled")]
    Cancelled,
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::Internal(format!("background task failed: {error}"))
    }
}

impl From<tokio::sync::AcquireError> for AppError {
    fn from(error: tokio::sync::AcquireError) -> Self {
        Self::Internal(format!("concurrency permit unavailable: {error}"))
    }
}

impl From<tauri_plugin_global_shortcut::Error> for AppError {
    fn from(error: tauri_plugin_global_shortcut::Error) -> Self {
        Self::Validation(format!("hotkey registration failed: {error}"))
    }
}

impl From<image::ImageError> for AppError {
    fn from(error: image::ImageError) -> Self {
        Self::Io(format!("image decode/encode failed: {error}"))
    }
}

impl From<enigo::NewConError> for AppError {
    fn from(error: enigo::NewConError) -> Self {
        Self::Internal(format!("input simulation unavailable: {error}"))
    }
}

impl From<enigo::InputError> for AppError {
    fn from(error: enigo::InputError) -> Self {
        Self::Internal(format!("input simulation failed: {error}"))
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::Io(format!("archive read failed: {error}"))
    }
}

impl From<rar::error::RarError> for AppError {
    fn from(error: rar::error::RarError) -> Self {
        Self::Io(format!("RAR extraction failed: {error}"))
    }
}
