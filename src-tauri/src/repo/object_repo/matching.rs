use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn get_matched_entry_key_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT matched_entry_key FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_matched_entry_keys_by_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<HashMap<String, String>, sqlx::Error> {
    let pairs = sqlx::query_as::<_, (String, String)>(
        "SELECT id, matched_entry_key FROM objects
         WHERE game_id = ? AND matched_entry_key IS NOT NULL",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    Ok(pairs.into_iter().collect())
}

/// Every object's `(matched_entry_key, custom_skins)` pair that carries both.
///
/// Feeds the MasterDB loader, which folds the user's own aliases into the
/// bundled entries so the matcher can see them. Returns the raw JSON: parsing
/// is the caller's business, and a blob that fails `json_valid` is dropped here
/// the same way the listing query drops it.
pub async fn get_user_alias_blobs(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT matched_entry_key, custom_skins FROM objects
         WHERE matched_entry_key IS NOT NULL
           AND custom_skins IS NOT NULL
           AND json_valid(custom_skins) = 1",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok((
                row.try_get("matched_entry_key")?,
                row.try_get("custom_skins")?,
            ))
        })
        .collect()
}

pub async fn get_object_folder_by_matched_entry_key<'c, E>(
    executor: E,
    game_id: &str,
    matched_entry_key: &str,
) -> Result<Option<String>, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query_scalar(
        "SELECT folder_path FROM objects
         WHERE game_id = ? AND matched_entry_key = ?
         ORDER BY updated_at DESC
         LIMIT 1",
    )
    .bind(game_id)
    .bind(matched_entry_key)
    .fetch_optional(executor)
    .await
}

pub async fn get_object_id_by_matched_entry_key<'c, E>(
    executor: E,
    game_id: &str,
    matched_entry_key: &str,
) -> Result<Option<String>, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query_scalar(
        "SELECT id FROM objects
         WHERE game_id = ? AND matched_entry_key = ?
         ORDER BY updated_at DESC
         LIMIT 1",
    )
    .bind(game_id)
    .bind(matched_entry_key)
    .fetch_optional(executor)
    .await
}

#[allow(clippy::too_many_arguments)] // Canonical match patch mirrors nullable DB columns at the repo boundary.
pub async fn apply_canonical_match<'c, E>(
    executor: E,
    object_id: &str,
    matched_entry_key: Option<&str>,
    matched_alias_name: Option<&str>,
    matched_confidence: Option<f64>,
    matched_reason: Option<&str>,
    matched_source: Option<&str>,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET matched_entry_key = ?,
             matched_alias_name = ?,
             matched_confidence = ?,
             matched_reason = ?,
             matched_source = ?,
             matched_at = CASE WHEN ? IS NULL THEN matched_at ELSE CURRENT_TIMESTAMP END
         WHERE id = ?",
    )
    .bind(matched_entry_key)
    .bind(matched_alias_name)
    .bind(matched_confidence)
    .bind(matched_reason)
    .bind(matched_source)
    .bind(matched_entry_key)
    .bind(object_id)
    .execute(executor)
    .await?;
    Ok(())
}

/// Fetch all objects that could be relevant for KeyViewer matching (primarily Characters).
/// Returns a list of (Name, HashDb, CustomSkins).
/// Characters and their hash sets, for KeyViewer matching.
///
/// Deliberately does not select `custom_skins`: the only caller discarded it,
/// and it is a JSON blob decoded per row.
pub async fn get_kv_matching_objects(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, crate::domain::models::HashDbPayload)>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT name, hash_db FROM objects WHERE game_id = ? AND object_type = 'Character'",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        let name: String = row.try_get("name")?;
        let hash_db: crate::domain::models::HashDbPayload = row.try_get("hash_db")?;
        result.push((name, hash_db));
    }
    Ok(result)
}
