use crate::domain::errors::AppError;
use crate::services::config::ConfigService;

/// Which slice of the runtime projection a mutation invalidated.
pub enum RefreshScope {
    /// Exactly these objects changed. Prefer this: a scoped refresh touches
    /// one row per object instead of rebuilding the whole game.
    Objects(Vec<String>),
    /// The mutation touched more than it can enumerate — bulk path rewrites,
    /// duplicate resolution, anything that moves folders between objects.
    FullGame,
}

/// What a mutation leaves behind for the finalizer to settle.
///
/// `#[must_use]` because the projection is a read-model: a mutation that
/// returns without settling leaves the grid, the counts and the in-game
/// overlay describing a library that no longer exists.
#[must_use = "a mutation's outcome must be passed to finalize_mutation, or the \
              runtime projection and overlay are left stale"]
pub struct MutationOutcome {
    pub scope: RefreshScope,
    /// Recompute the "current runtime" collection summary.
    pub collections_dirty: bool,
    /// Regenerate the in-game overlay artifacts.
    pub overlay_refresh: bool,
}

impl MutationOutcome {
    /// The common case: these objects changed, and both the collection summary
    /// and the overlay need to catch up.
    ///
    /// An empty list degrades to [`Self::full_game`]. A mutation that changed
    /// something but could not name the object has an unknown blast radius,
    /// not an empty one — refreshing nothing would leave the projection stale.
    pub fn objects(ids: impl IntoIterator<Item = String>) -> Self {
        let ids: Vec<String> = ids.into_iter().collect();
        if ids.is_empty() {
            return Self::full_game();
        }
        Self {
            scope: RefreshScope::Objects(ids),
            collections_dirty: true,
            overlay_refresh: true,
        }
    }

    /// For mutations whose blast radius cannot be enumerated.
    pub fn full_game() -> Self {
        Self {
            scope: RefreshScope::FullGame,
            collections_dirty: true,
            overlay_refresh: true,
        }
    }

    /// Nothing changed on disk — only the index needs to catch up.
    ///
    /// A toggle that finds the folder already in the requested state still
    /// re-syncs the projection in case the DB drifted, but must not mark
    /// collections dirty or regenerate the overlay for a no-op.
    pub fn resync_only(ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            scope: RefreshScope::Objects(ids.into_iter().collect()),
            collections_dirty: false,
            overlay_refresh: false,
        }
    }
}

/// Settle a mutation: refresh the runtime projection, then run the side
/// effects that depend on it.
///
/// The single owner of both steps. They used to be spelled out as a pair at
/// five call sites, each discarding both results with `let _ =`, so a failed
/// refresh was invisible and the ordering was re-decided every time. Errors
/// are logged here rather than propagated: the mutation itself already
/// succeeded, and failing it now would report a false negative to the user.
///
/// Returns whether the overlay was refreshed.
pub async fn finalize_mutation(
    pool: &sqlx::SqlitePool,
    config: &ConfigService,
    game_id: &str,
    outcome: MutationOutcome,
) -> bool {
    let refreshed = match &outcome.scope {
        RefreshScope::Objects(ids) => {
            crate::repo::runtime_projection_repo::refresh_projection_for_object_ids(
                pool, game_id, ids, false,
            )
            .await
        }
        RefreshScope::FullGame => {
            crate::repo::runtime_projection_repo::rebuild_game_projection(pool, game_id).await
        }
    };
    if let Err(error) = refreshed {
        log::error!("Runtime projection refresh failed for game '{game_id}': {error}");
    }

    match finalize_runtime_side_effects(RuntimeSideEffects {
        pool,
        config,
        game_id,
        collections_dirty: outcome.collections_dirty,
        overlay_refresh: outcome.overlay_refresh,
    })
    .await
    {
        Ok(overlay_refreshed) => overlay_refreshed,
        Err(error) => {
            log::error!("Runtime side effects failed for game '{game_id}': {error}");
            false
        }
    }
}

/// What a mutation needs settled once its own work is done.
///
/// A request struct rather than positional flags: the previous signature ended
/// in `(&[bool], bool, bool)` and every call site hand-built the slice.
pub struct RuntimeSideEffects<'a> {
    pub pool: &'a sqlx::SqlitePool,
    pub config: &'a ConfigService,
    pub game_id: &'a str,
    /// Recompute the "current runtime" collection summary.
    pub collections_dirty: bool,
    /// Regenerate the in-game overlay artifacts.
    pub overlay_refresh: bool,
}

/// Returns whether the overlay was refreshed.
pub async fn finalize_runtime_side_effects(
    request: RuntimeSideEffects<'_>,
) -> Result<bool, AppError> {
    let RuntimeSideEffects {
        pool,
        config,
        game_id,
        collections_dirty,
        overlay_refresh,
    } = request;

    if collections_dirty {
        // Once, not once per safe-mode corridor: `handle_dirty_state` ignores
        // the corridor argument, so repeating it only repeated the same reads.
        crate::services::collection_service::handle_dirty_state(pool, game_id, false).await?;
    }

    if !overlay_refresh {
        return Ok(false);
    }

    crate::services::app::post_apply::trigger_overlay_refresh_for_game(pool, config, game_id)
        .await?;

    Ok(true)
}
