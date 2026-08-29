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
pub use types::{ArchiveAnalysis, ArchiveFormat, ExtractionEvent, ExtractionResult, StagedArchive};

/// Extract an archive into app-owned staging without changing the source archive.
/// Placement remains a separate, explicitly confirmed mutation.
pub fn extract_archive_to_staging(
    archive_path: &std::path::Path,
    staging_dir: &std::path::Path,
) -> Result<StagedArchive, crate::shared::errors::AppError> {
    let format = ArchiveFormat::detect(archive_path).ok_or_else(|| {
        crate::shared::errors::AppError::Validation(format!(
            "Unsupported archive format: {}",
            archive_path.display()
        ))
    })?;
    if staging_dir.exists() {
        return Err(crate::shared::errors::AppError::Validation(format!(
            "Import staging destination already exists: {}",
            staging_dir.display()
        )));
    }
    std::fs::create_dir_all(staging_dir)?;
    let extracted =
        match extractors::extract_to_dir(archive_path, staging_dir, None, format, None, None) {
            Ok(count) => count,
            Err(error) => {
                if let Err(cleanup_error) = std::fs::remove_dir_all(staging_dir) {
                    log::warn!(
                        "Could not clean failed import staging '{}': {cleanup_error}",
                        staging_dir.display()
                    );
                }
                return Err(error);
            }
        };
    let mod_roots = classify::find_mod_roots(staging_dir, 5);
    if mod_roots.is_empty() {
        if let Err(cleanup_error) = std::fs::remove_dir_all(staging_dir) {
            log::warn!(
                "Could not clean invalid import staging '{}': {cleanup_error}",
                staging_dir.display()
            );
        }
        return Err(crate::shared::errors::AppError::Validation(
            "Not a valid 3DMigoto mod archive (no valid .ini found)".to_string(),
        ));
    }
    Ok(StagedArchive {
        mod_roots,
        files_extracted: extracted,
    })
}

#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;
