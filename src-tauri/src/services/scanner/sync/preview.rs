use sqlx::{Row, SqlitePool};
use std::path::Path;
use tauri::ipc::Channel;

use super::helpers::auto_matched_candidate;
use super::types::ScanPreviewItem;
use crate::services::scanner::core::types::{
    match_status_label, staged_confidence_label, ScanEvent,
};
use crate::services::scanner::core::walker;
use crate::services::scanner::deep_matcher;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::models::types;

/// Phase 1: Scan folders and run Deep Matcher, return preview items without writing to DB.
pub async fn scan_preview(
    pool: &SqlitePool,
    game_id: &str,
    mods_path: &Path,
    master_db: &deep_matcher::MasterDb,
    resource_dir: Option<&Path>,
    on_progress: Option<Channel<ScanEvent>>,
) -> Result<Vec<ScanPreviewItem>, String> {
    let candidates = walker::scan_mod_folders(mods_path)?;
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
        let already_matched = check_already_matched(pool, &existing).await?;

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
) -> Result<bool, String> {
    let row = match existing {
        Some(r) => r,
        None => return Ok(false),
    };

    let obj_id: Option<String> = row.try_get("object_id").unwrap_or(None);
    if let Some(id) = obj_id {
        let obj_exists = sqlx::query("SELECT 1 FROM objects WHERE id = ?")
            .bind(&id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(obj_exists.is_some())
    } else {
        Ok(false)
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
