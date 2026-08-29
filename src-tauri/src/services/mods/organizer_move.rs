use crate::domain::errors::AppError;
use crate::services::fs_utils::guard::ValidatedPath;
use crate::services::mods::core_ops::standardize_prefix;
use crate::services::scanner::watcher::WatcherState;
use std::path::{Component, Path, PathBuf};

pub struct MoveModsToObjectParams<'a> {
    pub game_id: &'a str,
    pub folder_paths: &'a [ValidatedPath],
    pub target_object_id: &'a str,
    pub target_subpath: Option<&'a str>,
    pub status: Option<&'a str>,
}

/// Exact filesystem transitions that terminal reconcile must apply in its
/// projection transaction. The organizer never writes these paths to SQLite.
#[derive(Debug, Clone)]
pub struct OrganizerMovePathHint {
    pub old_path: String,
    pub new_path: String,
    pub target_object_id: String,
}

pub struct OrganizerMoveOutcome {
    pub result: crate::services::mods::bulk::BulkResult,
    pub path_hints: Vec<OrganizerMovePathHint>,
}

pub async fn move_mods_to_object_service(
    pool: &sqlx::SqlitePool,
    _op_guard: &crate::services::fs_utils::operation_lock::OpGuard,
    watcher: &WatcherState,
    params: MoveModsToObjectParams<'_>,
) -> Result<OrganizerMoveOutcome, AppError> {
    if params.folder_paths.is_empty() {
        return Ok(OrganizerMoveOutcome {
            result: crate::services::mods::bulk::BulkResult::new(Vec::new(), Vec::new()),
            path_hints: Vec::new(),
        });
    }

    let game_mod_path = crate::repo::game::get_mod_path(pool, params.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let target_obj = crate::repo::object::get_game_object_by_id(pool, params.target_object_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Target object not found".to_string()))?;

    if target_obj.game_id != params.game_id {
        return Err(AppError::Validation(format!(
            "Target object '{}' belongs to game '{}', but requested move is for game '{}'",
            params.target_object_id, target_obj.game_id, params.game_id
        )));
    }

    let base_path = Path::new(&game_mod_path);
    let target_obj_path = base_path.join(&target_obj.folder_path);
    let target_base_path = resolve_target_base_path(&target_obj_path, params.target_subpath)?;

    // Sources move under the target root: register each source plus the
    // target base so the paired From/To events are both covered.
    let _guard = watcher.suppressor.suppress_paths(
        params
            .folder_paths
            .iter()
            .map(|path| path.as_ref().to_path_buf())
            .chain(std::iter::once(target_base_path.clone())),
    );
    let mut success = Vec::new();
    let mut failures = Vec::new();
    let mut path_hints = Vec::new();
    let mut path_rewrites = Vec::new();

    for folder_path in params.folder_paths {
        match move_one_mod_to_object(
            pool,
            params.game_id,
            folder_path,
            params.target_object_id,
            params.status,
            base_path,
            &target_obj_path,
            &target_base_path,
        )
        .await
        {
            Ok(result) => {
                success.push(result.new_rel.clone());
                path_hints.extend(result.path_hints);
                path_rewrites.extend(result.path_rewrites);
            }
            Err(error) => failures.push(crate::services::mods::bulk::BulkActionError {
                path: folder_path.original().to_string(),
                error,
            }),
        }
    }

    Ok(OrganizerMoveOutcome {
        result: crate::services::mods::bulk::BulkResult::with_collection_impact(
            success,
            failures,
            crate::domain::collection::CollectionReferenceImpact::default(),
            path_rewrites,
        ),
        path_hints,
    })
}

fn resolve_target_base_path(
    target_obj_path: &Path,
    target_subpath: Option<&str>,
) -> Result<PathBuf, AppError> {
    let Some(relative_subpath) = parse_target_subpath(target_subpath)? else {
        if !target_obj_path.exists() {
            std::fs::create_dir_all(target_obj_path)
                .map_err(|error| AppError::Io(error.to_string()))?;
        }
        return Ok(target_obj_path.to_path_buf());
    };

    let target = target_obj_path.join(relative_subpath);
    if !target.is_dir() {
        return Err(AppError::NotFound(format!(
            "Target subfolder does not exist: {}",
            target.to_string_lossy()
        )));
    }

    Ok(target)
}

fn parse_target_subpath(target_subpath: Option<&str>) -> Result<Option<PathBuf>, AppError> {
    let Some(raw_subpath) = target_subpath else {
        return Ok(None);
    };
    let trimmed = raw_subpath.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let mut relative = PathBuf::new();
    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(value) => relative.push(value),
            _ => {
                return Err(AppError::Security(format!(
                    "Invalid target subfolder: {trimmed}"
                )))
            }
        }
    }

    Ok(Some(relative))
}

fn relative_mod_path(path: &Path, mods_root: &Path) -> String {
    let canonical_root = mods_root
        .canonicalize()
        .unwrap_or_else(|_| mods_root.to_path_buf());
    path.strip_prefix(&canonical_root)
        .or_else(|_| path.strip_prefix(mods_root))
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

struct MoveOneResult {
    new_rel: String,
    path_hints: Vec<OrganizerMovePathHint>,
    path_rewrites: Vec<crate::domain::workspace::WorkspacePathRewrite>,
}

#[allow(clippy::too_many_arguments)] // Internal move receives validated batch context and target paths.
async fn move_one_mod_to_object(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    folder: &ValidatedPath,
    target_object_id: &str,
    status: Option<&str>,
    base_path: &Path,
    target_obj_path: &Path,
    target_base_path: &Path,
) -> Result<MoveOneResult, AppError> {
    let current_path = folder.to_path_buf();
    let mod_folder_name = current_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let mut new_mod_folder_name = mod_folder_name.clone();
    if status == Some("disabled") {
        new_mod_folder_name = standardize_prefix(&mod_folder_name, false);
    }
    if status == Some("only-enable") {
        new_mod_folder_name = standardize_prefix(&mod_folder_name, true);
    }

    let new_path = target_base_path.join(&new_mod_folder_name);
    let old_rel = relative_mod_path(&current_path, base_path);
    let new_rel = relative_mod_path(&new_path, base_path);

    if current_path != new_path {
        if new_path.exists() {
            return Err(AppError::Validation(format!(
                "Destination already exists: {}",
                new_path.to_string_lossy()
            )));
        }
        std::fs::rename(&current_path, &new_path)
            .map_err(|error| AppError::Io(error.to_string()))?;
    }

    let mut path_hints = vec![OrganizerMovePathHint {
        old_path: old_rel.clone(),
        new_path: new_rel.clone(),
        target_object_id: target_object_id.to_string(),
    }];
    let path_rewrites = vec![crate::domain::workspace::WorkspacePathRewrite {
        old_path: old_rel.clone(),
        new_path: new_rel.clone(),
    }];

    if status == Some("only-enable") {
        crate::services::mods::organizer_duplicates::disable_target_duplicates(
            pool,
            game_id,
            target_object_id,
            &new_rel,
            base_path,
            target_obj_path,
            &mut path_hints,
        )
        .await?;
    }

    Ok(MoveOneResult {
        new_rel,
        path_hints,
        path_rewrites,
    })
}

#[cfg(test)]
#[path = "tests/organizer_move_tests.rs"]
mod tests;
