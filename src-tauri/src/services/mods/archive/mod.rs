//! Archive extraction and smart flattening for mod archives.
//! broken down into submodules to respect line limits.
//!
//! # Covers: US-2.1, TC-2.1-01, TC-2.1-02

mod analyze;
pub mod classify;
mod destination;
mod extract;
mod extractors;
mod progress;
mod staging;
mod types;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Shared cancellation check for every extraction stage.
pub(super) fn is_cancelled(cancel_token: &Option<Arc<AtomicBool>>) -> bool {
    cancel_token
        .as_ref()
        .map(|token| token.load(Ordering::SeqCst))
        .unwrap_or(false)
}

// Re-export public API
pub use analyze::analyze_archive;
pub use extract::{extract_archive, ExtractOptions};
pub use types::{ArchiveAnalysis, ArchiveFormat, ExtractionEvent, ExtractionResult};

#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;
