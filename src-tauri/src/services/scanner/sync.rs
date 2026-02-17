use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;
use std::path::Path;
use tauri::ipc::Channel;

use crate::services::scanner::{deep_matcher, types, walker};

#[derive(Debug, Serialize, Clone)]
pub struct SyncResult {
    pub total_scanned: usize,
    pub new_mods: usize,
    pub updated_mods: usize,
    pub deleted_mods: usize,
    pub new_objects: usize,
}

/// A single preview item returned by scan_preview (before user confirms).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanPreviewItem {
    pub folder_path: String,
    pub display_name: String,
    pub is_disabled: bool,
    pub matched_object: Option<String>,
    pub match_level: String,
    pub confidence: String,
    pub match_detail: Option<String>,
    pub detected_skin: Option<String>,
    pub object_type: Option<String>,
    pub thumbnail_path: Option<String>,
    /// Tags from MasterDB entry (JSON array string)
    pub tags_json: Option<String>,
    /// Metadata from MasterDB entry (JSON object string)
    pub metadata_json: Option<String>,
    /// Whether this mod already exists in DB
    pub already_in_db: bool,
    /// Whether this mod already has an object_id assigned
    pub already_matched: bool,
}

/// User-confirmed item sent back from the review modal.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedScanItem {
    pub folder_path: String,
    pub display_name: String,
    pub is_disabled: bool,
    pub matched_object: Option<String>,
    pub object_type: Option<String>,
    pub thumbnail_path: Option<String>,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub skip: bool,
}

/// Upsert the game record into the `games` table so FK constraints are satisfied.
async fn ensure_game_exists(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    game_id: &str,
    game_name: &str,
    game_type: &str,
    mods_path: &str,
) -> Result<(), String> {
    sqlx::query("INSERT OR IGNORE INTO games (id, name, game_type, path) VALUES (?, ?, ?, ?)")
        .bind(game_id)
        .bind(game_name)
        .bind(game_type)
        .bind(mods_path)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Failed to ensure game exists: {e}"))?;
    Ok(())
}

/// Phase 1: Scan folders and run Deep Matcher, return preview items without writing to DB.
pub async fn scan_preview(
    pool: &SqlitePool,
    game_id: &str,
    mods_path: &Path,
    master_db: &deep_matcher::MasterDb,
    resource_dir: Option<&Path>,
    on_progress: Option<Channel<types::ScanEvent>>,
) -> Result<Vec<ScanPreviewItem>, String> {
    let candidates = walker::scan_mod_folders(mods_path)?;
    let total = candidates.len();

    if let Some(channel) = &on_progress {
        let _ = channel.send(types::ScanEvent::Started {
            total_folders: total,
        });
    }

    let mut items = Vec::with_capacity(total);

    for (idx, candidate) in candidates.iter().enumerate() {
        if let Some(channel) = &on_progress {
            let _ = channel.send(types::ScanEvent::Progress {
                current: idx + 1,
                folder_name: candidate.display_name.clone(),
            });
        }

        let folder_path_str = candidate.path.to_string_lossy().to_string();

        // Check if already in DB
        let existing =
            sqlx::query("SELECT id, object_id FROM mods WHERE folder_path = ? AND game_id = ?")
                .bind(&folder_path_str)
                .bind(game_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;

        let already_in_db = existing.is_some();
        // Verify that the mod's object_id points to an actual existing object
        // (prevents stale references from auto-skipping re-scan)
        let already_matched = if let Some(ref row) = existing {
            if let Some(obj_id) = row
                .try_get::<Option<String>, _>("object_id")
                .unwrap_or(None)
            {
                let obj_exists = sqlx::query("SELECT 1 FROM objects WHERE id = ?")
                    .bind(&obj_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| e.to_string())?;
                obj_exists.is_some()
            } else {
                false
            }
        } else {
            false
        };

        // Run Deep Matcher
        let content = walker::scan_folder_content(&candidate.path, 3);
        let match_result = deep_matcher::match_folder(candidate, master_db, &content);

        let (matched_object, match_level, confidence, match_detail, detected_skin, object_type) =
            if match_result.level != deep_matcher::MatchLevel::Unmatched {
                (
                    Some(match_result.object_name.clone()),
                    types::match_level_label(&match_result.level).to_string(),
                    types::confidence_label(&match_result.level).to_string(),
                    if match_result.detail.is_empty() {
                        None
                    } else {
                        Some(match_result.detail.clone())
                    },
                    match_result.detected_skin.clone(),
                    Some(match_result.object_type.clone()),
                )
            } else {
                (
                    None,
                    "Unmatched".to_string(),
                    "None".to_string(),
                    None,
                    None,
                    Some("Other".to_string()),
                )
            };

        // Resolve thumbnail + tags + metadata from MasterDB
        let db_entry = matched_object
            .as_ref()
            .and_then(|name| master_db.entries.iter().find(|e| &e.name == name));

        let db_thumbnail = db_entry.and_then(|entry| {
            let rel = if let Some(ref skin_name) = detected_skin {
                entry
                    .custom_skins
                    .iter()
                    .find(|s| &s.name == skin_name)
                    .and_then(|s| s.thumbnail_skin_path.clone())
                    .or_else(|| entry.thumbnail_path.clone())
            } else {
                entry.thumbnail_path.clone()
            };
            rel.and_then(|r| {
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
            })
        });

        let tags_json =
            db_entry.map(|e| serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".to_string()));
        let metadata_json = db_entry
            .and_then(|e| e.metadata.as_ref())
            .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string()));

        if let Some(channel) = &on_progress {
            if matched_object.is_some() {
                let _ = channel.send(types::ScanEvent::Matched {
                    folder_name: candidate.display_name.clone(),
                    object_name: matched_object.clone().unwrap_or_default(),
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
            match_detail,
            detected_skin,
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
        let _ = channel.send(types::ScanEvent::Finished {
            matched,
            unmatched: total - matched,
        });
    }

    Ok(items)
}

/// Phase 2: Commit user-confirmed scan results to DB.
pub async fn commit_scan_results(
    pool: &SqlitePool,
    game_id: &str,
    game_name: &str,
    game_type: &str,
    mods_path: &str,
    items: Vec<ConfirmedScanItem>,
    resource_dir: Option<&Path>,
) -> Result<SyncResult, String> {
    let _ = resource_dir; // reserved for future thumbnail resolution
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Ensure game record exists (fixes FK constraint)
    ensure_game_exists(&mut tx, game_id, game_name, game_type, mods_path).await?;

    let total = items.len();
    let mut new_mods_count = 0;
    let mut updated_mods_count = 0;
    let mut new_objects_count = 0;
    let mut processed_paths = HashSet::new();

    for item in &items {
        if item.skip {
            processed_paths.insert(item.folder_path.clone());
            continue;
        }

        let current_status = if item.is_disabled {
            "DISABLED"
        } else {
            "ENABLED"
        };

        // Check if mod exists
        let existing = sqlx::query(
            "SELECT id, object_id, status FROM mods WHERE folder_path = ? AND game_id = ?",
        )
        .bind(&item.folder_path)
        .bind(game_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        let mod_id: String;

        if let Some(row) = existing {
            mod_id = row.try_get("id").map_err(|e| e.to_string())?;
            let db_status: String = row.try_get("status").map_err(|e| e.to_string())?;
            if db_status != current_status {
                sqlx::query("UPDATE mods SET status = ? WHERE id = ?")
                    .bind(current_status)
                    .bind(&mod_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                updated_mods_count += 1;
            }
        } else {
            mod_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_type, is_favorite) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&mod_id)
            .bind(game_id)
            .bind(&item.display_name)
            .bind(&item.folder_path)
            .bind(current_status)
            .bind(item.object_type.as_deref().unwrap_or("Other"))
            .bind(false)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            new_mods_count += 1;
        }

        processed_paths.insert(item.folder_path.clone());

        // Link to object if matched; otherwise auto-categorize as "Other"
        if let Some(ref obj_name) = item.matched_object {
            let obj_type = item.object_type.as_deref().unwrap_or("Other");
            let db_thumb = item.thumbnail_path.as_deref();
            let tags = item.tags_json.as_deref().unwrap_or("[]");
            let meta = item.metadata_json.as_deref().unwrap_or("{}");

            let object_id = ensure_object_exists(
                &mut tx,
                game_id,
                obj_name,
                obj_type,
                db_thumb,
                tags,
                meta,
                &mut new_objects_count,
            )
            .await?;

            sqlx::query("UPDATE mods SET object_id = ?, object_type = ? WHERE id = ?")
                .bind(&object_id)
                .bind(obj_type)
                .bind(&mod_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            // Unmatched folder → auto-create "Other" object using display name
            let object_id = ensure_object_exists(
                &mut tx,
                game_id,
                &item.display_name,
                "Other",
                None,
                "[]",
                "{}",
                &mut new_objects_count,
            )
            .await?;

            sqlx::query("UPDATE mods SET object_id = ?, object_type = ? WHERE id = ?")
                .bind(&object_id)
                .bind("Other")
                .bind(&mod_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // Handle deletions (mods in DB but not on disk)
    let all_mods = sqlx::query("SELECT id, folder_path FROM mods WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let mut deleted_mods_count = 0;
    for row in all_mods {
        let fp: String = row.try_get("folder_path").map_err(|e| e.to_string())?;
        if !processed_paths.contains(&fp) && !Path::new(&fp).exists() {
            let id: String = row.try_get("id").map_err(|e| e.to_string())?;
            sqlx::query("DELETE FROM mods WHERE id = ?")
                .bind(&id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            deleted_mods_count += 1;
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(SyncResult {
        total_scanned: total,
        new_mods: new_mods_count,
        updated_mods: updated_mods_count,
        deleted_mods: deleted_mods_count,
        new_objects: new_objects_count,
    })
}

/// Legacy sync_with_db — kept for backward compatibility, now upserts game first.
#[allow(clippy::too_many_arguments)]
pub async fn sync_with_db(
    pool: &SqlitePool,
    game_id: &str,
    game_name: &str,
    game_type: &str,
    mods_path: &Path,
    master_db: &deep_matcher::MasterDb,
    resource_dir: Option<&Path>,
    on_progress: Option<Channel<types::ScanEvent>>,
) -> Result<SyncResult, String> {
    // 1. Scan Folders
    let candidates = walker::scan_mod_folders(mods_path)?;
    let total = candidates.len();

    if let Some(channel) = &on_progress {
        let _ = channel.send(types::ScanEvent::Started {
            total_folders: total,
        });
    }

    // 2. Begin Transaction
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // 2a. Ensure game record exists (fixes FK constraint)
    ensure_game_exists(
        &mut tx,
        game_id,
        game_name,
        game_type,
        &mods_path.to_string_lossy(),
    )
    .await?;

    let mut new_mods_count = 0;
    let mut updated_mods_count = 0;
    let mut new_objects_count = 0;

    let mut processed_ids = HashSet::new();

    for (idx, candidate) in candidates.iter().enumerate() {
        if let Some(channel) = &on_progress {
            let _ = channel.send(types::ScanEvent::Progress {
                current: idx + 1,
                folder_name: candidate.display_name.clone(),
            });
        }

        let folder_path_str = candidate.path.to_string_lossy().to_string();
        let current_status = if candidate.is_disabled {
            "DISABLED"
        } else {
            "ENABLED"
        };

        // Check if exists
        let existing = sqlx::query(
            "SELECT id, object_id, actual_name, status FROM mods WHERE folder_path = ? AND game_id = ?"
        )
        .bind(&folder_path_str)
        .bind(game_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        let mod_id: String;

        if let Some(row) = existing {
            // Update status if changed
            mod_id = row.try_get("id").map_err(|e| e.to_string())?;
            let db_status: String = row.try_get("status").map_err(|e| e.to_string())?;

            if db_status != current_status {
                sqlx::query("UPDATE mods SET status = ? WHERE id = ?")
                    .bind(current_status)
                    .bind(&mod_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                updated_mods_count += 1;
            }

            // Check if needs matching (no object_id or Uncategorized)
            let obj_id: Option<String> = row.try_get("object_id").unwrap_or(None);

            if obj_id.is_some() {
                processed_ids.insert(mod_id);
                continue; // Already matched
            }
        } else {
            // New Mod
            mod_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO mods (id, game_id, actual_name, folder_path, status, object_type, is_favorite) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&mod_id)
            .bind(game_id)
            .bind(&candidate.display_name)
            .bind(&folder_path_str)
            .bind(current_status)
            .bind("Other") // Default object_type
            .bind(false)           // Default is_favorite
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

            new_mods_count += 1;
        }

        processed_ids.insert(mod_id.clone());

        // 3. Run Deep Matcher (if new or unmatched)
        // We need content to match
        // Use scan_folder_content with max_depth=3 (standard)
        let content = walker::scan_folder_content(&candidate.path, 3);
        let match_result = deep_matcher::match_folder(candidate, master_db, &content);

        if match_result.level != deep_matcher::MatchLevel::Unmatched {
            // Found a match!
            // Look up the DB entry for thumbnail + tags + metadata
            let db_entry = master_db
                .entries
                .iter()
                .find(|e| e.name == match_result.object_name);

            // Resolve thumbnail: prefer skin thumbnail if skin detected, else base
            let db_thumbnail_relative = db_entry.and_then(|entry| {
                if let Some(ref skin_name) = match_result.detected_skin {
                    // Look for skin-specific thumbnail first
                    entry
                        .custom_skins
                        .iter()
                        .find(|s| &s.name == skin_name)
                        .and_then(|s| s.thumbnail_skin_path.clone())
                        .or_else(|| entry.thumbnail_path.clone())
                } else {
                    entry.thumbnail_path.clone()
                }
            });

            // Resolve relative path to absolute using resource_dir
            let db_thumbnail = db_thumbnail_relative.and_then(|rel_path| {
                if let Some(res_dir) = resource_dir {
                    let abs_path = res_dir.join(&rel_path);
                    if abs_path.exists() {
                        Some(abs_path.to_string_lossy().to_string())
                    } else {
                        log::warn!("MasterDB thumbnail not found: {}", abs_path.display());
                        None
                    }
                } else {
                    // No resource_dir, store as-is (fallback)
                    Some(rel_path)
                }
            });

            // Serialize tags and metadata from MasterDB entry
            let db_tags_json = db_entry
                .map(|e| serde_json::to_string(&e.tags).unwrap_or_else(|_| "[]".to_string()))
                .unwrap_or_else(|| "[]".to_string());
            let db_metadata_json = db_entry
                .and_then(|e| e.metadata.as_ref())
                .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string()))
                .unwrap_or_else(|| "{}".to_string());

            // Ensure object exists in DB
            let object_id = ensure_object_exists(
                &mut tx,
                game_id,
                &match_result.object_name,
                &match_result.object_type,
                db_thumbnail.as_deref(),
                &db_tags_json,
                &db_metadata_json,
                &mut new_objects_count,
            )
            .await?;

            // Link mod to object
            sqlx::query("UPDATE mods SET object_id = ?, object_type = ? WHERE id = ?")
                .bind(&object_id)
                .bind(&match_result.object_type)
                .bind(&mod_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            // Unmatched folder → auto-create "Other" object using display name
            let object_id = ensure_object_exists(
                &mut tx,
                game_id,
                &candidate.display_name,
                "Other",
                None,
                "[]",
                "{}",
                &mut new_objects_count,
            )
            .await?;

            sqlx::query("UPDATE mods SET object_id = ?, object_type = ? WHERE id = ?")
                .bind(&object_id)
                .bind("Other")
                .bind(&mod_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // 4. Handle Deletions (Mods in DB but not in FS)
    // Fetch all mods for this game
    let all_mods = sqlx::query("SELECT id, folder_path FROM mods WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let mut deleted_mods_count = 0;

    for row in all_mods {
        let id: String = row.try_get("id").map_err(|e| e.to_string())?;
        if !processed_ids.contains(&id) {
            // It's in DB but not found in current scan -> Deleted
            sqlx::query("DELETE FROM mods WHERE id = ?")
                .bind(&id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

            deleted_mods_count += 1;
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    if let Some(channel) = &on_progress {
        let _ = channel.send(types::ScanEvent::Finished {
            matched: new_mods_count, // Use new mods count as proxy for matched in this context
            unmatched: 0,
        });
    }

    Ok(SyncResult {
        total_scanned: total,
        new_mods: new_mods_count,
        updated_mods: updated_mods_count,
        deleted_mods: deleted_mods_count,
        new_objects: new_objects_count,
    })
}

#[allow(clippy::too_many_arguments)]
async fn ensure_object_exists(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    game_id: &str,
    obj_name: &str,
    obj_type: &str,
    db_thumbnail: Option<&str>,
    db_tags_json: &str,
    db_metadata_json: &str,
    new_objects_count: &mut usize,
) -> Result<String, String> {
    // Check if object exists by name within game
    let existing = sqlx::query(
        "SELECT id, thumbnail_path, tags, metadata FROM objects WHERE game_id = ? AND name = ?",
    )
    .bind(game_id)
    .bind(obj_name)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(row) = existing {
        let id: String = row.try_get("id").map_err(|e| e.to_string())?;
        let existing_thumb: Option<String> = row.try_get("thumbnail_path").unwrap_or(None);
        let existing_tags: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        let existing_meta: String = row.try_get("metadata").unwrap_or_else(|_| "{}".to_string());

        // Only set thumbnail from MasterDB if object has no existing thumbnail
        // (avoids overwriting user's custom thumb* files)
        if existing_thumb.is_none() {
            if let Some(thumb) = db_thumbnail {
                sqlx::query("UPDATE objects SET thumbnail_path = ? WHERE id = ?")
                    .bind(thumb)
                    .bind(&id)
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }

        // Backfill tags if currently empty (default '[]')
        if existing_tags == "[]" && db_tags_json != "[]" {
            sqlx::query("UPDATE objects SET tags = ? WHERE id = ?")
                .bind(db_tags_json)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        // Backfill metadata if currently empty (default '{}')
        if existing_meta == "{}" && db_metadata_json != "{}" {
            sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
                .bind(db_metadata_json)
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(id)
    } else {
        // Create new object with thumbnail, tags, and metadata from MasterDB
        let new_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO objects (id, game_id, name, object_type, thumbnail_path, tags, metadata) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
            .bind(&new_id)
            .bind(game_id)
            .bind(obj_name)
            .bind(obj_type)
            .bind(db_thumbnail)
            .bind(db_tags_json)
            .bind(db_metadata_json)
            .execute(&mut **tx)
            .await
            .map_err(|e| e.to_string())?;

        *new_objects_count += 1;
        Ok(new_id)
    }
}
