use crate::domain::errors::AppError;
use crate::domain::objects::{GetObjectsResult, ObjectFilter};

/// Objects from the DB index, with cold projection rows filled in from disk.
///
/// Runtime freshness is normally maintained by Disk Reconcile, so the disk
/// walk below only runs for rows the projection has not caught up with — the
/// first grid load after a fresh scan.
pub async fn get_filtered_objects_with_conflict_check(
    pool: &sqlx::SqlitePool,
    filter: &ObjectFilter,
) -> Result<GetObjectsResult, AppError> {
    let page = crate::repo::object_repo::get_filtered_objects(pool, filter).await?;
    let mut objects = page.objects;

    if !page.cold_object_ids.is_empty() {
        patch_cold_counts(pool, filter, &mut objects, &page.cold_object_ids).await?;
    }

    crate::repo::object_repo::apply_status_filter(&mut objects, filter.status_filter);

    Ok(GetObjectsResult {
        objects,
        lost_objects: vec![],
    })
}

/// Resolve counts for objects the projection has not built yet, then ask the
/// projection to catch up so the next read is a pure DB hit.
async fn patch_cold_counts(
    pool: &sqlx::SqlitePool,
    filter: &ObjectFilter,
    objects: &mut [crate::domain::objects::ObjectSummary],
    cold_ids: &[String],
) -> Result<(), AppError> {
    let cold: std::collections::HashSet<&str> = cold_ids.iter().map(String::as_str).collect();
    let cold_objects: Vec<_> = objects
        .iter()
        .filter(|object| cold.contains(object.id.as_str()))
        .cloned()
        .collect();

    let mods_path = crate::repo::object_repo::load_game_mods_path(pool, &filter.game_id).await?;
    let candidates = crate::repo::object_repo::load_object_count_candidates(
        pool,
        &filter.game_id,
        filter.safe_mode,
        &cold_objects,
    )
    .await?;

    // Classifying a terminal reads directories and parses INI headers, so it
    // runs off the async runtime rather than stalling a Tokio worker.
    let counts = tokio::task::spawn_blocking(move || {
        super::terminal::build_terminal_counts(&cold_objects, &candidates, mods_path.as_deref())
    })
    .await?;

    for object in objects.iter_mut() {
        let Some((mod_count, enabled_count, active_paths)) = counts.get(&object.id) else {
            continue;
        };
        object.mod_count = *mod_count;
        object.enabled_count = *enabled_count;
        object.active_mod_paths = active_paths.clone();
    }

    let _ = crate::repo::runtime_projection_repo::refresh_projection_for_object_ids(
        pool,
        &filter.game_id,
        cold_ids,
        false,
    )
    .await;

    Ok(())
}

pub async fn get_category_counts_service(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    corridor: crate::domain::corridor::Corridor,
) -> Result<Vec<crate::domain::objects::CategoryCount>, AppError> {
    Ok(crate::repo::object_repo::get_category_counts(pool, game_id, corridor.is_safe()).await?)
}

pub async fn get_object_by_id_service(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<crate::services::scanner::core::types::GameObject>, AppError> {
    Ok(crate::repo::object_repo::get_game_object_by_id(pool, id).await?)
}
