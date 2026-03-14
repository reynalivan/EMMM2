use crate::commands::mods::mod_core_cmds::toggle_mod_inner;
use crate::database::{collection_repo, game_repo, mod_repo};
use crate::services::scanner::watcher::WatcherState;
use serde::Serialize;
use sqlx::SqlitePool;

pub struct PrivacyManager;

#[derive(Debug)]
pub enum Mode {
    SFW,
    NSFW,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorridorSwitchResult {
    pub disabled_count: usize,
    pub restored_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModeSwitchPreview {
    pub target_coll_id: Option<String>,
    pub target_name: String,
}

impl PrivacyManager {
    /// Preview the target corridor state for confirmation dialogs.
    pub async fn preview_mode_switch(
        target_mode: Mode,
        pool: &SqlitePool,
        game_id: &str,
    ) -> Result<ModeSwitchPreview, String> {
        let target_safe = matches!(target_mode, Mode::SFW);

        let target_coll_id =
            collection_repo::get_last_unsaved_collection_id_pool(pool, game_id, target_safe)
                .await
                .map_err(|e| e.to_string())?;

        let target_name = if let Some(ref id) = target_coll_id {
            sqlx::query_scalar::<_, String>("SELECT name FROM collections WHERE id = ?")
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
                .unwrap_or_else(|| "Saved State".to_string())
        } else {
            "Empty State (All Disabled)".to_string()
        };

        Ok(ModeSwitchPreview {
            target_coll_id,
            target_name,
        })
    }

    /// Executes the atomic Corridor Handoff between SFW and NSFW modes.
    ///
    /// 1. Snapshot leaving corridor's enabled mods as an `is_last_unsaved` collection
    /// 2. Disable ALL leaving corridor mods (DB + disk rename)
    /// 3. Lookup target corridor's last saved state (is_last_unsaved collection)
    /// 4. Restore target corridor by re-enabling those mods
    /// 5. Return counts + any missing-mod warnings
    pub async fn switch_mode(
        target_mode: Mode,
        pool: &SqlitePool,
        watcher_state: &WatcherState,
        game_id: &str,
    ) -> Result<CorridorSwitchResult, String> {
        let leaving_safe = matches!(target_mode, Mode::NSFW); // switching TO nsfw = leaving SFW
        let target_safe = matches!(target_mode, Mode::SFW);

        // ─── STEP 1: Snapshot leaving corridor ──────────────────────────
        // Reuses the collections system's snapshot mechanism.
        // Creates/replaces an is_last_unsaved=1 collection for (game_id, leaving_is_safe_context).
        crate::services::collections::snapshot_current_state(pool, game_id, leaving_safe).await?;

        // ─── STEP 2: Disable ALL enabled mods indiscriminately ──────────
        // Guarantee 100% clean-slate before target corridor restoration
        let disabled_count = Self::disable_all_enabled_mods(pool, watcher_state, game_id).await?;

        // ─── STEP 3: Lookup target corridor's last state ────────────────
        let target_coll_id =
            collection_repo::get_last_unsaved_collection_id_pool(pool, game_id, target_safe)
                .await
                .map_err(|e| e.to_string())?;

        // ─── STEP 4: Restore target corridor from its saved collection ──
        let (restored_count, warnings) = if let Some(ref coll_id) = target_coll_id {
            Self::restore_corridor(pool, watcher_state, coll_id, game_id).await?
        } else {
            // No saved state for this corridor — fresh start (all disabled)
            (0, vec![])
        };

        Ok(CorridorSwitchResult {
            disabled_count,
            restored_count,
            warnings,
        })
    }

    /// Disables ALL ENABLED mods for a game unconditionally.
    /// This enforces a 100% clean-slate before restoring the target corridor,
    /// preventing rogue background mods from bleeding through.
    async fn disable_all_enabled_mods(
        pool: &SqlitePool,
        watcher_state: &WatcherState,
        game_id: &str,
    ) -> Result<usize, String> {
        let enabled_mods = collection_repo::get_enabled_mod_id_and_paths(pool, game_id)
            .await
            .map_err(|e| e.to_string())?;

        if enabled_mods.is_empty() {
            return Ok(0);
        }

        let game_paths = game_repo::get_all_game_mod_paths(pool)
            .await
            .map_err(|e| e.to_string())?;

        let mods_path = game_paths.get(game_id).cloned().unwrap_or_default();
        let mut disabled_count = 0;
        let mut disabled_ids = Vec::new();

        for (id, fp) in &enabled_mods {
            // Build absolute path from relative folder_path
            let abs_path = if !mods_path.is_empty() {
                std::path::Path::new(&mods_path)
                    .join(fp)
                    .to_string_lossy()
                    .to_string()
            } else {
                log::warn!(
                    "No mods_path found for game '{}', skipping mod '{}'",
                    game_id,
                    fp
                );
                continue;
            };

            // Physically rename to add DISABLED prefix
            match toggle_mod_inner(watcher_state, abs_path, false).await {
                Ok(new_path) => {
                    // Sync the new path back to DB
                    if new_path != *fp {
                        let rel_path = std::path::Path::new(&new_path)
                            .strip_prefix(&mods_path)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(new_path.clone());

                        let _ = mod_repo::update_mod_path_by_id(pool, id, &rel_path).await;

                        // Detect top-level folder renames (Flat Mod / Object directory)
                        let rel_components: Vec<_> =
                            std::path::Path::new(fp).components().collect();
                        if rel_components.len() == 1 {
                            let _ = crate::database::object_repo::update_object_folder_path(
                                pool, game_id, fp, &rel_path,
                            )
                            .await;

                            // Update child paths for nested mods
                            let old_prefix = format!("{}\\", fp);
                            let new_prefix = format!("{}\\", rel_path);
                            let old_prefix_fwd = format!("{}/", fp);
                            let new_prefix_fwd = format!("{}/", rel_path);

                            let _ = mod_repo::update_child_paths(
                                pool,
                                game_id,
                                &old_prefix,
                                &new_prefix,
                            )
                            .await;
                            let _ = mod_repo::update_child_paths(
                                pool,
                                game_id,
                                &old_prefix_fwd,
                                &new_prefix_fwd,
                            )
                            .await;
                        }
                    }
                    disabled_count += 1;
                    disabled_ids.push(id.clone());
                }
                Err(e) => {
                    log::warn!("Soft failure disabling mod {} ({}): {}", fp, id, e);
                    continue;
                }
            }
        }

        // Update DB status for disabled mods that were successfully renamed
        if !disabled_ids.is_empty() {
            let mut qb: sqlx::QueryBuilder<sqlx::Sqlite> =
                sqlx::QueryBuilder::new("UPDATE mods SET status = 'DISABLED' WHERE id IN (");
            let mut separated = qb.separated(", ");
            for id in &disabled_ids {
                separated.push_bind(id);
            }
            separated.push_unseparated(")");

            qb.build()
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(disabled_count)
    }

    /// Restores mods from a saved collection.
    /// Returns (restored_count, warnings) where warnings list missing mods.
    async fn restore_corridor(
        pool: &SqlitePool,
        watcher_state: &WatcherState,
        collection_id: &str,
        game_id: &str,
    ) -> Result<(usize, Vec<String>), String> {
        // Get mod IDs from the saved collection
        let target_ids =
            collection_repo::get_mod_ids_for_collection_in_game(pool, collection_id, game_id)
                .await
                .map_err(|e| e.to_string())?;

        if target_ids.is_empty() {
            return Ok((0, vec![]));
        }

        // Get current state of those mods to compute diff
        let states = collection_repo::get_mod_states_by_ids(pool, game_id, &target_ids)
            .await
            .map_err(|e| e.to_string())?;

        let mut warnings = Vec::new();

        // Check for orphaned items (mods that no longer exist in DB)
        let found_ids: std::collections::HashSet<_> = states.iter().map(|s| s.id.clone()).collect();
        let orphaned =
            collection_repo::get_collection_items_with_missing_mods(pool, collection_id, game_id)
                .await
                .map_err(|e| e.to_string())?;

        for (old_id, maybe_path) in &orphaned {
            if let Some(path) = maybe_path {
                // Try to re-link by path
                let found = collection_repo::get_mod_id_by_path(pool, path, game_id)
                    .await
                    .map_err(|e| e.to_string())?;

                if found.is_none() {
                    let name = std::path::Path::new(path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| old_id.clone());
                    warnings.push(format!("Missing mod: {}", name));
                }
            } else {
                warnings.push(format!("Missing mod (no path): {}", old_id));
            }
        }

        // Only enable mods that are currently DISABLED — skip any already ENABLED or missing
        let mods_to_enable: Vec<_> = states
            .iter()
            .filter(|s| s.status == "DISABLED" && found_ids.contains(&s.id))
            .collect();

        if mods_to_enable.is_empty() {
            return Ok((0, warnings));
        }

        let game_paths = game_repo::get_all_game_mod_paths(pool)
            .await
            .map_err(|e| e.to_string())?;

        let mods_path = game_paths.get(game_id).cloned().unwrap_or_default();
        let mut restored_count = 0;

        for state in &mods_to_enable {
            let abs_path = if !mods_path.is_empty() {
                std::path::Path::new(&mods_path)
                    .join(&state.folder_path)
                    .to_string_lossy()
                    .to_string()
            } else {
                continue;
            };

            match toggle_mod_inner(watcher_state, abs_path, true).await {
                Ok(new_path) => {
                    if new_path != state.folder_path {
                        let rel_path = std::path::Path::new(&new_path)
                            .strip_prefix(&mods_path)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(new_path.clone());

                        let _ = mod_repo::update_mod_path_by_id(pool, &state.id, &rel_path).await;

                        // Detect top-level folder renames
                        let rel_components: Vec<_> = std::path::Path::new(&state.folder_path)
                            .components()
                            .collect();
                        if rel_components.len() == 1 {
                            let _ = crate::database::object_repo::update_object_folder_path(
                                pool,
                                game_id,
                                &state.folder_path,
                                &rel_path,
                            )
                            .await;

                            let old_prefix = format!("{}\\", state.folder_path);
                            let new_prefix = format!("{}\\", rel_path);
                            let old_prefix_fwd = format!("{}/", state.folder_path);
                            let new_prefix_fwd = format!("{}/", rel_path);

                            let _ = mod_repo::update_child_paths(
                                pool,
                                game_id,
                                &old_prefix,
                                &new_prefix,
                            )
                            .await;
                            let _ = mod_repo::update_child_paths(
                                pool,
                                game_id,
                                &old_prefix_fwd,
                                &new_prefix_fwd,
                            )
                            .await;
                        }
                    }

                    // Update DB status
                    let _ = sqlx::query("UPDATE mods SET status = 'ENABLED' WHERE id = ?")
                        .bind(&state.id)
                        .execute(pool)
                        .await;

                    restored_count += 1;
                }
                Err(e) => {
                    let name = std::path::Path::new(&state.folder_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| state.id.clone());
                    warnings.push(format!("Could not restore mod {}: {}", name, e));
                    continue;
                }
            }
        }

        Ok((restored_count, warnings))
    }
}

#[cfg(test)]
#[path = "tests/privacy_service_tests.rs"]
mod tests;
