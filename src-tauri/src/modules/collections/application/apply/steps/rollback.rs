use super::object_toggle_error;
use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::platform::fs::rename::rename_no_replace;
use crate::shared::errors::CollectionError;

pub(super) struct AppliedRename {
    pub(super) sequence: u32,
    pub(super) old_path: std::path::PathBuf,
    pub(super) new_path: std::path::PathBuf,
    pub(super) expected_identity: String,
}

pub(super) fn rollback_applied(
    ctx: &mut ApplyContext,
    planned_count: usize,
    applied: &[AppliedRename],
) -> Result<(), CollectionError> {
    if let Some(guard) = ctx.mutation_guard.as_ref() {
        guard.begin_rollback().map_err(object_toggle_error)?;
    }
    rollback_renamed_paths(applied)?;
    let Some(guard) = ctx.mutation_guard.as_ref() else {
        ctx.mutation_started = false;
        ctx.runtime_path_rewrites.clear();
        return Ok(());
    };
    for sequence in 0..planned_count as u32 {
        guard
            .mark_step_rolled_back(sequence)
            .map_err(object_toggle_error)?;
    }
    if let Some(guard) = ctx.mutation_guard.take() {
        guard.finish_rollback().map_err(object_toggle_error)?;
    }
    ctx.mutation_started = false;
    ctx.runtime_path_rewrites.clear();
    Ok(())
}

fn rollback_renamed_paths(applied: &[AppliedRename]) -> Result<(), CollectionError> {
    for rename in applied.iter().rev() {
        let actual = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&rename.new_path);
        if actual.as_deref() != Some(rename.expected_identity.as_str()) {
            return Err(CollectionError::Io(format!(
                "Rollback folder changed after rename (step {}): {}",
                rename.sequence,
                rename.new_path.display()
            )));
        }
        match std::fs::symlink_metadata(&rename.old_path) {
            Ok(_) => {
                return Err(CollectionError::Io(format!(
                    "Rollback destination is occupied (step {}): {}",
                    rename.sequence,
                    rename.old_path.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(CollectionError::Io(format!(
                    "Could not inspect rollback destination (step {}): {}: {error}",
                    rename.sequence,
                    rename.old_path.display()
                )));
            }
        }
        rename_no_replace(&rename.new_path, &rename.old_path).map_err(|error| {
            CollectionError::Io(format!(
                "Rollback rename failed (step {}): {} to {}: {error}",
                rename.sequence,
                rename.new_path.display(),
                rename.old_path.display()
            ))
        })?;
    }
    Ok(())
}

pub(super) fn with_rollback_error(
    original: CollectionError,
    rollback: Result<(), CollectionError>,
) -> CollectionError {
    match rollback {
        Ok(()) => original,
        Err(error) => CollectionError::Io(format!("{original}; rollback failed: {error}")),
    }
}

pub(super) async fn reconcile_after_mutation_failure(ctx: &mut ApplyContext, warnings: &[String]) {
    ctx.warnings.extend(warnings.iter().cloned());
    let rename_events = ctx
        .runtime_path_rewrites
        .iter()
        .map(|rewrite| {
            crate::modules::workspace::application::scanner::watcher::ModWatchEvent::Renamed {
                from: rewrite.old_path.clone(),
                to: rewrite.new_path.clone(),
            }
        })
        .collect::<Vec<_>>();
    let changed_paths = ctx
        .runtime_path_rewrites
        .iter()
        .flat_map(|rewrite| [rewrite.old_path.clone(), rewrite.new_path.clone()])
        .collect::<Vec<_>>();
    let outcome = crate::modules::reconciliation::application::disk_reconcile::reconcile::reconcile_disk_projection(
        crate::modules::reconciliation::application::disk_reconcile::reconcile::ReconcileDiskProjectionRequest {
            pool: &ctx.pool,
            game_id: &ctx.game_id,
            mods_path: &ctx.mods_path,
            safe_mode_keywords: &ctx.settings.safety.keywords,
            reason: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::InternalMutation,
            changed_paths: &changed_paths,
            force_full: true,
            watcher_events: (!rename_events.is_empty()).then_some(rename_events.as_slice()),
            path_hints: &[],
            trusted_mutation_scope: false,
            progress_reporter: None,
            precomputed_discovery: None,
        },
    )
    .await;
    let recovery_message = match outcome {
        Ok(_) => "Full disk reconcile completed after failed mutation".to_string(),
        Err(error) => format!("Full disk reconcile failed after failed mutation: {error}"),
    };
    ctx.warnings.push(recovery_message);
    crate::modules::library::application::apply_progress::set_warnings(
        &ctx.game_id,
        ctx.warnings.clone(),
    );
}

#[cfg(test)]
mod tests {
    use super::{rollback_renamed_paths, AppliedRename};
    use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity;

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn atomic_rollback_rename_preserves_an_occupied_directory() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let destination_identity = filesystem_identity(&destination).unwrap();

        super::rename_no_replace(&source, &destination)
            .expect_err("destination must not be replaced");

        assert!(source.is_dir());
        assert_eq!(
            filesystem_identity(&destination),
            Some(destination_identity)
        );
    }

    #[test]
    fn rollback_rejects_replacement_renamed_folder() {
        let temp = tempfile::tempdir().unwrap();
        let old_path = temp.path().join("DISABLED Blue");
        let new_path = temp.path().join("Blue");
        let parked = temp.path().join("parked");
        std::fs::create_dir(&old_path).unwrap();
        let expected_identity = filesystem_identity(&old_path).unwrap();
        std::fs::rename(&old_path, &new_path).unwrap();
        std::fs::rename(&new_path, &parked).unwrap();
        std::fs::create_dir(&new_path).unwrap();

        let error = rollback_renamed_paths(&[AppliedRename {
            sequence: 0,
            old_path: old_path.clone(),
            new_path: new_path.clone(),
            expected_identity,
        }])
        .expect_err("replacement must not be moved");
        assert!(error.to_string().contains("changed"));
        assert!(!old_path.exists());
        assert!(new_path.exists());
        assert!(parked.exists());
    }

    #[test]
    fn rollback_rejects_occupied_original_path() {
        let temp = tempfile::tempdir().unwrap();
        let old_path = temp.path().join("DISABLED Blue");
        let new_path = temp.path().join("Blue");
        std::fs::create_dir(&old_path).unwrap();
        let expected_identity = filesystem_identity(&old_path).unwrap();
        std::fs::rename(&old_path, &new_path).unwrap();
        std::fs::create_dir(&old_path).unwrap();

        let error = rollback_renamed_paths(&[AppliedRename {
            sequence: 0,
            old_path: old_path.clone(),
            new_path: new_path.clone(),
            expected_identity,
        }])
        .expect_err("occupied original must not be replaced");
        assert!(error.to_string().contains("occupied"));
        assert!(old_path.exists());
        assert!(new_path.exists());
    }

    #[test]
    fn rollback_restores_matching_folder() {
        let temp = tempfile::tempdir().unwrap();
        let old_path = temp.path().join("DISABLED Blue");
        let new_path = temp.path().join("Blue");
        std::fs::create_dir(&old_path).unwrap();
        let expected_identity = filesystem_identity(&old_path).unwrap();
        std::fs::rename(&old_path, &new_path).unwrap();

        rollback_renamed_paths(&[AppliedRename {
            sequence: 0,
            old_path: old_path.clone(),
            new_path: new_path.clone(),
            expected_identity: expected_identity.clone(),
        }])
        .unwrap();
        assert_eq!(filesystem_identity(&old_path), Some(expected_identity));
        assert!(!new_path.exists());
    }
}
