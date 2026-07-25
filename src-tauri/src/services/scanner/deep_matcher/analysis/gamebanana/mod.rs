//! GameBanana metadata enrichment for the deep matcher pipeline.
//!
//! Detects GameBanana URLs in INI data/signals, fetches file metadata via their
//! public API, and returns normalized file stems as enrichment data.
//!
//! **Fail-safe**: All network failures are logged and skipped. This module
//! never blocks the pipeline.

mod api;
mod detect;
mod types;

pub use api::*;
pub use detect::*;
pub use types::*;

#[cfg(test)]
#[path = "../../tests/analysis/gamebanana_tests.rs"]
mod tests;
