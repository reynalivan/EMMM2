//! Commands related to archive detection and extraction.

use crate::domain::errors::AppError;
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
#[specta::specta]
pub async fn detect_archives_cmd(
    mods_path: String,
    config: State<'_, crate::services::config::ConfigService>,
) -> Result<Vec<ArchiveInfo>, AppError> {
    let mods_dir =
        crate::services::fs_utils::guard::validate_dir_in_configured_roots(&config, &mods_path)?;
    Ok(walker::detect_archives(&mods_dir)?)
}

/// Extract a single archive with optional password, smart flattening, and backup.
/// Automatically suppresses the file watcher during operation.
///
/// # Covers: TC-2.1-01, TC-2.1-04, TC-2.1-05, EC-2.06
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn extract_archive_cmd(
    archive_path: String,
    mods_dir: String,
    password: Option<String>,
    overwrite: Option<bool>,
    custom_name: Option<String>,
    disable_after: Option<bool>,
    unpack_nested: Option<bool>,
    on_progress: Channel<ExtractionEvent>,
    app: tauri::AppHandle,
) -> Result<ExtractionResult, AppError> {
    // One AppHandle instead of three `State` params: specta caps command
    // arity, and the states are all reachable from it.
    use tauri::Manager;
    let watcher = app.state::<WatcherState>();
    let ext_state = app.state::<ExtractionState>();
    let config = app.state::<crate::services::config::ConfigService>();
    let _guard = SuppressionGuard::new(&watcher.suppressor);

    // The archive itself may live anywhere (downloads folder); only the
    // write target is corridor-checked.
    let mods_dir =
        crate::services::fs_utils::guard::validate_dir_in_configured_roots(&config, &mods_dir)?;

    let archive = Path::new(&archive_path);
    let mods: &Path = &mods_dir;
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
        archive::ExtractOptions {
            password: pw_ref,
            overwrite: should_overwrite,
            cancel_token: Some(ext_state.is_cancelled.clone()),
            custom_name: name_ref,
            disable_after: should_disable,
            unpack_nested: should_unpack_nested,
            on_progress: Some(&on_progress),
        },
    )
}

/// Abort an ongoing extraction operation.
#[tauri::command]
#[specta::specta]
pub async fn abort_extraction_cmd(ext_state: State<'_, ExtractionState>) -> Result<(), AppError> {
    ext_state.is_cancelled.store(true, Ordering::SeqCst);
    Ok(())
}

/// Analyze an archive without extracting (file count, has_ini, size, etc).
///
/// # Covers: US-2.1 Pre-Extraction Analysis
#[tauri::command]
#[specta::specta]
pub async fn analyze_archive_cmd(archive_path: String) -> Result<ArchiveAnalysis, AppError> {
    let path = Path::new(&archive_path);
    archive::analyze_archive(path)
}

#[derive(Debug, serde::Serialize, serde::Deserialize, specta::Type)]
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
#[specta::specta]
pub async fn match_check_folder_cmd(
    folder_path: String,
    target_object_name: String,
    game_type: i32,
    app: tauri::AppHandle,
    config: State<'_, crate::services::config::ConfigService>,
) -> Result<MatchCheckResult, AppError> {
    let path =
        crate::services::fs_utils::guard::validate_dir_in_configured_roots(&config, &folder_path)?;
    let path = &*path;

    let Some(master_db) = crate::services::scanner::master_db::get_cached(&app, game_type).await?
    else {
        return Err(AppError::Scanner(
            crate::domain::errors::ScannerError::PathNotFound {
                path: format!("MasterDB for game type {game_type}"),
            },
        ));
    };
    let master_db = master_db.as_ref();

    let raw_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let is_disabled = crate::common::normalizer::is_disabled_folder(&raw_name);

    let candidate = walker::ModCandidate {
        path: path.to_path_buf(),
        display_name: crate::common::normalizer::normalize_display_name(&raw_name).into_owned(),
        raw_name,
        is_disabled,
    };

    let content = walker::scan_folder_content(&candidate.path, 3);
    let ini_filters = IniTokenizationConfig::default().prepare();
    let ai_config =
        crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig::default();

    let match_result =
        deep_matcher::match_folder_quick(&candidate, master_db, &content, &ini_filters, &ai_config);

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
