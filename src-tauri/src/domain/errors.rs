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

    #[error("Cannot apply {collection_mode} collection while in {current_mode} corridor")]
    CorridorMismatch {
        collection_mode: String,
        current_mode: String,
    },

    #[error("Corridor switch already in progress for game '{game_id}'")]
    SwitchInProgress { game_id: String },

    #[error("Rename failed for '{path}': {error}")]
    RenameFailed {
        path: String,
        error: String, // Converted std::io::Error to String for Serde/Specta
    },

    #[error("Batch rename partially failed: {succeeded} succeeded, {failed} failed")]
    PartialRenameFailed {
        #[specta(type = f64)]
        succeeded: usize,
        #[specta(type = f64)]
        failed: usize,
    },

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

    #[error("Cannot modify undo snapshot collection")]
    CannotModifyUndoSnapshot,

    #[error("No undo snapshot available for this corridor")]
    NoUndoAvailable,

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
    #[error("Pin validation failed: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Db(String),

    #[error("Account locked until {0}")]
    Locked(String),
}

impl From<sqlx::Error> for PinError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e.to_string())
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
    Collection(#[from] CollectionError),

    #[error("{0}")]
    Pin(#[from] PinError),

    #[error("{0}")]
    Metadata(#[from] MetadataError),

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

    #[error("Duplicate conflict for object: {0:?}")]
    DuplicateConflict(Vec<crate::domain::mods::DuplicateModInfo>),

    #[error("File in use by another process: {path}. Processes: {processes:?}")]
    FileInUse { path: String, processes: Vec<String> },
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
