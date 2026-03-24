use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// Result of a consistency check between filesystem and database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyResult {
    /// Number of mods that match between FS and DB
    pub matches: usize,
    /// List of mismatches found
    pub mismatches: Vec<MismatchDetail>,
}

/// Details of a single consistency mismatch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MismatchDetail {
    /// Mod ID from database
    pub mod_id: String,
    /// Folder path of the mod
    pub folder_path: String,
    /// Filesystem state: true if ENABLED (no DISABLED prefix), false if DISABLED
    pub fs_enabled: bool,
    /// Database status field value
    pub db_status: String,
    /// Database disabled_reason field value
    pub db_disabled_reason: Option<String>,
}

impl ConsistencyResult {
    pub fn is_consistent(&self) -> bool {
        self.mismatches.is_empty()
    }
}

/// Verifies that filesystem (DISABLED prefix) and database states are synchronized.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `mod_ids` - Optional list of specific mod IDs to check. If None, spot-checks 10 random recent mods.
///
/// # Returns
/// * `Ok(ConsistencyResult)` - Results of the consistency check
/// * `Err(String)` - Error reading from database or filesystem
pub async fn verify_fs_db_consistency(
    pool: &SqlitePool,
    mod_ids: Option<Vec<&str>>,
) -> Result<ConsistencyResult, String> {
    let mod_ids_to_check: Vec<String> = match mod_ids {
        Some(ids) => {
            if ids.is_empty() {
                return Ok(ConsistencyResult {
                    matches: 0,
                    mismatches: vec![],
                });
            }
            ids.into_iter().map(|s| s.to_string()).collect()
        }
        None => {
            // Spot-check 10 random recent mods
            sqlx::query_scalar::<_, String>("SELECT id FROM mods ORDER BY RANDOM() LIMIT 10")
                .fetch_all(pool)
                .await
                .map_err(|e| format!("Failed to query mods: {e}"))?
        }
    };

    let mut matches = 0;
    let mut mismatches = Vec::new();

    for mod_id in mod_ids_to_check {
        // Query the mod from database
        let mod_info =
            sqlx::query("SELECT id, folder_path, status, disabled_reason FROM mods WHERE id = ?")
                .bind(&mod_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Failed to query mod {}: {e}", mod_id))?;

        let Some(row) = mod_info else {
            // Mod doesn't exist in DB, skip
            continue;
        };

        let mod_id_val: String = row.try_get(0).map_err(|e| e.to_string())?;
        let folder_path: String = row.try_get(1).map_err(|e| e.to_string())?;
        let db_status_val: i64 = row.try_get(2).map_err(|e| e.to_string())?;
        let db_status = if db_status_val == 1 {
            crate::database::models::ItemStatus::Enabled
        } else {
            crate::database::models::ItemStatus::Disabled
        };
        let db_disabled_reason: Option<String> = row.try_get(3).map_err(|e| e.to_string())?;

        // Check if folder path has DISABLED prefix
        let folder_name = std::path::Path::new(&folder_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let fs_enabled = !is_disabled_prefix(folder_name);

        // Determine if DB state matches FS state
        let db_enabled = db_status == crate::database::models::ItemStatus::Enabled;

        if fs_enabled == db_enabled {
            matches += 1;
        } else {
            mismatches.push(MismatchDetail {
                mod_id: mod_id_val,
                folder_path,
                fs_enabled,
                db_status: db_status.as_str().to_string(),
                db_disabled_reason,
            });
        }
    }

    Ok(ConsistencyResult {
        matches,
        mismatches,
    })
}

/// Check if a folder name starts with the DISABLED prefix
fn is_disabled_prefix(folder_name: &str) -> bool {
    folder_name.starts_with("DISABLED ")
        || folder_name.starts_with("disabled ")
        || folder_name.starts_with("Disabled ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{insert_test_game, insert_test_mod, TestGameFixture, TestModFixture};

    async fn setup_pool() -> SqlitePool {
        crate::test_utils::init_test_db().await.pool
    }

    async fn seed_game(pool: &SqlitePool, id: &str, name: &str) {
        insert_test_game(
            pool,
            &TestGameFixture {
                id,
                name,
                game_type: crate::database::models::GameType::GIMI,
                path: "/dummy",
                mods_path: Some("/dummy/Mods"),
            },
        )
        .await
        .unwrap();
    }

    async fn seed_mod(
        pool: &SqlitePool,
        id: &str,
        game_id: &str,
        name: &str,
        path: &str,
        status: crate::database::models::ItemStatus,
        disabled_reason: Option<&str>,
    ) {
        // Create a dummy object to satisfy NOT NULL constraint
        crate::test_utils::insert_test_object(
            pool,
            &crate::test_utils::TestObjectFixture {
                id: "dummy_obj",
                game_id,
                name: "Dummy Object",
                folder_path: Some("Dummy"),
                object_type: "Other",
            },
        )
        .await
        .ok(); // Ignore if already exists

        insert_test_mod(
            pool,
            &TestModFixture {
                id,
                game_id,
                object_id: Some("dummy_obj"),
                actual_name: name,
                folder_path: path,
                status,
                is_safe: true,
                object_type: Some("Other"),
                mods_path: Some("/dummy/Mods"),
            },
        )
        .await
        .unwrap();

        // Update disabled_reason if provided
        if let Some(reason) = disabled_reason {
            sqlx::query("UPDATE mods SET disabled_reason = ? WHERE id = ?")
                .bind(reason)
                .bind(id)
                .execute(pool)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn test_consistency_check_all_enabled() {
        let pool = setup_pool().await;
        seed_game(&pool, "g1", "Genshin").await;

        seed_mod(
            &pool,
            "m1",
            "g1",
            "Mod1",
            "/dummy/Mods/Mod1",
            crate::database::models::ItemStatus::Enabled,
            None,
        )
        .await;
        seed_mod(
            &pool,
            "m2",
            "g1",
            "Mod2",
            "/dummy/Mods/Mod2",
            crate::database::models::ItemStatus::Enabled,
            None,
        )
        .await;

        let result = verify_fs_db_consistency(&pool, Some(vec!["m1", "m2"]))
            .await
            .unwrap();

        assert_eq!(result.matches, 2);
        assert!(result.mismatches.is_empty());
    }

    #[tokio::test]
    async fn test_consistency_check_with_mismatch() {
        let pool = setup_pool().await;
        seed_game(&pool, "g1", "Genshin").await;

        // Create a mod that is ENABLED in DB but has DISABLED prefix in path (MISMATCH)
        seed_mod(
            &pool,
            "m1",
            "g1",
            "Mod1",
            "/dummy/Mods/Mod1",
            crate::database::models::ItemStatus::Enabled,
            None,
        )
        .await;
        seed_mod(
            &pool,
            "m2",
            "g1",
            "Mod2",
            "/dummy/Mods/DISABLED Mod2",
            crate::database::models::ItemStatus::Enabled, // DB says ENABLED
            None,                                         // But FS has DISABLED prefix
        )
        .await;

        let result = verify_fs_db_consistency(&pool, Some(vec!["m1", "m2"]))
            .await
            .unwrap();

        // m2 has mismatch: FS=disabled pero DB=enabled
        assert!(!result.is_consistent());
        assert!(result.mismatches.iter().any(|m| m.mod_id == "m2"));
        assert_eq!(result.matches, 1); // Only m1 matches
    }

    #[tokio::test]
    async fn test_consistency_check_spot_check_random() {
        let pool = setup_pool().await;
        seed_game(&pool, "g1", "Genshin").await;

        // Create multiple mods
        for i in 0..15 {
            let id = format!("m{}", i);
            let path = format!("/dummy/Mods/Mod{}", i);
            let status = if i % 2 == 0 {
                crate::database::models::ItemStatus::Enabled
            } else {
                crate::database::models::ItemStatus::Disabled
            };
            let reason = if status == crate::database::models::ItemStatus::Disabled {
                Some("USER")
            } else {
                None
            };
            seed_mod(
                &pool,
                &id,
                "g1",
                &format!("Mod{}", i),
                &path,
                status,
                reason,
            )
            .await;
        }

        // When mod_ids is None, should spot-check approximately 10 mods
        let result = verify_fs_db_consistency(&pool, None).await.unwrap();

        // Should have checked some mods
        assert!(result.matches + result.mismatches.len() > 0);
        assert!(result.matches + result.mismatches.len() <= 10);
    }

    #[tokio::test]
    async fn test_consistency_check_filesystem_disabled_enabled_in_db() {
        let pool = setup_pool().await;
        seed_game(&pool, "g1", "Genshin").await;

        // Filesystem has DISABLED prefix, but DB shows ENABLED
        // This would be a mismatch: FS says disabled, DB says enabled
        seed_mod(
            &pool,
            "m1",
            "g1",
            "Mod1",
            "/dummy/Mods/DISABLED Mod1",
            crate::database::models::ItemStatus::Enabled,
            None,
        )
        .await;

        let result = verify_fs_db_consistency(&pool, Some(vec!["m1"]))
            .await
            .unwrap();

        assert!(!result.is_consistent());
        let mismatch = &result.mismatches[0];
        assert_eq!(mismatch.mod_id, "m1");
        assert!(!mismatch.fs_enabled); // FS shows disabled
        assert_eq!(mismatch.db_status, "active"); // DB shows enabled
    }

    #[tokio::test]
    async fn test_consistency_check_empty_mod_list() {
        let pool = setup_pool().await;
        seed_game(&pool, "g1", "Genshin").await;

        let result = verify_fs_db_consistency(&pool, Some(vec![])).await.unwrap();

        assert_eq!(result.matches, 0);
        assert!(result.mismatches.is_empty());
    }

    #[tokio::test]
    async fn test_consistency_check_nonexistent_mod() {
        let pool = setup_pool().await;
        seed_game(&pool, "g1", "Genshin").await;

        // Try to check a mod that doesn't exist
        // Should not error, just return no matches
        let result = verify_fs_db_consistency(&pool, Some(vec!["nonexistent"]))
            .await
            .unwrap();

        assert_eq!(result.matches, 0);
        assert!(result.mismatches.is_empty());
    }
}
