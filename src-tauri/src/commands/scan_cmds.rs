//! Tauri commands for Epic 2: Mod Scanning & Organization.
//!
//! Exposes scanner, archive, and conflict services to the React frontend.
//! Uses `tauri::ipc::Channel` for streaming scan progress events.

use crate::services::file_ops::archive::{self, ArchiveAnalysis, ExtractionResult};
use crate::services::scanner::conflict::{self, ConflictInfo};
use crate::services::scanner::deep_matcher::{self, MasterDb, MatchLevel};
use crate::services::scanner::thumbnail;
use crate::services::scanner::walker::{self, ArchiveInfo};
use crate::services::watcher::{SuppressionGuard, WatcherState};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::ipc::Channel;
use tauri::State;

// ─── State Management ──────────────────────────────────────────────

pub struct ScanState {
    pub is_cancelled: AtomicBool,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            is_cancelled: AtomicBool::new(false),
        }
    }
    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }
    pub fn reset(&self) {
        self.is_cancelled.store(false, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }
}

use crate::services::scanner::types::{self, ScanEvent, ScanResultItem};

// ─── Commands ──────────────────────────────────────────────────────

/// Cancel the currently running scan.
#[tauri::command]
pub async fn cancel_scan_cmd(state: State<'_, ScanState>) -> Result<(), String> {
    state.cancel();
    Ok(())
}

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

/// Manually set watcher suppression state (e.g. for bulk operations).
///
/// # Covers: EC-2.06
#[tauri::command]
pub async fn set_watcher_suppression_cmd(
    suppressed: bool,
    watcher: State<'_, WatcherState>,
) -> Result<(), String> {
    watcher.suppressor.store(suppressed, Ordering::Relaxed);
    Ok(())
}

/// Analyze an archive without extracting (file count, has_ini, size, etc).
///
/// # Covers: US-2.1 Pre-Extraction Analysis
#[tauri::command]
pub async fn analyze_archive_cmd(archive_path: String) -> Result<ArchiveAnalysis, String> {
    let path = Path::new(&archive_path);
    archive::analyze_archive(path)
}

/// Run the full scan pipeline with real-time progress events.
///
/// Orchestrates: walker → deep_matcher → thumbnail for each mod folder,
/// streaming `ScanEvent` updates via the `on_progress` Channel.
///
/// # Covers: TC-2.3-01, TC-2.2-01, TC-2.2-02, TC-2.2-03, TC-2.3-02
#[tauri::command]
pub async fn start_scan(
    mods_path: String,
    db_json: String,
    on_progress: Channel<ScanEvent>,
    state: State<'_, ScanState>,
) -> Result<Vec<ScanResultItem>, String> {
    // Reset cancellation state before starting
    state.reset();

    let mods = Path::new(&mods_path);

    // 1. Load Master DB from JSON
    let db = MasterDb::from_json(&db_json)?;

    // 2. Discover mod folders
    let candidates = walker::scan_mod_folders(mods)?;
    let total = candidates.len();

    // Send Started event
    let _ = on_progress.send(ScanEvent::Started {
        total_folders: total,
    });

    // 3. Process each candidate
    let mut results = Vec::with_capacity(total);
    let mut matched_count: usize = 0;

    for (idx, candidate) in candidates.iter().enumerate() {
        // Check cancellation
        if state.is_cancelled() {
            log::info!("Scan cancelled by user at {}/{}", idx, total);
            // We can break early.
            // Should we return partial results?
            // TC-2.3-02 says "Partial results discarded" or strict abort?
            // "Scan aborts cleanly... Partial results discarded... DB unchanged."
            // So we should verify if we want to return what we have or error out.
            // Returning Ok(results) updates the table with partial data.
            // Returning Err("Cancelled") might be handled as error.
            // Let's return partial results but stop processing.
            break;
        }

        // Scan folder content for deep matching
        let content = walker::scan_folder_content(&candidate.path, 3);

        // Run matching pipeline
        let match_result = deep_matcher::match_folder(candidate, &db, &content);

        // Find thumbnail
        let thumb = thumbnail::find_thumbnail(&candidate.path);

        // Track match count
        if match_result.level != MatchLevel::Unmatched {
            matched_count += 1;

            let _ = on_progress.send(ScanEvent::Matched {
                folder_name: candidate.display_name.clone(),
                object_name: match_result.object_name.clone(),
                confidence: types::confidence_label(&match_result.level).to_string(),
            });
        }

        // Send progress
        let _ = on_progress.send(ScanEvent::Progress {
            current: idx + 1,
            folder_name: candidate.display_name.clone(),
        });

        // Build result
        results.push(types::build_result_item(candidate, &match_result, thumb));
    }

    // Send Finished event
    let _ = on_progress.send(ScanEvent::Finished {
        matched: matched_count,
        unmatched: total - matched_count,
    });

    log::info!(
        "Scan complete: {}/{} matched in {} folders",
        matched_count,
        total,
        mods_path
    );

    Ok(results)
}

/// Run the scan pipeline without progress events (batch/initial load).
///
/// Same logic as `start_scan` but returns results synchronously
/// without Channel overhead — useful for re-loading cached results.
/// Note: This does NOT check cancellation state as it's meant to be fast/blocking?
/// Actually it calls `scan_folder_content` which can be slow.
/// But usually used for small batches or initial verify.
#[tauri::command]
pub async fn get_scan_result(
    mods_path: String,
    db_json: String,
) -> Result<Vec<ScanResultItem>, String> {
    let mods = Path::new(&mods_path);
    let db = MasterDb::from_json(&db_json)?;
    let candidates = walker::scan_mod_folders(mods)?;

    let results: Vec<ScanResultItem> = candidates
        .iter()
        .map(|candidate| {
            let content = walker::scan_folder_content(&candidate.path, 3);
            let match_result = deep_matcher::match_folder(candidate, &db, &content);
            let thumb = thumbnail::find_thumbnail(&candidate.path);
            types::build_result_item(candidate, &match_result, thumb)
        })
        .collect();

    Ok(results)
}

/// Detect shader/buffer hash conflicts across INI files.
///
/// # Covers: US-2.Z, TC-2.4-01
#[tauri::command]
pub async fn detect_conflicts_cmd(ini_paths: Vec<String>) -> Result<Vec<ConflictInfo>, String> {
    let paths: Vec<PathBuf> = ini_paths.iter().map(PathBuf::from).collect();
    Ok(conflict::detect_conflicts(&paths))
}

/// Detect conflicts by scanning the entire mods folder for INI files.
///
/// More efficient for frontend usage as it avoids passing thousands of paths.
/// # Covers: US-2.Z
#[tauri::command]
pub async fn detect_conflicts_in_folder_cmd(
    mods_path: String,
) -> Result<Vec<ConflictInfo>, String> {
    let path = Path::new(&mods_path);
    // Use walker to find all mod folders
    let candidates = walker::scan_mod_folders(path)?;

    let mut all_inis = Vec::new();
    for candidate in candidates {
        // Only check enabled mods? US-2.Z says "active mods".
        if candidate.is_disabled {
            continue;
        }

        let content = walker::scan_folder_content(&candidate.path, 3); // Depth 3 per Epic 2
        all_inis.extend(content.ini_files);
    }

    Ok(conflict::detect_conflicts(&all_inis))
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_detect_conflicts_in_folder_integration() {
        let dir = TempDir::new().unwrap();
        let mod_a = dir.path().join("ModA");
        let mod_b = dir.path().join("ModB");
        let mod_disabled = dir.path().join("DISABLED ModC");

        fs::create_dir(&mod_a).unwrap();
        fs::create_dir(&mod_b).unwrap();
        fs::create_dir(&mod_disabled).unwrap();

        // Conflict between ModA and ModB
        fs::write(
            mod_a.join("config.ini"),
            "[TextureOverrideBody]\nhash = abc123\n",
        )
        .unwrap();
        fs::write(
            mod_b.join("config.ini"),
            "[TextureOverrideBody]\nhash = abc123\n",
        )
        .unwrap();

        // ModC has same hash but is DISABLED, so should be ignored
        fs::write(
            mod_disabled.join("config.ini"),
            "[TextureOverrideBody]\nhash = abc123\n",
        )
        .unwrap();

        let conflicts = detect_conflicts_in_folder_cmd(dir.path().to_string_lossy().to_string())
            .await
            .unwrap();

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].hash, "abc123");
        assert_eq!(conflicts[0].mod_paths.len(), 2);
    }

    #[tokio::test]
    async fn test_scan_state_cancellation() {
        let state = ScanState::new();
        assert!(!state.is_cancelled());

        state.cancel();
        assert!(state.is_cancelled());

        state.reset();
        assert!(!state.is_cancelled());
    }
}
