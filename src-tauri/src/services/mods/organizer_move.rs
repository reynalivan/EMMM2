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

pub async fn move_mods_to_object_service(
    pool: &sqlx::SqlitePool,
    _op_guard: &crate::services::fs_utils::operation_lock::OpGuard,
    watcher: &WatcherState,
    params: MoveModsToObjectParams<'_>,
) -> Result<crate::services::mods::bulk::BulkResult, AppError> {
    if params.folder_paths.is_empty() {
        return Ok(crate::services::mods::bulk::BulkResult::new(
            Vec::new(),
            Vec::new(),
        ));
    }

    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, params.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let target_obj = crate::repo::object_repo::get_game_object_by_id(pool, params.target_object_id)
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
    let mut collection_impact = crate::domain::collection::CollectionReferenceImpact::default();
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
                collection_impact.merge(result.collection_impact);
                path_rewrites.extend(result.path_rewrites);
            }
            Err(error) => failures.push(crate::services::mods::bulk::BulkActionError {
                path: folder_path.original().to_string(),
                error,
            }),
        }
    }

    Ok(
        crate::services::mods::bulk::BulkResult::with_collection_impact(
            success,
            failures,
            collection_impact,
            path_rewrites,
        ),
    )
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

struct MoveOneResult {
    new_rel: String,
    collection_impact: crate::domain::collection::CollectionReferenceImpact,
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
    let old_rel = current_path
        .strip_prefix(base_path)
        .unwrap_or(&current_path)
        .to_string_lossy()
        .to_string();
    let new_rel = new_path
        .strip_prefix(base_path)
        .unwrap_or(&new_path)
        .to_string_lossy()
        .to_string();

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

    // Identity migration only (doc 1b, path 1): the row follows its folder so
    // tags/collections survive the move. `status` is not written here — it
    // derives from the folder name via the caller's scoped reconcile.
    let mod_id_status =
        crate::repo::mod_repo::get_mod_id_and_status_by_path(pool, &old_rel, game_id).await?;
    if let Some((mod_id, _, _)) = mod_id_status {
        crate::repo::mod_repo::set_mod_object(pool, &mod_id, target_object_id).await?;
    }

    crate::repo::mod_repo::update_mod_path_by_old_path_in_game(pool, game_id, &old_rel, &new_rel)
        .await?;

    let collection_impact = crate::services::collection_service::handle_mod_moved_or_renamed(
        pool,
        &old_rel,
        &new_rel,
        Some(target_object_id),
    )
    .await?;
    let mut path_rewrites = vec![crate::domain::workspace::WorkspacePathRewrite {
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
            &mut path_rewrites,
        )
        .await?;
    }

    Ok(MoveOneResult {
        new_rel,
        collection_impact,
        path_rewrites,
    })
}
