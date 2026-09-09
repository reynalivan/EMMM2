use crate::modules::catalog::domain::objects::{GetObjectsResult, ObjectFilter};
use crate::shared::errors::AppError;

/// Objects from the DB index, with cold projection rows filled in from disk.
///
/// Runtime freshness is normally maintained by Disk Reconcile, so the disk
/// walk below only runs for rows the projection has not caught up with — the
/// first grid load after a fresh scan.
pub async fn get_filtered_objects_with_conflict_check(
    pool: &sqlx::SqlitePool,
    filter: &ObjectFilter,
) -> Result<GetObjectsResult, AppError> {
    let page =
        crate::modules::catalog::adapters::sqlite::object::get_filtered_objects(pool, filter)
            .await?;
    let mut objects = page.objects;

    if !page.cold_object_ids.is_empty() {
        patch_cold_counts(pool, filter, &mut objects, &page.cold_object_ids).await?;
    }

    crate::modules::catalog::adapters::sqlite::object::apply_status_filter(
        &mut objects,
        filter.status_filter,
    );

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
    objects: &mut [crate::modules::catalog::domain::objects::ObjectSummary],
    cold_ids: &[String],
) -> Result<(), AppError> {
    let cold: std::collections::HashSet<&str> = cold_ids.iter().map(String::as_str).collect();
    let cold_objects: Vec<_> = objects
        .iter()
        .filter(|object| cold.contains(object.id.as_str()))
        .cloned()
        .collect();

    let mods_path = crate::modules::catalog::adapters::sqlite::object::load_game_mods_path(
        pool,
        &filter.game_id,
    )
    .await?;
    let candidates =
        crate::modules::catalog::adapters::sqlite::object::load_object_count_candidates(
            pool,
            &filter.game_id,
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
        let Some(counts) = counts.get(&object.id) else {
            continue;
        };
        object.mod_count = counts.total;
        object.enabled_count = counts.enabled;
        object.safe_mod_count = counts.safe;
        object.unsafe_mod_count = counts.unsafe_count;
        object.unclassified_mod_count = counts.unclassified;
        object.active_mod_paths = counts.active_paths.clone();
    }

    let _ = crate::modules::workspace::adapters::sqlite::runtime_projection::refresh_projection_for_object_ids(
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
) -> Result<Vec<crate::modules::catalog::domain::objects::CategoryCount>, AppError> {
    Ok(
        crate::modules::catalog::adapters::sqlite::object::get_category_counts(pool, game_id)
            .await?,
    )
}

pub async fn get_object_by_id_service(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<
    Option<crate::modules::workspace::application::scanner::core::types::GameObject>,
    AppError,
> {
    Ok(crate::modules::catalog::adapters::sqlite::object::get_game_object_by_id(pool, id).await?)
}
