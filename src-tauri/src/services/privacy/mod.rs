use crate::commands::mods::mod_core_cmds::toggle_mod_inner;
use crate::services::scanner::watcher::WatcherState;
use sqlx::SqlitePool;

pub struct PrivacyManager;

#[derive(Debug)]
pub enum Mode {
    SFW,
    NSFW,
}

impl PrivacyManager {
    /// Executes the atomic transition between SFW and NSFW modes.
    /// This follows the "Dual Guard" and "Atomic Switch Transaction" specs from Epic 7.
    pub async fn switch_mode(
        target_mode: Mode,
        pool: &SqlitePool,
        watcher_state: &WatcherState,
    ) -> Result<(), String> {
        // 1. Database Transaction Setup
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

        match target_mode {
            Mode::SFW => {
                // Ensure NSFW mode was active previously by checking SFW mode active status implicitly
                // 1. Snapshot current state of NSFW Mods
                crate::database::mod_repo::snapshot_nsfw_mods_status_tx(&mut tx)
                    .await
                    .map_err(|e| e.to_string())?;

                // 2. Disable All NSFW Mods in Database
                crate::database::mod_repo::disable_all_nsfw_mods_tx(&mut tx)
                    .await
                    .map_err(|e| e.to_string())?;

                // 3. Restore SFW State (Re-enable SFW mods previously active)
                crate::database::mod_repo::restore_sfw_mods_status_tx(&mut tx)
                    .await
                    .map_err(|e| e.to_string())?;

                // 4. Set SFW that was restored back to 0 so we don't accidentally enable it if we switch SFW->SFW.
                crate::database::mod_repo::clear_sfw_last_status_tx(&mut tx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Mode::NSFW => {
                // SFW to NSFW
                // 1. Snapshot current state of SFW Mods
                crate::database::mod_repo::snapshot_sfw_mods_status_tx(&mut tx)
                    .await
                    .map_err(|e| e.to_string())?;

                // 2. Restore NSFW State
                crate::database::mod_repo::restore_nsfw_mods_status_tx(&mut tx)
                    .await
                    .map_err(|e| e.to_string())?;

                // 3. Clear the NSFW last_status to prevent double enable issue
                crate::database::mod_repo::clear_nsfw_last_status_tx(&mut tx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }

        // Commit transaction to disk
        // If crash occurs AFTER this, Startup Recovery or manual Sync will fix the DB vs FS mismatch.
        tx.commit().await.map_err(|e| e.to_string())?;

        // 5. Physical Execution (Batch Rename)
        // Need game mods_path to convert absolute→relative after rename
        let game_paths = crate::database::game_repo::get_all_game_mod_paths(pool)
            .await
            .map_err(|e| e.to_string())?;

        // Retrieve all mods whose physical state might need update
        let rows = crate::database::mod_repo::get_all_mods_id_path_status_with_game(pool)
            .await
            .map_err(|e| e.to_string())?;

        for (id, fp, status, game_id) in rows {
            let should_be_enabled = status == "ENABLED";

            // folder_path in DB is relative (e.g. "Alhaitham" or "DISABLED Alhaitham").
            // toggle_mod_inner needs an absolute path to check existence and rename.
            let abs_path = if let Some(base) = game_paths.get(&game_id) {
                std::path::Path::new(base)
                    .join(&fp)
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

            // Physically rename using the core module, which also handles watcher suppression
            match toggle_mod_inner(watcher_state, abs_path, should_be_enabled).await {
                Ok(new_path) => {
                    // Sync the new path back to DB if it changed
                    if new_path != fp {
                        let rel_path = if let Some(base) = game_paths.get(&game_id) {
                            std::path::Path::new(&new_path)
                                .strip_prefix(base)
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or(new_path.clone())
                        } else {
                            new_path.clone()
                        };

                        let _ =
                            crate::database::mod_repo::update_mod_path_by_id(pool, &id, &rel_path)
                                .await;
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Soft failure renaming mod {} ({}): {}",
                        fp,
                        should_be_enabled,
                        e
                    );
                    // Just log and continue as per Epic 7 "Edge Case Handling (Robustness): Folder Missing -> Soft Fail"
                    continue;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/privacy_service_tests.rs"]
mod tests;
