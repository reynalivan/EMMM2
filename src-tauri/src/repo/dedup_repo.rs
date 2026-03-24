use sqlx::SqlitePool;

pub async fn get_duplicate_whitelist_pairs(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT folder_a_id, folder_b_id FROM duplicate_whitelist WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(pool)
        .await
}

#[derive(Debug, serde::Serialize, specta::Type, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistEntry {
    pub id: String,
    pub folder_a_id: String,
    pub folder_b_id: String,
    pub folder_a_name: String,
    pub folder_b_name: String,
    pub reason: String,
    pub ignored_at: String,
}

pub async fn get_whitelist_detailed(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<WhitelistEntry>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT 
            w.id,
            w.folder_a_id,
            w.folder_b_id,
            m1.actual_name as folder_a_name,
            m2.actual_name as folder_b_name,
            w.reason,
            w.ignored_at
        FROM duplicate_whitelist w
        JOIN mods m1 ON w.folder_a_id = m1.id
        JOIN mods m2 ON w.folder_b_id = m2.id
        WHERE w.game_id = ?
        ORDER BY w.ignored_at DESC
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_whitelist_entry(pool: &SqlitePool, entry_id: &str) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM duplicate_whitelist WHERE id = ?")
        .bind(entry_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn insert_whitelist_pair(
    pool: &SqlitePool,
    game_id: &str,
    canonical_a: &str,
    canonical_b: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO duplicate_whitelist (id, game_id, folder_a_id, folder_b_id, reason)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(game_id)
    .bind(canonical_a)
    .bind(canonical_b)
    .bind("Manual duplicate ignore")
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_group_status(
    pool: &SqlitePool,
    group_id: &str,
    status: &str,
    set_resolved_at: bool,
) -> Result<u64, sqlx::Error> {
    let query = if set_resolved_at {
        "UPDATE dedup_groups SET resolution_status = ?, resolved_at = CURRENT_TIMESTAMP WHERE id = ?"
    } else {
        "UPDATE dedup_groups SET resolution_status = ? WHERE id = ?"
    };

    let result = sqlx::query(query)
        .bind(status)
        .bind(group_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
#[path = "tests/dedup_repo_test.rs"]
mod tests;
