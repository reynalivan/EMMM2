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
