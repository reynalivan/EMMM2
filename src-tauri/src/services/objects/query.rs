use crate::repo::object_repo::{GetObjectsResult, ObjectFilter};

/// Pure DB read — no filesystem access.
/// Returns objects from the DB index with `has_naming_conflict = false` (default).
/// Runtime freshness is maintained by Disk Reconcile before queries run.
pub async fn get_filtered_objects_with_conflict_check(
    pool: &sqlx::SqlitePool,
    filter: &ObjectFilter,
) -> Result<GetObjectsResult, String> {
    let objects = crate::repo::object_repo::get_filtered_objects(pool, filter)
        .await
        .map_err(|e| e.to_string())?;

    // No further processing needed for now.

    Ok(GetObjectsResult {
        objects,
        lost_objects: vec![],
    })
}

pub async fn get_category_counts_service(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    safe_mode: bool,
) -> Result<Vec<crate::repo::object_repo::CategoryCount>, String> {
    crate::repo::object_repo::get_category_counts(pool, game_id, safe_mode)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_object_by_id_service(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<crate::services::scanner::core::types::GameObject>, String> {
    crate::repo::object_repo::get_game_object_by_id(pool, id)
        .await
        .map_err(|e| e.to_string())
}
