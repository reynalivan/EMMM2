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
mod security;
mod staging;
mod types;
mod zip_reader;

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
pub use analyze::{analyze_archive, analyze_archive_with_limits};
pub use extract::{extract_archive, ExtractOptions};
pub use security::ArchiveLimits;
pub use staging::{
    extract_archive_to_staging_with_limits, extract_archive_to_staging_with_options,
    StagingExtractOptions,
};
pub use types::{ArchiveAnalysis, ArchiveFormat, ExtractionEvent, ExtractionResult, StagedArchive};

/// Extract an archive into app-owned staging without changing the source archive.
/// Placement remains a separate, explicitly confirmed mutation.
pub fn extract_archive_to_staging(
    archive_path: &std::path::Path,
    staging_dir: &std::path::Path,
) -> Result<StagedArchive, crate::shared::errors::AppError> {
    extract_archive_to_staging_with_options(
        archive_path,
        staging_dir,
        StagingExtractOptions::default(),
    )
}

#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/security_tests.rs"]
mod security_tests;
