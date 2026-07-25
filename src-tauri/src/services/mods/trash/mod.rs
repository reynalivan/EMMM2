//! Soft delete (Trash) service for mod folders.
//!
//! Moves mod folders to `./app_data/trash/{uuid}/` with metadata JSON for restore.
//! Does NOT use the OS Recycle Bin — uses a custom app-level trash.
//!
//! # Covers: US-4.4 (Soft Delete), TC-4.5-01, DI-4.01
//!
//! Split by concern. Public API is unchanged: every item the rest of the crate
//! used to import from `services::mods::trash` is re-exported here.

mod service;
mod store;
mod timestamp;
mod types;

pub use service::*;
pub use store::*;
pub use types::*;

#[cfg(test)]
#[path = "../tests/trash_tests.rs"]
mod tests;
