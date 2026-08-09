use crate::domain::errors::ScannerError;
use sqlx::SqlitePool;
use std::path::Path;
use tauri::ipc::Channel;

use super::helpers::{auto_matched_candidate, canonical_entry_key};
use super::types::{ScanPreviewItem, ScoredCandidate};
use crate::services::scanner::core::types::{
    match_status_label, staged_confidence_label, ScanEvent,
};
use crate::services::scanner::core::walker;
use crate::services::scanner::deep_matcher;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::models::result_summary::score_to_percentage;
use crate::services::scanner::deep_matcher::models::types;

/// Phase 1: Scan folders and run the Deep Match Scanner preview without writing to DB.
pub async fn scan_preview(
    pool: &SqlitePool,
    game_id: &str,
    mods_path: &Path,
    master_db: &deep_matcher::MasterDb,
    resource_dir: Option<&Path>,
    on_progress: Option<Channel<ScanEvent>>,
    specific_paths: Option<Vec<std::path::PathBuf>>,
) -> Result<Vec<ScanPreviewItem>, ScannerError> {
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

    let mut items = Vec::with_capacity(total);
    let ini_filters = IniTokenizationConfig::default().prepare();
    let started = std::time::Instant::now();

    for (idx, candidate) in candidates.iter().enumerate() {
        if let Some(channel) = &on_progress {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let _ = channel.send(ScanEvent::Progress {
                current: idx + 1,
                total,
                folder_name: candidate.display_name.clone(),
                elapsed_ms,
                // Extrapolate from folders already finished (`idx`), not from the one
                // just starting, so the first tick reports 0 instead of a wild guess.
                eta_ms: if idx == 0 {
                    0
                } else {
                    elapsed_ms * (total - idx) as u64 / idx as u64
                },
            });
        }

        let folder_path_str = candidate.path.to_string_lossy().to_string();

        let existing = crate::repo::mod_repo::get_mod_id_and_object_id_by_path(
            pool,
            &folder_path_str,
            game_id,
        )
        .await?;

        // Run phased matcher (Quick first, then FullScoring fallback).
        let content = walker::scan_folder_content(&candidate.path, 3);
        let match_result = deep_matcher::match_folder_phased(
            candidate,
            master_db,
            &content,
            &ini_filters,
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
        let already_matched =
            check_already_matched(pool, &existing, matched_entry_key.as_deref()).await?;

        log::debug!(
            "scan_preview: item | folder={} status={match_level} confidence={confidence} score={} best={} already_in_db={already_in_db}",
            candidate.display_name,
            match_result.confidence_score(),
            matched_alias_name.as_deref().unwrap_or("-")
        );

        let db_entry = matched_alias_name
            .as_ref()
            .and_then(|name| master_db.entries.iter().find(|e| &e.name == name));

        let raw_thumbnail = db_entry.and_then(|e| e.thumbnail_path.clone());
        let db_thumbnail = resolve_thumbnail(game_id, mods_path, db_entry, None, resource_dir);
        let tags_json =
            db_entry.map(|e| serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".to_string()));
        let metadata_json = db_entry
            .and_then(|e| e.metadata.as_ref())
            .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string()));
        let hash_db_json = db_entry
            .map(|e| serde_json::to_string(&e.hash_db).unwrap_or_else(|_| "{}".to_string()));
        let custom_skins_json = db_entry
            .map(|e| serde_json::to_string(&e.custom_skins).unwrap_or_else(|_| "[]".to_string()));

        if let Some(channel) = &on_progress {
            if let Some(ref matched) = matched_alias_name {
                let _ = channel.send(ScanEvent::Matched {
                    folder_name: candidate.display_name.clone(),
                    object_name: matched.clone(),
                    confidence: confidence.clone(),
                });
            }
        }

        // Build scored candidates from matcher's top-k for the dropdown
        let scored_candidates: Vec<ScoredCandidate> = match_result
            .candidates_topk
            .iter()
            .map(|c| ScoredCandidate {
                name: c.name.clone(),
                object_type: c.object_type.clone(),
                score_pct: score_to_percentage(c),
            })
            .collect();

        items.push(ScanPreviewItem {
            folder_path: folder_path_str,
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
        });
    }

    let matched = items
        .iter()
        .filter(|i| i.matched_entry_key.is_some())
        .count();

    log::info!(
        "scan_preview: done | game_id={game_id} folders={total} matched={matched} unmatched={} elapsed_ms={}",
        total - matched,
        started.elapsed().as_millis()
    );

    if let Some(channel) = &on_progress {
        let _ = channel.send(ScanEvent::Finished {
            matched,
            unmatched: total - matched,
        });
    }

    Ok(items)
}

async fn check_already_matched(
    pool: &SqlitePool,
    existing: &Option<(String, Option<String>)>,
    expected_entry_key: Option<&str>,
) -> Result<bool, ScannerError> {
    let Some(expected_entry_key) = expected_entry_key else {
        return Ok(false);
    };
    let (_, obj_id) = match existing {
        Some(r) => r,
        None => return Ok(false),
    };

    let Some(id) = obj_id else {
        return Ok(false);
    };

    let matched_entry_key = crate::repo::object_repo::get_matched_entry_key_by_id(pool, id).await?;

    Ok(matched_entry_key.as_deref() == Some(expected_entry_key))
}

fn resolve_thumbnail(
    _game_id: &str,
    mods_path: &Path,
    db_entry: Option<&types::DbEntry>,
    detected_skin: Option<&String>,
    resource_dir: Option<&Path>,
) -> Option<String> {
    let entry = db_entry?;

    let rel = if let Some(skin_name) = detected_skin {
        entry
            .custom_skins
            .iter()
            .find(|s| &s.name == skin_name)
            .and_then(|s| s.thumbnail_skin_path.clone())
            .or_else(|| entry.thumbnail_path.clone())
    } else {
        entry.thumbnail_path.clone()
    };

    // ... previous content from resolve_thumbnail ...
    let r = rel?;

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

