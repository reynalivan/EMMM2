use sqlx::{Row, SqlitePool};
use std::path::Path;
use tauri::ipc::Channel;

use super::helpers::auto_matched_candidate;
use super::types::{ScanPreviewItem, ScoredCandidate};
use crate::services::scanner::core::types::{
    match_status_label, staged_confidence_label, ScanEvent,
};
use crate::services::scanner::core::walker;
use crate::services::scanner::deep_matcher;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::models::result_summary::score_to_percentage;
use crate::services::scanner::deep_matcher::models::types;

/// Phase 1: Scan folders and run Deep Matcher, return preview items without writing to DB.
pub async fn scan_preview(
    pool: &SqlitePool,
    game_id: &str,
    mods_path: &Path,
    master_db: &deep_matcher::MasterDb,
    resource_dir: Option<&Path>,
    on_progress: Option<Channel<ScanEvent>>,
    specific_paths: Option<Vec<std::path::PathBuf>>,
) -> Result<Vec<ScanPreviewItem>, String> {
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

    let mut items = Vec::with_capacity(total);
    let ini_config = IniTokenizationConfig::default();

    for (idx, candidate) in candidates.iter().enumerate() {
        if let Some(channel) = &on_progress {
            let _ = channel.send(ScanEvent::Progress {
                current: idx + 1,
                total,
                folder_name: candidate.display_name.clone(),
                elapsed_ms: 0,
                eta_ms: 0,
            });
        }

        let folder_path_str = candidate.path.to_string_lossy().to_string();

        let existing =
            sqlx::query("SELECT id, object_id FROM mods WHERE folder_path = ? AND game_id = ?")
                .bind(&folder_path_str)
                .bind(game_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;

        let already_in_db = existing.is_some();
        let already_matched = check_already_matched(pool, &existing, master_db).await?;

        // Run phased matcher (Quick first, then FullScoring fallback).
        let content = walker::scan_folder_content(&candidate.path, 3);
        let match_result = deep_matcher::match_folder_phased(
            candidate,
            master_db,
            &content,
            &ini_config,
            &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig::default(),
        );
        let auto_candidate = auto_matched_candidate(&match_result);

        let matched_object = auto_candidate.map(|c| c.name.clone());
        let object_type = auto_candidate.map(|c| c.object_type.clone());
        let match_level = match_status_label(&match_result.status).to_string();
        let confidence = staged_confidence_label(&match_result).to_string();
        let match_detail = Some(match_result.summary());

        let db_entry = matched_object
            .as_ref()
            .and_then(|name| master_db.entries.iter().find(|e| &e.name == name));

        let db_thumbnail = resolve_thumbnail(db_entry, None, resource_dir);
        let tags_json =
            db_entry.map(|e| serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".to_string()));
        let metadata_json = db_entry
            .and_then(|e| e.metadata.as_ref())
            .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string()));

        if let Some(channel) = &on_progress {
            if let Some(ref matched) = matched_object {
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
            matched_object,
            match_level,
            confidence,
            confidence_score: match_result.confidence_score(),
            match_detail,
            detected_skin: None,
            object_type,
            thumbnail_path: db_thumbnail,
            tags_json,
            metadata_json,
            already_in_db,
            already_matched,
            scored_candidates,
        });
    }

    if let Some(channel) = &on_progress {
        let matched = items.iter().filter(|i| i.matched_object.is_some()).count();
        let _ = channel.send(ScanEvent::Finished {
            matched,
            unmatched: total - matched,
        });
    }

    Ok(items)
}

async fn check_already_matched(
    pool: &SqlitePool,
    existing: &Option<sqlx::sqlite::SqliteRow>,
    master_db: &deep_matcher::MasterDb,
) -> Result<bool, String> {
    let row = match existing {
        Some(r) => r,
        None => return Ok(false),
    };

    let obj_id: Option<String> = row.try_get("object_id").unwrap_or(None);
    let Some(id) = obj_id else {
        return Ok(false);
    };

    let obj_row = sqlx::query("SELECT name FROM objects WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    match obj_row {
        Some(r) => {
            let obj_name: String = r.try_get("name").unwrap_or_default();
            // Only "already matched" if the object name exists in MasterDB
            // (i.e., it was matched to a real character/weapon, not a folder-name placeholder)
            Ok(master_db.entries.iter().any(|e| e.name == obj_name))
        }
        None => Ok(false),
    }
}

fn resolve_thumbnail(
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
        Some(r)
    }
}

/// Computes the percentage score for a specific batch of candidates against a folder.
/// Used for lazy loading dropdown percentages without scoring all DB entries.
pub fn score_candidates_batch(
    folder_path: &str,
    master_db: &deep_matcher::MasterDb,
    candidate_names: Vec<String>,
) -> std::collections::HashMap<String, u8> {
    use std::collections::HashMap;

    let mut results = HashMap::new();
    let path = Path::new(folder_path);

    if !path.exists() {
        return results;
    }

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
    let mut signal_cache =
        crate::services::scanner::deep_matcher::state::signal_cache::SignalCache::new();

    // Map requested names to entry IDs
    let entry_ids: Vec<usize> = master_db
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| candidate_names.contains(&e.name))
        .map(|(id, _)| id)
        .collect();

    if entry_ids.is_empty() {
        return results;
    }

    // Rather than hardcoding the pipeline logic here, we just run the specialized forced scoring
    // pipeline. It bypasses early exits and prunes to give us exact scores for the requested items.
    let match_result = deep_matcher::score_forced_candidates(
        &candidate,
        master_db,
        &content,
        &ini_config,
        &ai_config,
        &mut signal_cache,
        &entry_ids,
    );

    // Provide baseline scores (0%) for requested names in case matcher filtered them out
    for name in &candidate_names {
        results.insert(name.clone(), 0);
    }

    // Update with actual scores if they survived to the final candidates list
    for c in &match_result.candidates_all {
        if candidate_names.contains(&c.name) {
            results.insert(c.name.clone(), score_to_percentage(c));
        }
    }

    results
}
