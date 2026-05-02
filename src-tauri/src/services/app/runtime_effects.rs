use std::collections::BTreeSet;
use std::sync::Arc;

use crate::services::config::ConfigService;
use crate::services::scanner::watcher::WatcherSuppressor;

pub async fn finalize_runtime_side_effects(
    pool: &sqlx::SqlitePool,
    config: &ConfigService,
    watcher_suppressor: Arc<WatcherSuppressor>,
    game_id: &str,
    safe_contexts_affected: &[bool],
    collections_dirty: bool,
    overlay_refresh: bool,
) -> Result<bool, String> {
    if collections_dirty {
        let mut distinct_contexts = BTreeSet::new();
        if safe_contexts_affected.is_empty() {
            distinct_contexts.insert(false);
            distinct_contexts.insert(true);
        } else {
            for is_safe in safe_contexts_affected {
                distinct_contexts.insert(*is_safe);
            }
        }

        for is_safe in distinct_contexts {
            crate::services::collection_service::handle_dirty_state(pool, game_id, is_safe)
                .await
                .map_err(|error| {
                    format!(
                        "Collection dirty-state refresh failed for game '{}' (safe={}): {}",
                        game_id, is_safe, error
                    )
                })?;
        }
    }

    if !overlay_refresh {
        return Ok(false);
    }

    crate::services::app::post_apply::trigger_overlay_refresh_for_game(
        pool,
        config,
        watcher_suppressor,
        game_id,
    )
    .await?;

    Ok(true)
}
