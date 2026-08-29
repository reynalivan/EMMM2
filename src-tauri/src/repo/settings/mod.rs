use sqlx::{Executor, Sqlite, SqlitePool};
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
pub async fn set_setting<'e, E>(executor: E, key: &str, value: &str) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("INSERT OR REPLACE INTO app_settings (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(executor)
        .await?;
    Ok(())
}

/// Remove an optional setting so a cleared value does not reappear after restart.
pub async fn delete_setting<'e, E>(executor: E, key: &str) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("DELETE FROM app_settings WHERE key = ?")
        .bind(key)
        .execute(executor)
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
    reset_all_data_with_revision(pool, None).await
}

/// Reset all data and optionally seed the next settings CAS generation in the
/// same transaction, preventing pre-reset snapshots from becoming valid again.
pub async fn reset_all_data_with_revision(
    pool: &SqlitePool,
    settings_revision: Option<u64>,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Disable foreign keys temporarily for the reset transaction
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *tx)
        .await?;

    // Child tables first (FK dependencies)
    sqlx::query("DELETE FROM collection_mods")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM collection_objects")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM object_runtime_projection")
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
    let has_legacy_scan_results: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'scan_results')",
    )
    .fetch_one(&mut *tx)
    .await?;
    if has_legacy_scan_results {
        sqlx::query("DELETE FROM scan_results")
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM mods").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM objects").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM collection_runtime_state")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM collections")
        .execute(&mut *tx)
        .await?;

    // Root tables
    sqlx::query("DELETE FROM games").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM app_settings")
        .execute(&mut *tx)
        .await?;
    if let Some(revision) = settings_revision {
        sqlx::query("INSERT INTO app_settings (key, value) VALUES ('settings_revision', ?)")
            .bind(revision.to_string())
            .execute(&mut *tx)
            .await?;
    }

    // Restore foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn vacuum_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("VACUUM").execute(pool).await?;
    Ok(())
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
#[path = "../tests/settings_repo_test.rs"]
mod tests;
