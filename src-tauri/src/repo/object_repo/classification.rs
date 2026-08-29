//! Transaction-scoped persistence used by the classification writer.

#[derive(sqlx::FromRow)]
pub struct ClassificationObjectState {
    pub custom_skins: Option<String>,
}

pub async fn get_classification_object_state_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    object_id: &str,
) -> Result<Option<ClassificationObjectState>, sqlx::Error> {
    sqlx::query_as("SELECT custom_skins FROM objects WHERE game_id = ? AND id = ?")
        .bind(game_id)
        .bind(object_id)
        .fetch_optional(conn)
        .await
}

pub struct ClassificationFields<'a> {
    pub category: &'a str,
    pub subcategory: Option<&'a str>,
    pub metadata_json: &'a str,
    pub custom_skins_json: Option<&'a str>,
}

pub async fn apply_classification_fields_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    object_id: &str,
    fields: ClassificationFields<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE objects
         SET object_type = ?,
             sub_category = ?,
             metadata = ?,
             custom_skins = CASE WHEN ? IS NULL THEN custom_skins ELSE ? END
         WHERE game_id = ? AND id = ?",
    )
    .bind(fields.category)
    .bind(fields.subcategory)
    .bind(fields.metadata_json)
    .bind(fields.custom_skins_json)
    .bind(fields.custom_skins_json)
    .bind(game_id)
    .bind(object_id)
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}
