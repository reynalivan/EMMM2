use std::path::Path;
use tokio::task::JoinSet;

use crate::database::{mod_repo, object_repo};
use crate::services::fs_utils::file_utils::rename_cross_drive_fallback;
use crate::services::path_key::folder_path_key;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};

use super::core_ops::standardize_prefix;

/// Performs an atomic, parallel, mass-toggle of a given list of mods.
///
/// 1. Runs all `std::fs::rename` operations concurrently using a `JoinSet` of `spawn_blocking` tasks.
/// 2. Collects all successfully renamed paths.
/// 3. Initiates a single SQLite transaction to update the DB paths and object relationships.
///
/// Returns `(successful_count, failures_as_warnings)`.
pub async fn bulk_toggle_mods(
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    mods_path: &str,
    game_id: &str,
    mods_to_toggle: Vec<(String, String)>, // (id, relative_folder_path)
    target_enabled: bool,
    disabled_reason: Option<&str>,
) -> Result<(usize, Vec<String>), String> {
    if mods_to_toggle.is_empty() {
        return Ok((0, vec![]));
    }

    let _guard = SuppressionGuard::new(&watcher_state.suppressor);

    let mut fs_set = JoinSet::new();
    let mut warnings = Vec::new();

    // ─── Phase 1: Parallel FS Renames ─────────────────────────────────────
    let mut total_tasks = 0;
    for (id, rel_path) in mods_to_toggle {
        let old_abs = Path::new(mods_path).join(&rel_path);
        let old_name = old_abs
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let new_name = standardize_prefix(&old_name, target_enabled);

        if new_name == old_name || new_name.is_empty() {
            // Unchanged or empty, skip
            continue;
        }

        let new_abs = old_abs.with_file_name(&new_name);

        let mods_path_clone = mods_path.to_string();
        let rel_path_clone = rel_path.clone();

        total_tasks += 1;
        fs_set.spawn_blocking(move || {
            // Guard against conflict
            if new_abs.exists() && old_abs != new_abs {
                // Ignore case-only rename conflicts on Windows by checking if old!=new as strings
                return Err((
                    id,
                    format!("Target folder already exists: {}", new_abs.display()),
                ));
            }

            match rename_cross_drive_fallback(&old_abs, &new_abs) {
                Ok(_) => {
                    let new_rel = new_abs
                        .strip_prefix(&mods_path_clone)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| new_abs.to_string_lossy().to_string());
                    Ok((id, rel_path_clone, new_rel))
                }
                Err(e) => Err((id, format!("Failed to rename folder: {}", e))),
            }
        });
    }

    if total_tasks == 0 {
        return Ok((0, vec![]));
    }

    // Collect results
    let mut successes = Vec::new();
    while let Some(res) = fs_set.join_next().await {
        match res {
            Ok(Ok((id, old_rel, new_rel))) => successes.push((id, old_rel, new_rel)),
            Ok(Err((id, err_msg))) => warnings.push(format!("Mod {}: {}", id, err_msg)),
            Err(e) => warnings.push(format!("Task execution panicked/cancelled: {}", e)),
        }
    }

    if successes.is_empty() {
        return Ok((0, warnings));
    }

    // ─── Phase 2: Single Transactional DB Commit ──────────────────────────
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let new_status = if target_enabled {
        "ENABLED"
    } else {
        "DISABLED"
    };

    for (id, old_rel, new_rel) in &successes {
        // Update mod identity
        sqlx::query(
            "UPDATE mods SET folder_path = ?, folder_path_key = ?, status = ?, disabled_reason = ? WHERE id = ?",
        )
        .bind(new_rel)
        .bind(folder_path_key(new_rel, Some(mods_path)))
        .bind(new_status)
        .bind(disabled_reason)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        // If it was a root-level mod folder, we must also update its object & children
        let rel_components: Vec<_> = Path::new(old_rel).components().collect();
        if rel_components.len() == 1 {
            object_repo::update_object_folder_path(&mut *tx, game_id, old_rel, new_rel)
                .await
                .map_err(|e| e.to_string())?;

            for (old_sep, new_sep) in [("\\", "\\"), ("/", "/")] {
                let _ = mod_repo::update_child_paths_tx(
                    &mut *tx,
                    game_id,
                    &format!("{}{}", old_rel, old_sep),
                    &format!("{}{}", new_rel, new_sep),
                    Some(mods_path),
                )
                .await;
            }
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    // Clear disabled reason if enabled (we handle this during the UPDATE, but for safety it's already done)

    Ok((successes.len(), warnings))
}
