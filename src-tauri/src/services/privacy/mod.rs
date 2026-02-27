use crate::commands::mods::mod_core_cmds::toggle_mod_inner;
use crate::services::scanner::watcher::WatcherState;
use sqlx::{Row, SqlitePool};

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
                sqlx::query(
                    "UPDATE mods SET last_status_nsfw = (CASE WHEN status = 'ENABLED' THEN 1 ELSE 0 END) WHERE is_safe = 0"
                )
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;

                // 2. Disable All NSFW Mods in Database
                sqlx::query("UPDATE mods SET status = 'DISABLED' WHERE is_safe = 0")
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;

                // 3. Restore SFW State (Re-enable SFW mods previously active)
                sqlx::query(
                    "UPDATE mods SET status = 'ENABLED' WHERE is_safe = 1 AND last_status_sfw = 1",
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

                // 4. Set SFW that was restored back to 0 so we don't accidentally enable it if we switch SFW->SFW.
                sqlx::query(
                    "UPDATE mods SET last_status_sfw = 0 WHERE is_safe = 1 AND last_status_sfw = 1",
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            }
            Mode::NSFW => {
                // SFW to NSFW
                // 1. Snapshot current state of SFW Mods
                sqlx::query(
                    "UPDATE mods SET last_status_sfw = (CASE WHEN status = 'ENABLED' THEN 1 ELSE 0 END) WHERE is_safe = 1"
                )
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;

                // 2. Restore NSFW State
                sqlx::query(
                    "UPDATE mods SET status = 'ENABLED' WHERE is_safe = 0 AND last_status_nsfw = 1",
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

                // 3. Clear the NSFW last_status to prevent double enable issue
                sqlx::query(
                    "UPDATE mods SET last_status_nsfw = 0 WHERE is_safe = 0 AND last_status_nsfw = 1"
                )
                .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            }
        }

        // Commit transaction to disk
        // If crash occurs AFTER this, Startup Recovery or manual Sync will fix the DB vs FS mismatch.
        tx.commit().await.map_err(|e| e.to_string())?;

        // 5. Physical Execution (Batch Rename)
        // Retrieve all mods whose physical state might need update
        let rows = sqlx::query("SELECT id, folder_path, status FROM mods")
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;

        for row in rows {
            let id: String = row.try_get("id").map_err(|e| e.to_string())?;
            let fp: String = row.try_get("folder_path").map_err(|e| e.to_string())?;
            let status: String = row.try_get("status").map_err(|e| e.to_string())?;

            let should_be_enabled = status == "ENABLED";

            // Physically rename using the core module, which also handles watcher suppression
            match toggle_mod_inner(watcher_state, fp.clone(), should_be_enabled).await {
                Ok(new_path) => {
                    // Sync the new path back to DB if it changed
                    if new_path != fp {
                        let _ = sqlx::query("UPDATE mods SET folder_path = ? WHERE id = ?")
                            .bind(&new_path)
                            .bind(&id)
                            .execute(pool)
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
