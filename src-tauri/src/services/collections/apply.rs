use super::types::{ApplyCollectionResult, ModState};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use regex::Regex;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use uuid::Uuid;

static DISABLED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(disabled|disable|dis)[_\-\s]*").expect("valid regex"));

pub async fn apply_collection(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    collection_id: &str,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<ApplyCollectionResult, String> {
    let (collection_name, is_safe_context): (String, bool) = sqlx::query_as(
        "SELECT name, is_safe_context FROM collections WHERE id = ? AND game_id = ?",
    )
    .bind(collection_id)
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or("Collection not found")?;

    if safe_mode_enabled && !is_safe_context {
        return Err(
            "Collection contains non-safe context. Disable Safe Mode to apply.".to_string(),
        );
    }

    // Step 1: Get mod_ids that still exist in the mods table
    let target_ids: Vec<String> = sqlx::query_scalar(
        "SELECT ci.mod_id FROM collection_items ci JOIN mods m ON m.id = ci.mod_id WHERE ci.collection_id = ? AND m.game_id = ?",
    )
    .bind(collection_id)
    .bind(game_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Step 2: Reconcile orphaned items by mod_path (US-8.3 fallback)
    let orphaned: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT ci.mod_id, ci.mod_path FROM collection_items ci WHERE ci.collection_id = ? AND ci.mod_id NOT IN (SELECT id FROM mods WHERE game_id = ?)",
    )
    .bind(collection_id)
    .bind(game_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut reconciled_ids = target_ids;
    let mut reconcile_warnings = Vec::new();

    for (old_id, maybe_path) in &orphaned {
        if let Some(path) = maybe_path {
            let found: Option<String> =
                sqlx::query_scalar("SELECT id FROM mods WHERE folder_path = ? AND game_id = ?")
                    .bind(path)
                    .bind(game_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| e.to_string())?;

            if let Some(new_id) = found {
                // Re-link: update collection_items to point to the new mod ID
                sqlx::query(
                    "UPDATE collection_items SET mod_id = ? WHERE collection_id = ? AND mod_id = ?",
                )
                .bind(&new_id)
                .bind(collection_id)
                .bind(old_id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
                reconciled_ids.push(new_id);
            } else {
                let name = Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| old_id.clone());
                reconcile_warnings.push(format!("Skipping missing mod: {}", name));
            }
        } else {
            reconcile_warnings.push(format!("Skipping orphaned mod (no path): {}", old_id));
        }
    }

    let target_ids = reconciled_ids;

    if target_ids.is_empty() {
        return Ok(ApplyCollectionResult {
            changed_count: 0,
            warnings: vec![format!("Collection '{collection_name}' has no items")],
        });
    }

    let mut states = fetch_mod_states(pool, game_id, &target_ids).await?;
    let object_ids: Vec<String> = sqlx::query_scalar("SELECT DISTINCT object_id FROM mods WHERE id IN (SELECT mod_id FROM collection_items WHERE collection_id = ?) AND object_id IS NOT NULL")
        .bind(collection_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let conflicts = fetch_enabled_conflicts(pool, game_id, &target_ids, &object_ids).await?;
    states.extend(conflicts);

    snapshot_current_state(pool, game_id, safe_mode_enabled).await?;

    let mut result = apply_state_change(pool, watcher_state, states, &target_ids).await?;
    result.warnings.extend(reconcile_warnings);
    Ok(result)
}

async fn snapshot_current_state(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode_enabled: bool,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Delete existing snapshot collection mapping and the collection itself for this game
    sqlx::query("DELETE FROM collection_items WHERE collection_id IN (SELECT id FROM collections WHERE game_id = ? AND is_last_unsaved = 1)")
        .bind(game_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM collections WHERE game_id = ? AND is_last_unsaved = 1")
        .bind(game_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let currently_enabled: Vec<(String, String)> =
        sqlx::query_as("SELECT id, folder_path FROM mods WHERE game_id = ? AND status = 'ENABLED'")
            .bind(game_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

    let snapshot_id = Uuid::new_v4().to_string();
    let name = format!("Unsaved {}", Uuid::new_v4());

    sqlx::query("INSERT INTO collections (id, name, game_id, is_safe_context, is_last_unsaved) VALUES (?, ?, ?, ?, 1)")
        .bind(&snapshot_id)
        .bind(&name)
        .bind(game_id)
        .bind(safe_mode_enabled)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    for (mod_id, mod_path) in currently_enabled {
        sqlx::query(
            "INSERT INTO collection_items (collection_id, mod_id, mod_path) VALUES (?, ?, ?)",
        )
        .bind(&snapshot_id)
        .bind(&mod_id)
        .bind(&mod_path)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn fetch_mod_states(
    pool: &SqlitePool,
    game_id: &str,
    ids: &[String],
) -> Result<Vec<ModState>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT id, folder_path, status FROM mods WHERE game_id = ");
    qb.push_bind(game_id).push(" AND id IN (");
    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    qb.push(")");

    qb.build_query_as::<(String, String, String)>()
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(id, folder_path, status)| ModState {
                    id,
                    folder_path,
                    status,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

async fn fetch_enabled_conflicts(
    pool: &SqlitePool,
    game_id: &str,
    target_ids: &[String],
    object_ids: &[String],
) -> Result<Vec<ModState>, String> {
    if object_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<'_, Sqlite> =
        QueryBuilder::new("SELECT id, folder_path, status FROM mods WHERE game_id = ");
    qb.push_bind(game_id)
        .push(" AND status = 'ENABLED' AND object_id IN (");

    let mut object_separated = qb.separated(", ");
    for object_id in object_ids {
        object_separated.push_bind(object_id);
    }
    qb.push(")");

    if !target_ids.is_empty() {
        qb.push(" AND id NOT IN (");
        let mut id_separated = qb.separated(", ");
        for id in target_ids {
            id_separated.push_bind(id);
        }
        qb.push(")");
    }

    qb.build_query_as::<(String, String, String)>()
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(id, folder_path, status)| ModState {
                    id,
                    folder_path,
                    status,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

async fn apply_state_change(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    states: Vec<ModState>,
    target_ids: &[String],
) -> Result<ApplyCollectionResult, String> {
    let desired: HashMap<String, String> = states
        .iter()
        .map(|state| {
            let target = if target_ids.contains(&state.id) {
                "ENABLED"
            } else {
                "DISABLED"
            };
            (state.id.clone(), target.to_string())
        })
        .collect();

    let (changed, warnings) =
        apply_with_desired_status(pool, watcher_state, states, desired).await?;

    Ok(ApplyCollectionResult {
        changed_count: changed,
        warnings,
    })
}

async fn apply_with_desired_status(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    states: Vec<ModState>,
    desired: HashMap<String, String>,
) -> Result<(usize, Vec<String>), String> {
    let mut updates = Vec::new();
    let mut warnings = Vec::new();

    {
        let _guard = SuppressionGuard::new(&watcher_state.suppressor);
        for state in &states {
            let next_status = desired
                .get(&state.id)
                .cloned()
                .unwrap_or_else(|| state.status.clone());
            if state.status == next_status {
                continue;
            }

            let new_path = rename_for_status(&state.folder_path, next_status == "ENABLED")?;
            if let Some(path) = new_path {
                if !Path::new(&state.folder_path).exists() {
                    let folder_name = Path::new(&state.folder_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| state.id.clone());
                    warnings.push(format!("Skipping missing mod: {}", folder_name));
                    continue;
                }
                fs::rename(&state.folder_path, &path).map_err(|e| e.to_string())?;
                updates.push((state.id.clone(), next_status, path));
                continue;
            }

            updates.push((state.id.clone(), next_status, state.folder_path.clone()));
        }
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    for (id, status, folder_path) in &updates {
        sqlx::query("UPDATE mods SET status = ?, folder_path = ? WHERE id = ?")
            .bind(status)
            .bind(folder_path)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;

    Ok((updates.len(), warnings))
}

fn rename_for_status(path: &str, to_enabled: bool) -> Result<Option<String>, String> {
    let old = Path::new(path);
    let parent = old.parent().ok_or("Invalid mod folder path")?;
    let name = old
        .file_name()
        .ok_or("Invalid mod folder name")?
        .to_string_lossy()
        .to_string();

    let clean_name = DISABLED_RE.replace(&name, "").trim().to_string();
    let next_name = if to_enabled {
        clean_name
    } else {
        format!("DISABLED {clean_name}")
    };

    if next_name == name {
        return Ok(None);
    }

    let next = parent.join(next_name);
    if next.exists() {
        return Err(format!("Target path already exists: {}", next.display()));
    }
    Ok(Some(next.to_string_lossy().to_string()))
}
