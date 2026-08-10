use crate::domain::errors::ScannerError;
use rayon::prelude::*;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;

use super::helpers::{auto_matched_candidate, canonical_entry_key};
use super::types::{ScanPreviewItem, ScoredCandidate};
use crate::services::scanner::core::types::{
    match_status_label, staged_confidence_label, ScanEvent,
};
use crate::services::scanner::core::walker;
use crate::services::scanner::deep_matcher;
use crate::services::scanner::deep_matcher::analysis::content::PreparedTokenFilters;
use crate::services::scanner::deep_matcher::models::result_summary::score_to_percentage;
use crate::services::scanner::deep_matcher::models::types;

type ExistingMod = (String, Option<String>);

struct PreviewWorker {
    mods_path: PathBuf,
    master_db: Arc<deep_matcher::MasterDb>,
    ini_filters: PreparedTokenFilters,
    resource_dir: Option<PathBuf>,
    existing_by_path: HashMap<String, ExistingMod>,
    matched_key_by_object: HashMap<String, String>,
}

struct PreviewWorkerInput<'a> {
    pool: &'a SqlitePool,
    game_id: &'a str,
    mods_path: &'a Path,
    master_db: Arc<deep_matcher::MasterDb>,
    ini_filters: PreparedTokenFilters,
    resource_dir: Option<&'a Path>,
}

struct PreviewProgress {
    channel: Option<Channel<ScanEvent>>,
    total: usize,
    started: std::time::Instant,
}

pub struct ScanPreviewRequest<'a> {
    pub pool: &'a SqlitePool,
    pub game_id: &'a str,
    pub mods_path: &'a Path,
    pub master_db: Arc<deep_matcher::MasterDb>,
    pub ini_filters: &'a PreparedTokenFilters,
    pub resource_dir: Option<&'a Path>,
    pub on_progress: Option<Channel<ScanEvent>>,
    pub specific_paths: Option<Vec<PathBuf>>,
}

/// Phase 1: Scan folders and run the Deep Match Scanner preview without writing to DB.
pub async fn scan_preview(
    request: ScanPreviewRequest<'_>,
) -> Result<Vec<ScanPreviewItem>, ScannerError> {
    let ScanPreviewRequest {
        pool,
        game_id,
        mods_path,
        master_db,
        ini_filters,
        resource_dir,
        on_progress,
        specific_paths,
    } = request;
    let candidates = if let Some(paths) = specific_paths {
        walker::scan_specific_folders(&paths)?
    } else {
        walker::scan_mod_folders(mods_path)?
    };
    let total = candidates.len();

    if let Some(channel) = &on_progress {
        let _ = channel.send(ScanEvent::Started {
            total_folders: total,
        });
    }

    log::info!(
        "scan_preview: start | game_id={game_id} folders={total} mods_path={}",
        mods_path.display()
    );

    let started = std::time::Instant::now();
    let worker = build_preview_worker(PreviewWorkerInput {
        pool,
        game_id,
        mods_path,
        master_db,
        ini_filters: ini_filters.clone(),
        resource_dir,
    })
    .await?;
    let completed = Arc::new(AtomicUsize::new(0));
    let progress = Arc::new(PreviewProgress {
        channel: on_progress,
        total,
        started,
    });
    let worker_progress = Arc::clone(&progress);
    let items = tauri::async_runtime::spawn_blocking(move || {
        candidates
            .into_par_iter()
            .map(|candidate| {
                let preview_item = worker.preview_item(&candidate);
                let current = completed.fetch_add(1, Ordering::Relaxed) + 1;
                worker_progress.send_item(&preview_item, current);
                preview_item
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|error| ScannerError::Io(format!("preview worker failed: {error}")))?;

    progress.finish(game_id, &items);
    Ok(items)
}

impl PreviewWorker {
    fn preview_item(&self, candidate: &walker::ModCandidate) -> ScanPreviewItem {
        let folder_path = candidate.path.to_string_lossy().to_string();
        let path_key = crate::common::path_key::folder_path_key(
            &folder_path,
            Some(&self.mods_path.to_string_lossy()),
        );
        let existing = self.existing_by_path.get(&path_key);
        let content = walker::scan_folder_content(&candidate.path, 3);
        let match_result = deep_matcher::match_folder_phased(
            candidate,
            &self.master_db,
            &content,
            &self.ini_filters,
            &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig::default(),
        );
        let auto_candidate = auto_matched_candidate(&match_result);

        let matched_alias_name = auto_candidate.map(|c| c.name.clone());
        let matched_entry_key = matched_alias_name
            .as_ref()
            .map(|name| canonical_entry_key(name));
        let object_type = auto_candidate.map(|c| c.object_type.clone());
        let match_level = match_status_label(&match_result.status).to_string();
        let confidence = staged_confidence_label(&match_result).to_string();
        let match_detail = Some(match_result.summary());
        let already_in_db = existing.is_some();
        let already_matched = self.is_already_matched(existing, matched_entry_key.as_deref());

        log::debug!(
            "scan_preview: item | folder={} status={match_level} confidence={confidence} score={} best={} already_in_db={already_in_db}",
            candidate.display_name,
            match_result.confidence_score(),
            matched_alias_name.as_deref().unwrap_or("-")
        );

        let db_entry = matched_alias_name.as_ref().and_then(|name| {
            self.master_db
                .entries
                .iter()
                .find(|entry| &entry.name == name)
        });

        let raw_thumbnail = db_entry.and_then(|e| e.thumbnail_path.clone());
        let db_thumbnail =
            resolve_thumbnail(&self.mods_path, db_entry, self.resource_dir.as_deref());
        let tags_json =
            db_entry.map(|e| serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".to_string()));
        let metadata_json = db_entry
            .and_then(|e| e.metadata.as_ref())
            .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string()));
        let hash_db_json = db_entry
            .map(|e| serde_json::to_string(&e.hash_db).unwrap_or_else(|_| "{}".to_string()));
        let custom_skins_json = db_entry
            .map(|e| serde_json::to_string(&e.custom_skins).unwrap_or_else(|_| "[]".to_string()));

        let scored_candidates: Vec<ScoredCandidate> = match_result
            .candidates_topk
            .iter()
            .map(|c| ScoredCandidate {
                name: c.name.clone(),
                object_type: c.object_type.clone(),
                score_pct: score_to_percentage(c),
            })
            .collect();

        ScanPreviewItem {
            folder_path,
            display_name: candidate.display_name.clone(),
            is_disabled: candidate.is_disabled,
            matched_entry_key,
            matched_alias_name,
            match_level,
            confidence,
            confidence_score: match_result.confidence_score(),
            match_detail,
            detected_skin: None,
            object_type,
            thumbnail_path: db_thumbnail,
            tags_json,
            metadata_json,
            hash_db_json,
            custom_skins_json,
            db_thumbnail: raw_thumbnail,
            already_in_db,
            already_matched,
            scored_candidates,
        }
    }

    fn is_already_matched(
        &self,
        existing: Option<&ExistingMod>,
        expected_entry_key: Option<&str>,
    ) -> bool {
        let (Some((_, Some(object_id))), Some(expected_key)) = (existing, expected_entry_key)
        else {
            return false;
        };
        self.matched_key_by_object
            .get(object_id)
            .map(String::as_str)
            == Some(expected_key)
    }
}

async fn build_preview_worker(
    input: PreviewWorkerInput<'_>,
) -> Result<PreviewWorker, ScannerError> {
    let mut connection = input.pool.acquire().await?;
    let rows =
        crate::repo::mod_repo::get_all_mods_sync_info_tx(&mut connection, input.game_id).await?;
    drop(connection);
    let mods_root = input.mods_path.to_string_lossy();
    let existing_by_path = rows
        .into_iter()
        .map(|(id, path, _, object_id, _, _)| {
            let key = crate::common::path_key::folder_path_key(&path, Some(&mods_root));
            (key, (id, object_id))
        })
        .collect();
    let matched_key_by_object =
        crate::repo::object_repo::get_matched_entry_keys_by_game(input.pool, input.game_id).await?;

    Ok(PreviewWorker {
        mods_path: input.mods_path.to_path_buf(),
        master_db: input.master_db,
        ini_filters: input.ini_filters,
        resource_dir: input.resource_dir.map(Path::to_path_buf),
        existing_by_path,
        matched_key_by_object,
    })
}

impl PreviewProgress {
    fn send_item(&self, preview_item: &ScanPreviewItem, current: usize) {
        let Some(channel) = &self.channel else { return };
        let elapsed_ms = self.started.elapsed().as_millis() as u64;
        let _ = channel.send(ScanEvent::Progress {
            current,
            total: self.total,
            folder_name: preview_item.display_name.clone(),
            elapsed_ms,
            eta_ms: elapsed_ms * (self.total - current) as u64 / current.max(1) as u64,
        });
        if let Some(object_name) = &preview_item.matched_alias_name {
            let _ = channel.send(ScanEvent::Matched {
                folder_name: preview_item.display_name.clone(),
                object_name: object_name.clone(),
                confidence: preview_item.confidence.clone(),
            });
        }
    }

    fn finish(&self, game_id: &str, items: &[ScanPreviewItem]) {
        let matched = items
            .iter()
            .filter(|preview_item| preview_item.matched_entry_key.is_some())
            .count();
        log::info!(
            "scan_preview: done | game_id={game_id} folders={} matched={matched} unmatched={} elapsed_ms={}",
            self.total,
            self.total - matched,
            self.started.elapsed().as_millis()
        );
        let Some(channel) = &self.channel else { return };
        let _ = channel.send(ScanEvent::Finished {
            matched,
            unmatched: self.total - matched,
        });
    }
}

fn resolve_thumbnail(
    mods_path: &Path,
    db_entry: Option<&types::DbEntry>,
    resource_dir: Option<&Path>,
) -> Option<String> {
    let entry = db_entry?;
    let r = entry.thumbnail_path.clone()?;

    if let Some(res_dir) = resource_dir {
        let abs = res_dir.join(&r);
        if abs.exists() {
            Some(abs.to_string_lossy().to_string())
        } else {
            None
        }
    } else {
        // No resource_dir: use mods_path as fallback base for absolute path resolution.
        let abs = mods_path.join(r);
        if abs.exists() {
            Some(abs.to_string_lossy().to_string())
        } else {
            None
        }
    }
}
