//! Tauri commands for Epic 2: Mod Scanning & Organization.
//!
//! Exposes core scanning loops to the React frontend.
//! Uses `tauri::ipc::Channel` for streaming scan progress events.

use crate::services::scanner::core::thumbnail;
use crate::services::scanner::deep_matcher::analysis::ai_provider::HttpAiRerankProvider;
use crate::services::scanner::deep_matcher::analysis::ai_rerank::{
    AiRerankConfig, AiRerankProvider,
};
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::state::signal_cache::SignalCache;
use crate::services::scanner::deep_matcher::{self, MasterDb};

use crate::services::scanner::core::types::{
    build_result_item_from_staged, staged_auto_matched_object_name, staged_confidence_label,
    ScanEvent, ScanResultItem,
};
use crate::services::scanner::core::walker::{self, ModCandidate};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

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
}

impl Default for ScanState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScanState {
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

// ─── Core Commands ─────────────────────────────────────────────────

/// Cancel the currently running scan.
#[tauri::command]
pub async fn cancel_scan_cmd(state: State<'_, ScanState>) -> Result<(), String> {
    state.cancel();
    Ok(())
}

/// Helper to build AI Config
fn get_ai_provider(app_handle: &AppHandle) -> Option<HttpAiRerankProvider> {
    let config_service = app_handle.state::<crate::services::config::ConfigService>();
    let ai_settings = config_service.get_settings().ai;

    if ai_settings.enabled {
        if let Some(api_key) = ai_settings.api_key {
            if !api_key.is_empty() {
                return Some(HttpAiRerankProvider::new(api_key, ai_settings.base_url));
            }
        }
    }
    None
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
    app_handle: AppHandle,
) -> Result<Vec<ScanResultItem>, String> {
    state.reset();

    let mods = Path::new(&mods_path);
    let db = MasterDb::from_json(&db_json)?;
    let candidates = walker::scan_mod_folders(mods)?;
    let total = candidates.len();

    let _ = on_progress.send(ScanEvent::Started {
        total_folders: total,
    });

    let mut results = Vec::with_capacity(total);
    let mut matched_count = 0;

    let ai_provider = get_ai_provider(&app_handle);
    let config_service = app_handle.state::<crate::services::config::ConfigService>();
    let ai_settings = config_service.get_settings().ai;

    let ai_config = AiRerankConfig {
        ai_enabled: ai_settings.enabled,
        db_version: Some("db-v1"),
        provider: ai_provider.as_ref().map(|p| p as &dyn AiRerankProvider),
        cache: None,
    };

    let ini_config = IniTokenizationConfig::default();
    let scan_start = Instant::now();
    let mut signal_cache = SignalCache::new();

    for (idx, candidate) in candidates.iter().enumerate() {
        if state.is_cancelled() {
            log::info!("Scan cancelled by user at {}/{}", idx, total);
            break;
        }

        let item = process_candidate_and_notify(
            candidate,
            &db,
            &ini_config,
            &ai_config,
            &mut signal_cache,
            idx,
            total,
            scan_start,
            &on_progress,
            &mut matched_count,
        );
        results.push(item);
    }

    let _ = on_progress.send(ScanEvent::Finished {
        matched: matched_count,
        unmatched: total - matched_count,
    });

    log::info!("Scan complete: {}/{} matched", matched_count, total);
    Ok(results)
}

/// Helper for starting scan, separates scanning one item to apply SRP.
#[allow(clippy::too_many_arguments)]
fn process_candidate_and_notify(
    candidate: &ModCandidate,
    db: &MasterDb,
    ini_config: &IniTokenizationConfig,
    ai_config: &AiRerankConfig,
    signal_cache: &mut SignalCache,
    idx: usize,
    total: usize,
    scan_start: Instant,
    on_progress: &Channel<ScanEvent>,
    matched_count: &mut usize,
) -> ScanResultItem {
    let content = walker::scan_folder_content(&candidate.path, 3);

    let match_result = deep_matcher::match_folder_phased_cached(
        candidate,
        db,
        &content,
        ini_config,
        ai_config,
        signal_cache,
    );

    let thumb = thumbnail::find_thumbnail(&candidate.path);

    if let Some(object_name) = staged_auto_matched_object_name(&match_result) {
        *matched_count += 1;
        let _ = on_progress.send(ScanEvent::Matched {
            folder_name: candidate.display_name.clone(),
            object_name: object_name.to_string(),
            confidence: staged_confidence_label(&match_result).to_string(),
        });
    }

    let elapsed = scan_start.elapsed().as_millis() as u64;
    let done = (idx + 1) as u64;
    let remaining = (total as u64).saturating_sub(done);
    let eta = if done > 0 {
        (elapsed / done) * remaining
    } else {
        0
    };

    let _ = on_progress.send(ScanEvent::Progress {
        current: idx + 1,
        total,
        folder_name: candidate.display_name.clone(),
        elapsed_ms: elapsed,
        eta_ms: eta,
    });

    let (detected_skin, skin_folder_name) =
        deep_matcher::detect_skin_for_staged(&match_result, &candidate.display_name, db);

    build_result_item_from_staged(
        candidate,
        &match_result,
        thumb,
        detected_skin,
        skin_folder_name,
    )
}

/// Run the scan pipeline without progress events (batch/initial load).
///
/// Same logic as `start_scan` but returns results synchronously.
#[tauri::command]
pub async fn get_scan_result(
    mods_path: String,
    db_json: String,
) -> Result<Vec<ScanResultItem>, String> {
    let mods = Path::new(&mods_path);
    let db = MasterDb::from_json(&db_json)?;
    let candidates = walker::scan_mod_folders(mods)?;
    let ini_config = IniTokenizationConfig::default();
    let ai_config = AiRerankConfig::default();

    let results = candidates
        .iter()
        .map(|candidate| process_candidate_batch(candidate, &db, &ini_config, &ai_config))
        .collect();

    Ok(results)
}

/// Helper for get_scan_result batch processing.
fn process_candidate_batch(
    candidate: &ModCandidate,
    db: &MasterDb,
    ini_config: &IniTokenizationConfig,
    ai_config: &AiRerankConfig,
) -> ScanResultItem {
    let content = walker::scan_folder_content(&candidate.path, 3);
    let match_result =
        deep_matcher::match_folder_phased(candidate, db, &content, ini_config, ai_config);
    let thumb = thumbnail::find_thumbnail(&candidate.path);
    let (detected_skin, skin_folder_name) =
        deep_matcher::detect_skin_for_staged(&match_result, &candidate.display_name, db);

    build_result_item_from_staged(
        candidate,
        &match_result,
        thumb,
        detected_skin,
        skin_folder_name,
    )
}

#[cfg(test)]
#[path = "tests/scan_cmds_tests.rs"]
mod tests;
