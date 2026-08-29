use crate::shared::errors::AppError;
use crate::modules::workspace::application::workspace_mutation::import_commit::{rollback_move_journal, MoveJournalEntry};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};

#[derive(Debug, Default)]
pub struct ObjectDisableResult {
    pub disabled_objects: u32,
    pub warning: Option<String>,
}

pub async fn disable_object_roots(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_ids: &[String],
) -> Result<ObjectDisableResult, AppError> {
    if object_ids.is_empty() {
        return Ok(ObjectDisableResult::default());
    }
    let mods_root_raw = crate::modules::games::adapters::outbound::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}'")))?;
    let mods_root = Path::new(&mods_root_raw).canonicalize()?;
    let mut plans = Vec::new();
    let mut seen_ids = BTreeSet::new();
    for object_id in object_ids {
        if !seen_ids.insert(object_id.as_str()) {
            return Err(AppError::Validation(
                "Object IDs to disable must be unique".to_string(),
            ));
        }
        let object = crate::modules::catalog::adapters::outbound::sqlite::object::get_game_object_by_id(pool, object_id)
            .await?
            .filter(|object| object.game_id == game_id)
            .ok_or_else(|| AppError::NotFound(format!("Object '{object_id}'")))?;
        let stored = PathBuf::from(&object.folder_path);
        let raw_path = if stored.is_absolute() {
            stored
        } else {
            mods_root.join(stored)
        };
        let source = crate::modules::library::application::mods::core_ops::resolve_existing_runtime_variant(
            &mods_root, &raw_path, false,
        )
        .unwrap_or(raw_path);
        let source = source.canonicalize().map_err(|error| {
            AppError::Validation(format!("Object folder is unavailable: {error}"))
        })?;
        if !source.starts_with(&mods_root) {
            return Err(AppError::Security(format!(
                "Object folder escapes the configured Mods root: {}",
                source.display()
            )));
        }
        let name = source
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| AppError::Validation("Object folder name is invalid".to_string()))?;
        let target = source.with_file_name(crate::modules::library::application::mods::core_ops::standardize_prefix(
            name, false,
        ));
        if source == target {
            continue;
        }
        if target.exists() {
            return Err(AppError::Validation(format!(
                "Cannot disable '{}': destination already exists",
                object.name
            )));
        }
        plans.push((object_id.clone(), source, target));
    }
    if plans.is_empty() {
        return Ok(ObjectDisableResult::default());
    }

    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight(app, pool, game_id).await?;
    let operation_lock = app
        .try_state::<crate::platform::fs::operation_lock::OperationLock>()
        .ok_or_else(|| AppError::Internal("OperationLock state is unavailable".to_string()))?;
    let disk_state = app
        .try_state::<crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileState>()
        .ok_or_else(|| AppError::Internal("DiskReconcileState is unavailable".to_string()))?;
    let lease = disk_state
        .acquire_mutation_lease(game_id, operation_lock.inner())
        .await?;
    let watcher = app
        .try_state::<crate::modules::workspace::application::scanner::watcher::WatcherState>()
        .ok_or_else(|| AppError::Internal("WatcherState is unavailable".to_string()))?;
    let suppression = watcher.suppressor.suppress_paths(
        plans
            .iter()
            .flat_map(|(_, source, target)| [source.as_path(), target.as_path()]),
    );
    let mut journal = Vec::new();
    for (_, source, target) in &plans {
        if let Err(error) =
            crate::platform::fs::file_utils::rename_cross_drive_fallback(source, target)
        {
            let rollback = rollback_move_journal(&journal);
            drop(suppression);
            let recovery =
                crate::modules::workspace::application::disk_reconcile::emit::run_full_internal_disk_reconcile_under_lease(
                    app, pool, game_id, &lease,
                )
                .await;
            drop(lease);
            return Err(AppError::Io(format!(
                "Could not disable object folder: {error}; rollback: {}; recovery reconcile: {}",
                result_label(rollback),
                result_label(recovery)
            )));
        }
        journal.push(MoveJournalEntry::new(source.clone(), target.clone()));
    }
    drop(suppression);

    let changed_paths = plans
        .iter()
        .flat_map(|(_, source, target)| {
            [
                source.to_string_lossy().into_owned(),
                target.to_string_lossy().into_owned(),
            ]
        })
        .collect::<Vec<_>>();
    let hints = plans
        .iter()
        .map(|(object_id, source, target)| {
            crate::modules::library::application::mods::organizer_move::OrganizerMovePathHint {
                old_path: source.to_string_lossy().into_owned(),
                new_path: target.to_string_lossy().into_owned(),
                target_object_id: object_id.clone(),
            }
        })
        .collect();
    let reconcile =
        crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile_with_path_hints_under_lease(
            app,
            pool,
            game_id,
            changed_paths,
            hints,
            &lease,
        )
        .await;
    drop(lease);
    let warning = match reconcile {
        Ok(result) => {
            let applied = result.status.applied();
            let warning = (!applied)
                .then(|| format!("Disk reconcile requires attention: {:?}", result.status));
            let _ = app.emit("disk_reconcile:result", result);
            warning
        }
        Err(error) => Some(format!(
            "Object folders were disabled, but Disk Reconcile failed: {error}"
        )),
    };
    Ok(ObjectDisableResult {
        disabled_objects: plans.len() as u32,
        warning,
    })
}

fn result_label<T, E: std::fmt::Display>(result: Result<T, E>) -> String {
    match result {
        Ok(_) => "ok".to_string(),
        Err(error) => error.to_string(),
    }
}
