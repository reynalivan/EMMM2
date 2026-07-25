//! Entry point that hands an apply request to the apply pipeline.

use crate::domain::collection::ApplyResult;
use crate::domain::errors::CollectionError;
use sqlx::SqlitePool;

pub struct ApplyCollectionRequest<'a> {
    pub pool: &'a SqlitePool,
    pub game_id: &'a str,
    pub collection_id: &'a str,
    pub is_safe: bool,
    pub mods_path: std::path::PathBuf,
    pub suppressor: std::sync::Arc<crate::services::scanner::watcher::WatcherSuppressor>,
    pub ignore_missing: bool,
    pub settings: crate::services::config::AppSettings,
}

pub async fn apply_collection(
    request: ApplyCollectionRequest<'_>,
) -> Result<ApplyResult, CollectionError> {
    // Drive the pipeline with the caller's current corridor (request.is_safe),
    // NOT the collection's own is_safe — validate_corridor then rejects a
    // cross-corridor apply before any filesystem mutation.
    let mut ctx = crate::pipeline::apply_pipeline::ApplyContext::new(
        crate::pipeline::apply_pipeline::ApplyContextInput {
            pool: request.pool.clone(),
            game_id: request.game_id.to_string(),
            collection_id: request.collection_id.to_string(),
            is_safe: request.is_safe,
            mods_path: request.mods_path,
            suppressor: request.suppressor,
            ignore_missing: request.ignore_missing,
            settings: request.settings,
        },
    );

    crate::pipeline::apply_pipeline::execute(&mut ctx).await
}
