//! Persistence for the in-app browser: settings and downloads.
//! Pure SQL only — nothing here may depend on `services::`.

mod downloads;
mod settings;

pub use downloads::*;
pub use settings::*;
