use super::types::{
    ApplyCollectionResult, CollectionsUndoState, ModState, SnapshotEntry, UndoCollectionResult,
    UndoSnapshot,
};
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use regex::Regex;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

static DISABLED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(disabled|disable|dis)[_\-\s]*").expect("valid regex"));

pub async fn apply_collection(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    undo_state: &CollectionsUndoState,
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

    let target_ids: Vec<String> = sqlx::query_scalar(
        "SELECT ci.mod_id FROM collection_items ci JOIN mods m ON m.id = ci.mod_id WHERE ci.collection_id = ? AND m.game_id = ?",
    )
    .bind(collection_id)
    .bind(game_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

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

    apply_state_change(
        pool,
        watcher_state,
        undo_state,
        game_id,
        states,
        &target_ids,
    )
    .await
}

pub async fn undo_collection_apply(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    undo_state: &CollectionsUndoState,
    game_id: &str,
) -> Result<UndoCollectionResult, String> {
    let snapshot = undo_state
        .take()
        .ok_or("No collection snapshot available to undo")?;
    if snapshot.game_id != game_id {
        return Err("Snapshot belongs to a different game".to_string());
    }

    let target_ids: Vec<String> = snapshot
        .entries
        .iter()
        .map(|entry| entry.mod_id.clone())
        .collect();
    let states = fetch_mod_states(pool, game_id, &target_ids).await?;
    let desired: HashMap<String, String> = snapshot
        .entries
        .into_iter()
        .map(|entry| (entry.mod_id, entry.previous_status))
        .collect();

    let restored = apply_with_desired_status(pool, watcher_state, states, desired).await?;
    Ok(UndoCollectionResult {
        restored_count: restored,
    })
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
    undo_state: &CollectionsUndoState,
    game_id: &str,
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

    let snapshot_entries: Vec<SnapshotEntry> = states
        .iter()
        .map(|state| SnapshotEntry {
            mod_id: state.id.clone(),
            previous_status: state.status.clone(),
        })
        .collect();

    let changed = apply_with_desired_status(pool, watcher_state, states, desired).await?;
    if changed > 0 {
        undo_state.set(UndoSnapshot {
            game_id: game_id.to_string(),
            entries: snapshot_entries,
        });
    }

    Ok(ApplyCollectionResult {
        changed_count: changed,
        warnings: Vec::new(),
    })
}

async fn apply_with_desired_status(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    states: Vec<ModState>,
    desired: HashMap<String, String>,
) -> Result<usize, String> {
    let mut updates = Vec::new();

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

    Ok(updates.len())
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
