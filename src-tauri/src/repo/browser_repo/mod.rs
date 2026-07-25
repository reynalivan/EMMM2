//! Persistence for the in-app browser: settings, downloads, and import jobs.
//! Pure SQL only — nothing here may depend on `services::`.

mod downloads;
mod import_jobs;
mod settings;

pub use downloads::*;
pub use import_jobs::*;
pub use settings::*;
