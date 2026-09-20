//! Explicit enable/disable of an object root folder (Workspace Switch).

use super::resolve::{heal_object_root_path, resolve_object_root_path, resolve_object_root_paths};
use crate::modules::library::application::mods::core_ops::{
    plan_toggle_rename, rename_toggle_on_disk, ToggleRenamePlan,
};
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::shared::errors::AppError;
use std::path::Path;

pub struct ObjectSwitchOutcome {
    pub object_id: String,
    pub original_path: String,
    pub next_path: String,
}

#[derive(Debug, Clone)]
pub struct PreparedObjectSwitch {
    object_id: String,
    original_path: String,
    current_path: std::path::PathBuf,
    plan: Option<ToggleRenamePlan>,
}

impl PreparedObjectSwitch {
    pub fn object_id(&self) -> &str {
        &self.object_id
    }

    pub fn journal_steps(&self) -> Vec<(u32, std::path::PathBuf, std::path::PathBuf)> {
        self.journal_step(0).into_iter().collect()
    }

    pub fn journal_step(
        &self,
        sequence: u32,
    ) -> Option<(u32, std::path::PathBuf, std::path::PathBuf)> {
        self.plan.as_ref().map(|plan| {
            (
                sequence,
                plan.old_path().to_path_buf(),
                plan.new_path().to_path_buf(),
            )
        })
    }

    pub fn current_path(&self) -> &Path {
        &self.current_path
    }

    pub fn execute(&self, watcher: &WatcherState) -> Result<ObjectSwitchOutcome, AppError> {
        let paths = self
            .plan
            .as_ref()
            .map(|plan| vec![plan.old_path(), plan.new_path()])
            .unwrap_or_else(|| vec![self.current_path.as_path()]);
        let _suppression = watcher.suppressor.suppress_paths(paths);
        if let Some(plan) = &self.plan {
            plan.apply("object folder")?;
        }
        Ok(ObjectSwitchOutcome {
            object_id: self.object_id.clone(),
            original_path: self.original_path.clone(),
            next_path: self
                .plan
                .as_ref()
                .map(|plan| plan.new_path())
                .unwrap_or(&self.current_path)
                .to_string_lossy()
                .into_owned(),
        })
    }

    pub fn rollback(&self, watcher: &WatcherState) -> Result<(), AppError> {
        let Some(plan) = &self.plan else {
            return Ok(());
        };
        let _suppression = watcher
            .suppressor
            .suppress_paths([plan.old_path(), plan.new_path()]);
        plan.rollback("object folder")
    }
}

pub async fn prepare_object_root_switch(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
    enable: bool,
) -> Result<PreparedObjectSwitch, AppError> {
    let (object, mods_path, current_absolute_path) =
        resolve_object_root_path(pool, game_id, object_id).await?;
    prepare_resolved_object_root_switch(
        object_id,
        &object.folder_path,
        &mods_path,
        &current_absolute_path,
        enable,
    )
}

fn prepare_resolved_object_root_switch(
    object_id: &str,
    object_folder_path: &str,
    mods_path: &str,
    current_absolute_path: &str,
    enable: bool,
) -> Result<PreparedObjectSwitch, AppError> {
    let current_path = std::path::PathBuf::from(&current_absolute_path);
    let original_path = Path::new(&mods_path)
        .join(object_folder_path)
        .to_string_lossy()
        .into_owned();
    let plan = plan_toggle_rename(&current_path, enable)?;
    Ok(PreparedObjectSwitch {
        object_id: object_id.to_string(),
        original_path,
        current_path,
        plan,
    })
}

pub async fn prepare_object_root_switches(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_ids: &[String],
    enable: bool,
) -> Result<Vec<PreparedObjectSwitch>, AppError> {
    let resolved = resolve_object_root_paths(pool, game_id, object_ids).await?;
    resolved
        .into_iter()
        .zip(object_ids)
        .map(|((object, mods_path, current_absolute_path), object_id)| {
            prepare_resolved_object_root_switch(
                object_id,
                &object.folder_path,
                &mods_path,
                &current_absolute_path,
                enable,
            )
        })
        .collect()
}

/// Rename one resolved object root without changing projection state.
pub fn toggle_object_root_on_disk(
    current_path: &Path,
    enable: bool,
) -> Result<Option<std::path::PathBuf>, AppError> {
    if !current_path.exists() || !current_path.is_dir() {
        return Err(AppError::RuntimePathNotFound {
            target: current_path.to_string_lossy().to_string(),
        });
    }

    rename_toggle_on_disk(current_path, enable, "object folder")
}

/// Workspace Switch owns explicit object-root enable/disable.
/// Do not route object targets through mod-toggle services. Disk-only: the
/// caller's scoped reconcile settles the DB afterwards.
pub async fn toggle_object_root_service(
    pool: &sqlx::SqlitePool,
    watcher_state: &WatcherState,
    _op_guard: &crate::platform::fs::operation_lock::OpGuard,
    game_id: &str,
    object_id: &str,
    enable: bool,
) -> Result<ObjectSwitchOutcome, AppError> {
    let (object, mods_path, current_absolute_path) =
        resolve_object_root_path(pool, game_id, object_id).await?;
    let current_relative_path = Path::new(&current_absolute_path)
        .strip_prefix(Path::new(&mods_path))
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| current_absolute_path.clone());
    heal_object_root_path(
        pool,
        game_id,
        &object.folder_path,
        &current_relative_path,
        &mods_path,
    )
    .await?;
    // Toggle rename keeps identity, so one path-scoped entry covers both
    // spellings, through the async event tail after return.
    let _guard = watcher_state
        .suppressor
        .suppress_paths([current_absolute_path.as_str()]);
    let original_absolute_path = Path::new(&mods_path)
        .join(&object.folder_path)
        .to_string_lossy()
        .to_string();
    let current_path = Path::new(&current_absolute_path);

    // Disk is the source of truth: the rename is the whole mutation. Object
    // status, folder_path, child paths and the runtime projection converge
    // via the scoped InternalMutation reconcile the caller runs afterwards —
    // the single writer of those columns. Child mod status is NOT cascaded:
    // it derives from each mod's own folder name (see
    // `disk_reconcile::helpers::load_runtime_mod_metadata`), and the UI
    // derives EffectivelyDisabled from the ancestor chain.
    let Some(next_absolute_path) = toggle_object_root_on_disk(current_path, enable)? else {
        // Already in the requested state: the caller's reconcile re-syncs any
        // DB drift; a no-op needs nothing else.
        return Ok(ObjectSwitchOutcome {
            object_id: object_id.to_string(),
            original_path: original_absolute_path,
            next_path: current_absolute_path,
        });
    };

    Ok(ObjectSwitchOutcome {
        object_id: object_id.to_string(),
        original_path: original_absolute_path,
        next_path: next_absolute_path.to_string_lossy().to_string(),
    })
}
