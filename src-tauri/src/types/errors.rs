use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, serde::Deserialize, specta::Type)]
pub enum CommandError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("App error: {0}")]
    App(String),
    #[error("Object has {0} mods")]
    ObjectHasMods(i32),
}

impl From<sqlx::Error> for CommandError {
    fn from(error: sqlx::Error) -> Self {
        CommandError::Database(error.to_string())
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
