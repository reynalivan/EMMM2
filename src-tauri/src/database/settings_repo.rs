use sqlx::SqlitePool;
use std::collections::HashMap;

// ── KV Settings ─────────────────────────────────────────────

/// Get a single setting value by key.
pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

/// Upsert a single setting (INSERT OR REPLACE).
pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR REPLACE INTO app_settings (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

/// Fetch all settings as a HashMap.
pub async fn get_all_settings(pool: &SqlitePool) -> Result<HashMap<String, String>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM app_settings")
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().collect())
}

// ── Reset ───────────────────────────────────────────────────

/// Delete all user data from every table, restoring the app to fresh-install state.
/// Tables are cleared in FK-safe order within a single transaction.
pub async fn reset_all_data(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Child tables first (FK dependencies)
    sqlx::query("DELETE FROM collection_items")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM dedup_group_members")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM dedup_groups")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM dedup_jobs")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM duplicate_whitelist")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM scan_results")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM mods").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM objects").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM collections")
        .execute(&mut *tx)
        .await?;

    // Root tables
    sqlx::query("DELETE FROM games").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM app_settings")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn vacuum_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("VACUUM").execute(pool).await?;
    Ok(())
}

pub async fn get_all_thumbnail_paths(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    use sqlx::Row;
    let rows =
        sqlx::query("SELECT DISTINCT thumbnail_path FROM objects WHERE thumbnail_path IS NOT NULL")
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|r| r.get("thumbnail_path")).collect())
}

pub async fn remove_orphaned_collection_items(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM collection_items
         WHERE collection_id NOT IN (SELECT id FROM collections)
            OR mod_id NOT IN (SELECT id FROM mods)",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn get_app_meta(pool: &SqlitePool, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM app_meta WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

pub async fn set_app_meta(pool: &SqlitePool, key: &str, value: &str) {
    let _ = sqlx::query("INSERT OR REPLACE INTO app_meta (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await;
}

#[cfg(test)]
#[path = "tests/settings_repo_test.rs"]
mod tests;
