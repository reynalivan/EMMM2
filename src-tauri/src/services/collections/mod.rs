mod apply;
pub(crate) mod apply_progress;
mod effective_state;
pub mod nested_walker;
mod root_resolution;
mod runtime_snapshot;
mod storage;
pub mod types;

pub mod undo;

pub use apply::apply_collection;
pub use apply::snapshot_current_state;
pub(crate) use effective_state::resolve_current_effective_corridor_state;
pub(crate) use runtime_snapshot::ensure_collection_runtime_materialized;
pub use runtime_snapshot::get_collection_runtime_preview;
pub(crate) use runtime_snapshot::get_corridor_runtime_snapshot as resolve_corridor_runtime_snapshot;
pub(crate) use runtime_snapshot::materialize_game_collections_if_missing;
pub(crate) use runtime_snapshot::persist_collection_runtime_materialization;
pub use storage::{
    create_collection, delete_collection, list_collections, save_snapshot_collection_as_named,
    update_collection,
};
pub use types::{
    ApplyCollectionProgress, ApplyCollectionProgressPhase, ApplyCollectionResult, Collection,
    CollectionDetails, CollectionPreviewMod, CollectionRuntimePreview, CollectionStateKind,
    CorridorRuntimeSnapshot, CreateCollectionInput, RuntimeObjectState, UpdateCollectionInput,
};
pub use undo::undo_collection;

pub async fn auto_disable_auto_tagged_outside_corridor(
    pool: &sqlx::SqlitePool,
    watcher_state: &crate::services::scanner::watcher::WatcherState,
    game_id: &str,
    is_safe: bool,
) -> Result<usize, String> {
    let mods_path = crate::database::game_repo::get_mod_path(pool, game_id)
        .await
        .map_err(|e| format!("Failed to get mods_path: {e}"))?;
    let Some(mods_path) = mods_path else {
        return Ok(0);
    };

    let mismatched = crate::database::mod_repo::get_enabled_auto_tagged_mods_outside_corridor(
        pool, game_id, is_safe,
    )
    .await
    .map_err(|e| e.to_string())?;
    if mismatched.is_empty() {
        return Ok(0);
    }

    let (changed, warnings) = crate::services::mods::bulk_ops::bulk_toggle_mods(
        pool,
        watcher_state,
        &mods_path,
        game_id,
        mismatched,
        false,
        Some(crate::services::corridor_constants::DISABLED_REASON_SYSTEM),
    )
    .await?;

    for warning in warnings {
        log::warn!("Auto-tagged corridor cleanup warning: {warning}");
    }

    Ok(changed)
}
