//! Advanced Parallel & Atomic Bulk Operations.
//!
//! NOTE: This module is currently ORPHANED and not wired to any Tauri commands.
//! The frontend currently uses the simpler sequential `bulk.rs`.
//! This is preserved as a high-quality candidate for a future parallel/atomic upgrade.

#![allow(dead_code)]

use std::path::Path;
use tokio::task::JoinSet;

use crate::repo::{mod_repo, object_repo};
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
    let db_commit_result = apply_db_bulk_updates(
        pool,
        game_id,
        mods_path,
        &successes,
        target_enabled,
        disabled_reason,
    )
    .await;

    if let Err(db_error) = db_commit_result {
        let rollback_warnings = rollback_successful_fs_renames(mods_path, &successes);
        if rollback_warnings.is_empty() {
            return Err(format!(
                "Bulk toggle DB phase failed after FS renames, rollback succeeded: {db_error}"
            ));
        }

        let rollback_detail = rollback_warnings.join("; ");
        return Err(format!(
            "Bulk toggle DB phase failed after FS renames and rollback was partial: {db_error}; rollback warnings: {rollback_detail}"
        ));
    }

    // Recompute signatures globally for this game so Corridor caches align
    let _ = crate::services::corridor_service::recompute_signature(pool, game_id, true).await;
    let _ = crate::services::corridor_service::recompute_signature(pool, game_id, false).await;

    Ok((successes.len(), warnings))
}

async fn apply_db_bulk_updates(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &str,
    successes: &[(String, String, String)],
    target_enabled: bool,
    disabled_reason: Option<&str>,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let new_status = if target_enabled {
        1 // ItemStatus::Enabled
    } else {
        0 // ItemStatus::Disabled
    };

    for (id, old_rel, new_rel) in successes {
        let conn = &mut *tx;

        sqlx::query(
            "UPDATE mods SET folder_path = ?, folder_path_key = ?, status = ?, disabled_reason = ? WHERE id = ?",
        )
        .bind(new_rel)
        .bind(folder_path_key(new_rel, Some(mods_path)))
        .bind(new_status)
        .bind(disabled_reason)
        .bind(id)
        .execute(&mut *conn)
        .await
        .map_err(|e: sqlx::Error| e.to_string())?;

        let rel_components: Vec<_> = Path::new(old_rel).components().collect();
        if rel_components.len() == 1 {
            object_repo::update_object_folder_path(&mut *conn, game_id, old_rel, new_rel)
                .await
                .map_err(|e: sqlx::Error| e.to_string())?;

            for (old_sep, new_sep) in [("\\", "\\"), ("/", "/")] {
                let _ = mod_repo::update_child_paths_tx(
                    &mut *conn,
                    game_id,
                    &format!("{}{}", old_rel, old_sep),
                    &format!("{}{}", new_rel, new_sep),
                    Some(mods_path),
                )
                .await;
            }
        }
    }

    tx.commit().await.map_err(|e| e.to_string())
}

fn rollback_successful_fs_renames(
    mods_path: &str,
    successes: &[(String, String, String)],
) -> Vec<String> {
    let mut rollback_warnings = Vec::new();

    for (id, old_rel, new_rel) in successes.iter().rev() {
        let old_abs = Path::new(mods_path).join(old_rel);
        let new_abs = Path::new(mods_path).join(new_rel);

        if !new_abs.exists() {
            rollback_warnings.push(format!(
                "Mod {}: rollback source missing on disk: {}",
                id,
                new_abs.display()
            ));
            continue;
        }

        if old_abs.exists() {
            rollback_warnings.push(format!(
                "Mod {}: rollback target already exists: {}",
                id,
                old_abs.display()
            ));
            continue;
        }

        if let Err(error) = rename_cross_drive_fallback(&new_abs, &old_abs) {
            rollback_warnings.push(format!(
                "Mod {}: rollback rename failed ({} -> {}): {}",
                id,
                new_abs.display(),
                old_abs.display(),
                error
            ));
        }
    }

    rollback_warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::{GameType, ItemStatus};
    use crate::repo::game_repo::{upsert_game, GameRow};
    use crate::services::scanner::watcher::WatcherState;
    use crate::test_utils::{init_test_db, insert_test_mod, TestModFixture};

    #[tokio::test]
    async fn bulk_toggle_rolls_back_fs_when_db_phase_fails() {
        let ctx = init_test_db().await;
        let pool = &ctx.pool;
        let watcher_state = WatcherState::new();
        let mods_root = tempfile::tempdir().expect("tempdir");
        let mods_path = mods_root.path().to_string_lossy().to_string();

        upsert_game(
            pool,
            &GameRow {
                id: "g1".to_string(),
                name: "Game 1".to_string(),
                game_type: GameType::GIMI,
                path: "C:/Game1".to_string(),
                mods_path: Some(mods_path.clone()),
                game_exe: None,
                launcher_path: None,
                loader_exe: None,
                launch_args: None,
            },
        )
        .await
        .expect("insert game");

        std::fs::create_dir_all(mods_root.path().join("ModA")).expect("create mod folder");

        insert_test_mod(
            pool,
            &TestModFixture {
                id: "m1",
                game_id: "g1",
                object_id: None,
                actual_name: "ModA",
                folder_path: "ModA",
                status: ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Other"),
                mods_path: Some(&mods_path),
            },
        )
        .await
        .expect("insert mod");

        sqlx::query("DROP TABLE mods")
            .execute(pool)
            .await
            .expect("drop mods table");

        let result = bulk_toggle_mods(
            pool,
            &watcher_state,
            &mods_path,
            "g1",
            vec![("m1".to_string(), "ModA".to_string())],
            false,
            Some("SYSTEM"),
        )
        .await;

        let error = result.expect_err("DB failure should bubble as error");
        assert!(error.contains("rollback succeeded"));
        assert!(mods_root.path().join("ModA").exists());
        assert!(!mods_root.path().join("DISABLED ModA").exists());
    }

    #[test]
    fn rollback_reports_missing_source_as_warning() {
        let mods_root = tempfile::tempdir().expect("tempdir");
        let mods_path = mods_root.path().to_string_lossy().to_string();

        let warnings = rollback_successful_fs_renames(
            &mods_path,
            &[(
                "m1".to_string(),
                "ModA".to_string(),
                "DISABLED ModA".to_string(),
            )],
        );

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("rollback source missing on disk"));
    }
}
