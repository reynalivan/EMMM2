//! Commands related to archive detection and extraction.

use crate::services::mod_files::archive::{self, ArchiveAnalysis, ExtractionResult};
use crate::services::scanner::core::walker::{self, ArchiveInfo};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::Path;
use tauri::State;

/// Detect archive files (ZIP, 7z, RAR) in the mods directory.
///
/// # Covers: US-2.1
#[tauri::command]
pub async fn detect_archives_cmd(mods_path: String) -> Result<Vec<ArchiveInfo>, String> {
    let path = Path::new(&mods_path);
    walker::detect_archives(path)
}

/// Extract a single archive with optional password, smart flattening, and backup.
/// Automatically suppresses the file watcher during operation.
///
/// # Covers: TC-2.1-01, TC-2.1-04, TC-2.1-05, EC-2.06
#[tauri::command]
pub async fn extract_archive_cmd(
    archive_path: String,
    mods_dir: String,
    password: Option<String>,
    overwrite: Option<bool>,
    watcher: State<'_, WatcherState>,
) -> Result<ExtractionResult, String> {
    let _guard = SuppressionGuard::new(&watcher.suppressor);

    let archive = Path::new(&archive_path);
    let mods = Path::new(&mods_dir);
    let pw_ref = password.as_deref();
    let should_overwrite = overwrite.unwrap_or(false);

    archive::extract_archive(archive, mods, pw_ref, should_overwrite)
}

/// Analyze an archive without extracting (file count, has_ini, size, etc).
///
/// # Covers: US-2.1 Pre-Extraction Analysis
#[tauri::command]
pub async fn analyze_archive_cmd(archive_path: String) -> Result<ArchiveAnalysis, String> {
    let path = Path::new(&archive_path);
    archive::analyze_archive(path)
}
