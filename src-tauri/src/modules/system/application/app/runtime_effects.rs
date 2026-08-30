use crate::shared::errors::AppError;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState;
use crate::modules::reconciliation::application::disk_reconcile::types::PendingRuntimeEffects;
use std::future::Future;

async fn retry_once<T, F, Fut>(mut operation: F) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    match operation().await {
        Ok(value) => Ok(value),
        Err(first_error) => {
            log::warn!("Runtime effect failed; retrying once: {first_error}");
            operation().await
        }
    }
}

/// What a mutation needs settled once its own work is done.
///
/// A request struct rather than positional flags: the previous signature ended
/// in `(&[bool], bool, bool)` and every call site hand-built the slice.
#[derive(Clone, Copy)]
pub struct RuntimeSideEffects<'a> {
    pub pool: &'a sqlx::SqlitePool,
    pub config: &'a ConfigService,
    pub game_id: &'a str,
    /// Mark collection/runtime queries as needing refresh after the mutation.
    pub collections_dirty: bool,
    /// Regenerate the in-game overlay artifacts.
    pub overlay_refresh: bool,
}

/// The post-commit outcome of runtime work. A mutation has already committed
/// when this is constructed, so an effect failure is represented as pending
/// work rather than an error that falsely reports the mutation as failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeEffectsSettlement {
    pub pending_runtime_effects: PendingRuntimeEffects,
    pub warning: Option<String>,
    pub effect_attempts: u8,
}

/// Stage runtime effects for a committed mutation and execute them with the
/// same bounded retry used by reconcile. A failure preserves the intent in
/// `DiskReconcileState`; the next reconcile pass retries it even when the
/// disk projection itself is unchanged.
pub(crate) async fn settle_committed_runtime_effects(
    state: &DiskReconcileState,
    request: RuntimeSideEffects<'_>,
) -> RuntimeEffectsSettlement {
    settle_committed_runtime_effects_with(
        state,
        request.game_id,
        PendingRuntimeEffects {
            collections_dirty: request.collections_dirty,
            overlay_refresh: request.overlay_refresh,
        },
        || finalize_runtime_side_effects_once(request),
    )
    .await
}

/// Injection point shared by focused command tests. Production callers use
/// [`settle_committed_runtime_effects`] so the effect implementation remains
/// the normal collection/overlay finalizer.
pub(crate) async fn settle_committed_runtime_effects_with<F, Fut>(
    state: &DiskReconcileState,
    game_id: &str,
    requested: PendingRuntimeEffects,
    mut finalize: F,
) -> RuntimeEffectsSettlement
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<bool, AppError>>,
{
    let staged = state.stage_runtime_effects_for_settlement(game_id, requested);
    let mut effect_attempts = 0;
    match retry_once(|| {
        effect_attempts += 1;
        finalize()
    })
    .await
    {
        Ok(_) => {
            let pending_runtime_effects = state.acknowledge_runtime_effects(game_id, staged);
            RuntimeEffectsSettlement {
                pending_runtime_effects,
                warning: None,
                effect_attempts,
            }
        }
        Err(error) => RuntimeEffectsSettlement {
            pending_runtime_effects: staged.pending,
            warning: Some(format!(
                "Runtime effects remain pending and will be retried: {error}"
            )),
            effect_attempts,
        },
    }
}

async fn finalize_runtime_side_effects_once(
    request: RuntimeSideEffects<'_>,
) -> Result<bool, AppError> {
    let RuntimeSideEffects {
        pool,
        config,
        game_id,
        overlay_refresh,
        ..
    } = request;

    if !overlay_refresh {
        return Ok(false);
    }

    crate::modules::system::application::app::post_apply::trigger_overlay_refresh_for_game(pool, config, game_id)
        .await?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{retry_once, settle_committed_runtime_effects_with};
    use crate::shared::errors::AppError;
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState;
    use crate::modules::reconciliation::application::disk_reconcile::types::PendingRuntimeEffects;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    #[tokio::test]
    async fn runtime_effect_retry_recovers_from_one_transient_failure() {
        let mut attempts = 0;
        let result = retry_once(|| {
            attempts += 1;
            let attempt = attempts;
            async move {
                if attempt == 1 {
                    Err(AppError::Io("transient".to_string()))
                } else {
                    Ok("settled")
                }
            }
        })
        .await;

        assert_eq!(result.expect("second attempt should settle"), "settled");
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn runtime_effect_retry_is_bounded_to_two_attempts() {
        let mut attempts = 0;
        let result: Result<(), AppError> = retry_once(|| {
            attempts += 1;
            async { Err(AppError::Io("still broken".to_string())) }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn older_success_does_not_acknowledge_newer_failed_runtime_effects() {
        let state = Arc::new(DiskReconcileState::new());
        let a_staged = Arc::new(Barrier::new(2));
        let release_a = Arc::new(Barrier::new(2));

        let a_state = Arc::clone(&state);
        let a_staged_in_finalize = Arc::clone(&a_staged);
        let release_a_in_finalize = Arc::clone(&release_a);
        let a = tokio::spawn(async move {
            settle_committed_runtime_effects_with(
                &a_state,
                "game",
                PendingRuntimeEffects {
                    collections_dirty: true,
                    overlay_refresh: false,
                },
                move || {
                    let a_staged = Arc::clone(&a_staged_in_finalize);
                    let release_a = Arc::clone(&release_a_in_finalize);
                    async move {
                        a_staged.wait().await;
                        release_a.wait().await;
                        Ok(false)
                    }
                },
            )
            .await
        });

        a_staged.wait().await;
        let b = settle_committed_runtime_effects_with(
            &state,
            "game",
            PendingRuntimeEffects {
                collections_dirty: false,
                overlay_refresh: true,
            },
            || async { Err(AppError::Io("B remains pending".to_string())) },
        )
        .await;
        release_a.wait().await;
        let a = a.await.expect("A settlement task should finish");

        let merged = PendingRuntimeEffects {
            collections_dirty: true,
            overlay_refresh: true,
        };
        assert_eq!(b.pending_runtime_effects, merged);
        assert_eq!(b.effect_attempts, 2);
        assert_eq!(a.pending_runtime_effects, merged);

        assert_eq!(
            state.stage_runtime_effects("game", PendingRuntimeEffects::default()),
            merged,
            "B's newer intent must remain staged after A succeeds"
        );

        let retry = settle_committed_runtime_effects_with(
            &state,
            "game",
            PendingRuntimeEffects::default(),
            || async { Ok(false) },
        )
        .await;
        assert_eq!(
            retry.pending_runtime_effects,
            PendingRuntimeEffects::default()
        );
        assert_eq!(retry.effect_attempts, 1);
    }
}
