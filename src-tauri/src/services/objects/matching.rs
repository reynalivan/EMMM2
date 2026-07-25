use crate::domain::errors::AppError;

/// The canonical match payload written onto the resolved object.
pub struct ObjectMatchFields<'a> {
    pub entry_key: Option<&'a str>,
    pub alias_name: Option<&'a str>,
    pub confidence: Option<f64>,
    pub reason: Option<&'a str>,
    pub source: Option<&'a str>,
}

/// Resolves the object a match applies to (explicit id, else physical folder
/// lookup) and writes the canonical match onto it.
pub async fn apply_object_match(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: Option<&str>,
    folder_path: Option<&str>,
    matched: ObjectMatchFields<'_>,
) -> Result<(), AppError> {
    let target_object_id = match object_id {
        Some(object_id) => object_id.to_string(),
        None => {
            let folder_path = folder_path.ok_or_else(|| {
                AppError::Validation(
                    "apply_object_match_cmd requires object_id or folder_path".to_string(),
                )
            })?;

            crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, folder_path, game_id)
                .await
                .map_err(|error| AppError::Db(error.to_string()))?
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "No physical object found for folder '{}'",
                        folder_path
                    ))
                })?
        }
    };

    crate::repo::object_repo::apply_canonical_match(
        pool,
        &target_object_id,
        matched.entry_key,
        matched.alias_name,
        matched.confidence,
        matched.reason,
        Some(matched.source.unwrap_or("manual_match")),
    )
    .await
    .map_err(|error| AppError::Db(error.to_string()))?;

    Ok(())
}
