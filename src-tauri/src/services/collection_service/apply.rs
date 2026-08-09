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
    /// Per-game reconcile mutex from `DiskReconcileState::game_lock`, held
    /// around the pipeline's inline reconcile so it cannot interleave with a
    /// queued reconcile for the same game. `None` in tests and the recovery
    /// path (no app handle there; recovery holds the op lock and blanket
    /// suppression, and the inline reconcile stays idempotent regardless).
    pub reconcile_lock: Option<std::sync::Arc<tokio::sync::Mutex<()>>>,
}

pub async fn apply_collection(
    request: ApplyCollectionRequest<'_>,
) -> Result<ApplyResult, CollectionError> {
    // Drive the pipeline with the caller's current corridor (request.is_safe),
    // NOT the collection's own is_safe — validate_corridor then rejects a
    // cross-corridor apply before any filesystem mutation.
    let mut ctx = crate::pipeline::apply_pipeline::ApplyContext::new(request);

    crate::pipeline::apply_pipeline::execute(&mut ctx).await
}
