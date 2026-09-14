//! One reconcile pass: resolves the game, runs the disk projection, then
//! applies runtime side-effects and builds the result.

use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::DiskScopedDiscovery;
use crate::modules::reconciliation::application::disk_reconcile::reconcile::{
    reconcile_disk_projection, ReconcileDiskProjectionRequest, ReconcileOutcome,
};
use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcilePhase, DiskReconcileReason, DiskReconcileResult, DiskReconcileStatus,
    DiskReconcileWarning, DiskReconcileWarningKind, PendingRuntimeEffects,
};
use crate::modules::workspace::application::scanner::watcher::ModWatchEvent;
use crate::shared::errors::AppError;

use super::request::DiskReconcileContext;

struct RuntimeEffectsRequest<'a> {
    context: DiskReconcileContext<'a>,
    game_id: &'a str,
    reason: DiskReconcileReason,
    outcome: ReconcileOutcome,
    defer_overlay_sync: bool,
}

fn requested_runtime_effects(
    reason: &DiskReconcileReason,
    status: &DiskReconcileStatus,
    folders_changed: bool,
    objects_changed: bool,
    runtime_file_changed: bool,
    defer_overlay_sync: bool,
) -> PendingRuntimeEffects {
    let applied = status.applied();
    let authority_boundary = matches!(
        reason,
        DiskReconcileReason::StartupBoot | DiskReconcileReason::OnboardingCompleted
    );

    PendingRuntimeEffects {
        collections_dirty: applied
            && (authority_boundary || folders_changed || objects_changed || runtime_file_changed),
        // The first index must publish even for an empty library: the unified
        // status panel is still a valid F7 overlay without any character mod.
        overlay_refresh: !defer_overlay_sync
            && applied
            && (authority_boundary || folders_changed || runtime_file_changed),
    }
}

async fn finalize_runtime_effects(request: RuntimeEffectsRequest<'_>) -> DiskReconcileResult {
    let applied = request.outcome.status.applied();
    let current_effects = requested_runtime_effects(
        &request.reason,
        &request.outcome.status,
        request.outcome.folders_changed,
        request.outcome.objects_changed,
        request.outcome.runtime_file_changed,
        request.defer_overlay_sync,
    );
    let pending_effects = if applied {
        request
            .context
            .state
            .stage_runtime_effects(request.game_id, current_effects)
    } else {
        PendingRuntimeEffects::default()
    };
    let collections_changed = pending_effects.collections_dirty;

    let settlement = if applied {
        Some(
            crate::modules::system::application::app::runtime_effects::settle_committed_runtime_effects(
                request.context.state,
                crate::modules::system::application::app::runtime_effects::RuntimeSideEffects {
                    pool: request.context.pool,
                    config: request.context.config,
                    game_id: request.game_id,
                    collections_dirty: collections_changed,
                    overlay_refresh: pending_effects.overlay_refresh,
                    overlay_cause: match request.reason {
                        DiskReconcileReason::StartupBoot => crate::modules::system::application::app::post_apply::OverlaySyncCause::Startup,
                        DiskReconcileReason::OnboardingCompleted => crate::modules::system::application::app::post_apply::OverlaySyncCause::FirstIndex,
                        DiskReconcileReason::ManualRepair => crate::modules::system::application::app::post_apply::OverlaySyncCause::Recovery,
                        DiskReconcileReason::GameSwitched => crate::modules::system::application::app::post_apply::OverlaySyncCause::GameActivated,
                        DiskReconcileReason::WatcherBatch => crate::modules::system::application::app::post_apply::OverlaySyncCause::EffectiveIniChanged,
                        DiskReconcileReason::InternalMutation => crate::modules::system::application::app::post_apply::OverlaySyncCause::EffectiveModsChanged,
                        DiskReconcileReason::ModsViewEntered | DiskReconcileReason::WindowRefocused | DiskReconcileReason::StorageSizeBackfill => crate::modules::system::application::app::post_apply::OverlaySyncCause::Recovery,
                    },
                },
            )
            .await,
        )
    } else {
        None
    };
    let result_pending_effects = settlement
        .as_ref()
        .map_or(PendingRuntimeEffects::default(), |result| {
            result.pending_runtime_effects
        });
    let warnings = settlement
        .and_then(|result| {
            result.warning.map(|message| DiskReconcileWarning {
                kind: DiskReconcileWarningKind::RuntimeEffectsPending,
                message,
            })
        })
        .into_iter()
        .collect();

    DiskReconcileResult {
        game_id: request.game_id.to_string(),
        reconcile_revision: 0,
        reason: request.reason,
        status: request.outcome.status,
        scan_scope: request.outcome.scan_scope,
        error_message: request.outcome.error_message,
        folder_conflicts: request.outcome.folder_conflicts,
        rename_confirmations: request.outcome.rename_confirmations,
        changed_roots: request.outcome.changed_roots,
        objects_changed: request.outcome.objects_changed,
        folders_changed: request.outcome.folders_changed,
        collections_changed,
        runtime_file_changed: request.outcome.runtime_file_changed,
        thumbnail_roots: request.outcome.thumbnail_roots,
        cleared_selection_paths: request.outcome.cleared_selection_paths,
        path_updates: request.outcome.path_updates,
        collection_reference_impact: request.outcome.collection_reference_impact,
        change_summary: request.outcome.change_summary,
        pending_runtime_effects: result_pending_effects,
        warnings,
    }
}

pub(super) struct RefreshRequest<'a> {
    pub(super) context: DiskReconcileContext<'a>,
    pub(super) game_id: &'a str,
    pub(super) reason: DiskReconcileReason,
    pub(super) changed_paths: Vec<String>,
    pub(super) force_full: bool,
    pub(super) watcher_events: Vec<ModWatchEvent>,
    pub(super) path_hints: Vec<super::request::DiskReconcilePathHint>,
    pub(super) defer_overlay_sync: bool,
    pub(super) precomputed_discovery: Option<DiskScopedDiscovery>,
}

pub(super) async fn run_refresh_once(
    request: RefreshRequest<'_>,
) -> Result<DiskReconcileResult, AppError> {
    let settings = request.context.config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|entry| entry.id == request.game_id)
        .ok_or_else(|| {
            AppError::Internal(format!(
                "Game '{}' not found for disk reconcile",
                request.game_id
            ))
        })?;
    let watcher_events = if request.watcher_events.is_empty() {
        None
    } else {
        Some(request.watcher_events.as_slice())
    };
    crate::modules::library::application::mods::core_ops::recover_folder_conflict_journals(
        &game.mod_path,
        &request.context.watcher_suppressor,
    )?;

    if let Some(progress) = &request.context.progress_reporter {
        progress.emit(DiskReconcilePhase::DiscoveringRoots, 0, None, None);
    }
    let reconcile = match reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: request.context.pool,
        game_id: request.game_id,
        mods_path: &game.mod_path,
        safe_mode_keywords: &settings.safety.keywords,
        reason: &request.reason,
        changed_paths: &request.changed_paths,
        force_full: request.force_full,
        watcher_events,
        path_hints: &request.path_hints,
        progress_reporter: request.context.progress_reporter.clone(),
        precomputed_discovery: request.precomputed_discovery,
    })
    .await
    {
        Ok(reconcile) => reconcile,
        Err(error) => {
            if let Some(progress) = &request.context.progress_reporter {
                progress.emit(DiskReconcilePhase::Failed, 0, None, None);
            }
            return Err(error);
        }
    };

    if let Some(progress) = &request.context.progress_reporter {
        progress.emit(DiskReconcilePhase::Finalizing, 0, None, None);
    }

    let progress_reporter = request.context.progress_reporter.clone();
    let result = finalize_runtime_effects(RuntimeEffectsRequest {
        context: request.context,
        game_id: request.game_id,
        reason: request.reason,
        outcome: reconcile,
        defer_overlay_sync: request.defer_overlay_sync,
    })
    .await;
    if let Some(progress) = &progress_reporter {
        progress.emit(DiskReconcilePhase::Completed, 0, None, None);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_retries_runtime_projection_even_when_disk_rows_are_unchanged() {
        let effects = requested_runtime_effects(
            &DiskReconcileReason::StartupBoot,
            &DiskReconcileStatus::Applied,
            false,
            false,
            false,
            false,
        );

        assert_eq!(
            effects,
            PendingRuntimeEffects {
                collections_dirty: true,
                overlay_refresh: true,
            }
        );
    }

    #[test]
    fn first_index_publishes_the_status_overlay_for_an_empty_library() {
        let effects = requested_runtime_effects(
            &DiskReconcileReason::OnboardingCompleted,
            &DiskReconcileStatus::Applied,
            false,
            false,
            false,
            false,
        );

        assert_eq!(
            effects,
            PendingRuntimeEffects {
                collections_dirty: true,
                overlay_refresh: true,
            }
        );
    }

    #[test]
    fn ordinary_noop_reconcile_does_not_repeat_runtime_effects() {
        let effects = requested_runtime_effects(
            &DiskReconcileReason::ModsViewEntered,
            &DiskReconcileStatus::Applied,
            false,
            false,
            false,
            false,
        );

        assert_eq!(effects, PendingRuntimeEffects::default());
    }

    #[test]
    fn partial_conflict_reconcile_still_refreshes_safe_runtime_effects() {
        let effects = requested_runtime_effects(
            &DiskReconcileReason::WatcherBatch,
            &DiskReconcileStatus::AppliedWithFolderConflicts,
            true,
            false,
            false,
            false,
        );

        assert_eq!(
            effects,
            PendingRuntimeEffects {
                collections_dirty: true,
                overlay_refresh: true,
            }
        );
    }

    #[test]
    fn deferred_root_reconcile_keeps_projection_work_but_not_the_overlay_publish() {
        let effects = requested_runtime_effects(
            &DiskReconcileReason::ManualRepair,
            &DiskReconcileStatus::Applied,
            true,
            true,
            false,
            true,
        );

        assert_eq!(
            effects,
            PendingRuntimeEffects {
                collections_dirty: true,
                overlay_refresh: false,
            }
        );
    }
}
