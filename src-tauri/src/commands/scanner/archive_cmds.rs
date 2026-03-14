//! Commands related to archive detection and extraction.

use crate::services::mods::archive::{self, ArchiveAnalysis, ExtractionEvent, ExtractionResult};
use crate::services::scanner::core::walker::{self, ArchiveInfo};
use crate::services::scanner::deep_matcher;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::models::result_summary::score_to_percentage;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::State;

/// State for managing ongoing archive extractions.
///
/// **B4 Note**: Uses a single shared `AtomicBool` cancel token. This assumes
/// only one extraction flow (Scanner OR DnD/ObjectList) is active at a time.
/// If both paths run concurrently, cancelling one will cancel both.
/// A per-extraction UUID token would fix this if concurrent extraction is needed.
pub struct ExtractionState {
    pub is_cancelled: Arc<AtomicBool>,
}

impl Default for ExtractionState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtractionState {
    pub fn new() -> Self {
        Self {
            is_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
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
    custom_name: Option<String>,
    disable_after: Option<bool>,
    unpack_nested: Option<bool>,
    on_progress: Channel<ExtractionEvent>,
    watcher: State<'_, WatcherState>,
    ext_state: State<'_, ExtractionState>,
) -> Result<ExtractionResult, String> {
    let _guard = SuppressionGuard::new(&watcher.suppressor);

    let archive = Path::new(&archive_path);
    let mods = Path::new(&mods_dir);
    let pw_ref = password.as_deref();
    let should_overwrite = overwrite.unwrap_or(false);
    let name_ref = custom_name.as_deref();
    let should_disable = disable_after.unwrap_or(false);
    let should_unpack_nested = unpack_nested.unwrap_or(true);

    // Reset cancellation token before starting
    ext_state.is_cancelled.store(false, Ordering::SeqCst);

    archive::extract_archive(
        archive,
        mods,
        pw_ref,
        should_overwrite,
        Some(ext_state.is_cancelled.clone()),
        name_ref,
        should_disable,
        should_unpack_nested,
        Some(&on_progress),
    )
}

/// Abort an ongoing extraction operation.
#[tauri::command]
pub async fn abort_extraction_cmd(ext_state: State<'_, ExtractionState>) -> Result<(), String> {
    ext_state.is_cancelled.store(true, Ordering::SeqCst);
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchCheckResult {
    pub matched_name: Option<String>,
    pub match_score_pct: u8,
    pub target_score_pct: u8,
    pub is_match: bool,
    pub confidence: String,
}

/// Light match check against a specific target object name.
/// Used for auto-organize validation after archive extraction.
///
/// # Covers: Req-38 (Auto-organizer Match Detection)
#[tauri::command]
pub async fn match_check_folder_cmd(
    folder_path: String,
    target_object_name: String,
    db_json: String,
) -> Result<MatchCheckResult, String> {
    let path = Path::new(&folder_path);
    if !path.exists() {
        return Err(format!("Extracted folder does not exist: {}", folder_path));
    }

    let master_db = deep_matcher::MasterDb::from_json(&db_json)?;

    let raw_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let is_disabled = crate::services::scanner::core::normalizer::is_disabled_folder(&raw_name);

    let candidate = walker::ModCandidate {
        path: path.to_path_buf(),
        display_name: crate::services::scanner::core::normalizer::normalize_display_name(&raw_name),
        raw_name,
        is_disabled,
    };

    let content = walker::scan_folder_content(&candidate.path, 3);
    let ini_config = IniTokenizationConfig::default();
    let ai_config =
        crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig::default();

    let match_result =
        deep_matcher::match_folder_quick(&candidate, &master_db, &content, &ini_config, &ai_config);

    let matched_name = match_result.best.as_ref().map(|c| c.name.clone());
    let match_score_pct = match_result
        .best
        .as_ref()
        .map(score_to_percentage)
        .unwrap_or(0);
    let confidence =
        crate::services::scanner::core::types::staged_confidence_label(&match_result).to_string();

    let mut target_score_pct = 0;
    if let Some(best) = &match_result.best {
        if best.name == target_object_name {
            target_score_pct = match_score_pct;
        }
    }

    // If target isn't the best match, find its score
    if target_score_pct == 0 {
        if let Some(target_cand) = match_result
            .candidates_all
            .iter()
            .find(|c| c.name == target_object_name)
        {
            target_score_pct = score_to_percentage(target_cand);
        }
    }

    let is_match = matched_name.as_deref() == Some(target_object_name.as_str());

    Ok(MatchCheckResult {
        matched_name,
        match_score_pct,
        target_score_pct,
        is_match,
        confidence,
    })
}

#[cfg(test)]
#[path = "tests/archive_cmds_tests.rs"]
mod tests;
