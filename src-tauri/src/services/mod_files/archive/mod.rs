//! Archive extraction and smart flattening for mod archives.
//! broken down into submodules to respect line limits.
//!
//! # Covers: US-2.1, TC-2.1-01, TC-2.1-02

mod analyze;
mod extract;
mod types;

// Re-export public API
pub use analyze::analyze_archive;
pub use extract::extract_archive;
pub use types::{ArchiveAnalysis, ArchiveFormat, ExtractionResult};

#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;
