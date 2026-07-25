//! Stable mod/object identity: deterministic ID policy plus the one-time
//! startup migration that rewrites legacy IDs and path keys in place.

use sqlx::{Row, SqlitePool};

/// Generate a deterministic mod ID from `game_id` + `relative_path`.
/// Uses BLAKE3 hash (first 32 hex chars) so the same folder always gets the same ID.
/// Per TRD §B.6 — replaces random UUID v4 for mod entries.
pub fn generate_stable_id(game_id: &str, folder_path: &str) -> String {
    let key = crate::common::path_key::folder_path_key(folder_path, None);
    let input = format!("{}:{}", game_id, key);
    let hash = blake3::hash(input.as_bytes());
    hash.to_hex()[..32].to_string()
}

/// One-time startup migration: Stabilize mod IDs and path keys for mods, objects, and collection members.
///
/// This ensures that identities are stable even when folders alternate between enabled/disabled
/// (prefixing with `DISABLED `).
pub async fn migrate_to_stable_ids(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut migrated = 0;

    // 1. Fetch ALL mods to check for ID or Key stability issues
    let mods = sqlx::query("SELECT id, game_id, folder_path, folder_path_key FROM mods")
        .fetch_all(&mut *tx)
        .await?;

    for row in mods {
        let old_id: String = row.get(0);
        let game_id: String = row.get(1);
        let folder_path: String = row.get(2);
        let old_key: String = row.get(3);

        let new_key = crate::common::path_key::folder_path_key(&folder_path, None);
        let new_id = generate_stable_id(&game_id, &folder_path);

        if new_id != old_id || new_key != old_key {
            sqlx::query("UPDATE mods SET id = ?, folder_path_key = ? WHERE id = ?")
                .bind(&new_id)
                .bind(&new_key)
                .bind(&old_id)
                .execute(&mut *tx)
                .await?;
            migrated += 1;
        }
    }

    // 2. Fetch ALL objects to stabilize their folder_path_key
    let objects = sqlx::query("SELECT id, folder_path, folder_path_key FROM objects")
        .fetch_all(&mut *tx)
        .await?;

    for row in objects {
        let obj_id: String = row.get(0);
        let folder_path: String = row.get(1);
        let old_key: String = row.get(2);

        let new_key = crate::common::path_key::folder_path_key(&folder_path, None);
        if new_key != old_key {
            sqlx::query("UPDATE objects SET folder_path_key = ? WHERE id = ?")
                .bind(&new_key)
                .bind(&obj_id)
                .execute(&mut *tx)
                .await?;
            migrated += 1;
        }
    }

    // 3. Fetch ALL collection mods to stabilize their mod_path_key (if using keys)
    // and potentially mod_path itself if it's stored as a key in some contexts.
    // In EMMM v2, collection_mods.mod_path is the relative path from mods root.
    let col_mods = sqlx::query("SELECT collection_id, mod_path FROM collection_mods")
        .fetch_all(&mut *tx)
        .await?;

    for row in col_mods {
        let coll_id: String = row.get(0);
        let old_path: String = row.get(1);

        let new_key = crate::common::path_key::folder_path_key(&old_path, None);
        if new_key != old_path {
            // If mod_path was stored as a key or needs stabilization
            sqlx::query(
                "UPDATE collection_mods SET mod_path = ? WHERE collection_id = ? AND mod_path = ?",
            )
            .bind(&new_key)
            .bind(&coll_id)
            .bind(&old_path)
            .execute(&mut *tx)
            .await?;
            migrated += 1;
        }
    }

    // 4. Fetch ALL collection roots to stabilize their root_path_key
    let roots = sqlx::query("SELECT collection_id, root_path_key FROM collection_roots")
        .fetch_all(&mut *tx)
        .await?;

    for row in roots {
        let coll_id: String = row.get(0);
        let old_key: String = row.get(1);

        let new_key = crate::common::path_key::folder_path_key(&old_key, None);
        if new_key != old_key {
            sqlx::query("UPDATE collection_roots SET root_path_key = ? WHERE collection_id = ? AND root_path_key = ?")
                .bind(&new_key)
                .bind(&coll_id)
                .bind(&old_key)
                .execute(&mut *tx)
                .await
                ?;
            migrated += 1;
        }
    }

    tx.commit().await?;

    if migrated > 0 {
        log::info!("Stabilized {migrated} IDs and path keys for identity persistence");
    }

    Ok(migrated)
}
