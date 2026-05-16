use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::PathGuard;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::mods::core_ops::standardize_prefix;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};
use std::path::{Component, Path, PathBuf};

pub struct MoveModsToObjectParams<'a> {
    pub game_id: &'a str,
    pub folder_paths: &'a [String],
    pub target_object_id: &'a str,
    pub target_subpath: Option<&'a str>,
    pub status: Option<&'a str>,
}

pub struct MoveModToObjectParams<'a> {
    pub game_id: &'a str,
    pub folder_path: &'a str,
    pub target_object_id: &'a str,
    pub status: Option<&'a str>,
}

pub async fn move_mod_to_object_service(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    op_lock: &OperationLock,
    watcher: &WatcherState,
    params: MoveModToObjectParams<'_>,
) -> Result<(), AppError> {
    let folder_paths = vec![params.folder_path.to_string()];
    let result = move_mods_to_object_service(
        config,
        pool,
        op_lock,
        watcher,
        MoveModsToObjectParams {
            game_id: params.game_id,
            folder_paths: &folder_paths,
            target_object_id: params.target_object_id,
            target_subpath: None,
            status: params.status,
        },
    )
    .await?;

    if let Some(failure) = result.failures.into_iter().next() {
        return Err(failure.error);
    }

    Ok(())
}

pub async fn move_mods_to_object_service(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    op_lock: &OperationLock,
    watcher: &WatcherState,
    params: MoveModsToObjectParams<'_>,
) -> Result<crate::services::mods::bulk::BulkResult, AppError> {
    let _lock = op_lock.acquire().await.map_err(AppError::Internal)?;
    let _guard = SuppressionGuard::new(&watcher.suppressor);

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
    let mut success = Vec::new();
    let mut failures = Vec::new();
    let mut changed_object_ids = Vec::new();
    let mut collection_impact = crate::domain::collection::CollectionReferenceImpact::default();
    let mut path_rewrites = Vec::new();

    for folder_path in params.folder_paths {
        match move_one_mod_to_object(
            config,
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
                changed_object_ids.extend(result.changed_object_ids);
                collection_impact.merge(result.collection_impact);
                path_rewrites.extend(result.path_rewrites);
            }
            Err(error) => failures.push(crate::services::mods::bulk::BulkActionError {
                path: folder_path.clone(),
                error,
            }),
        }
    }

    changed_object_ids.sort();
    changed_object_ids.dedup();
    crate::services::runtime_projection_service::refresh_projection_for_object_ids(
        pool,
        params.game_id,
        &changed_object_ids,
        false,
    )
    .await?;

    let _ = crate::services::app::runtime_effects::finalize_runtime_side_effects(
        pool,
        config,
        watcher.suppressor.clone(),
        params.game_id,
        &[true, false],
        true,
        true,
    )
    .await;

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
    changed_object_ids: Vec<String>,
    collection_impact: crate::domain::collection::CollectionReferenceImpact,
    path_rewrites: Vec<crate::domain::workspace::WorkspacePathRewrite>,
}

#[allow(clippy::too_many_arguments)] // Internal move receives validated batch context and target paths.
async fn move_one_mod_to_object(
    config: &ConfigService,
    pool: &sqlx::SqlitePool,
    game_id: &str,
    folder_path: &str,
    target_object_id: &str,
    status: Option<&str>,
    base_path: &Path,
    target_obj_path: &Path,
    target_base_path: &Path,
) -> Result<MoveOneResult, AppError> {
    use crate::database::models::ItemStatus;
    use crate::services::scanner::core::normalizer::is_disabled_folder;

    let current_path =
        PathGuard::validate_path(config, game_id, folder_path).map_err(AppError::Security)?;
    let old_object_id =
        crate::repo::mod_repo::get_object_id_by_folder_and_game(pool, folder_path, game_id).await?;
    let mod_folder_name = current_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let is_currently_disabled = is_disabled_folder(&mod_folder_name);
    let mut new_mod_folder_name = mod_folder_name.clone();
    let mut new_status = ItemStatus::from_is_disabled(is_currently_disabled);

    if status == Some("disabled") {
        new_mod_folder_name = standardize_prefix(&mod_folder_name, false);
        new_status = ItemStatus::Disabled;
    }
    if status == Some("only-enable") {
        new_mod_folder_name = standardize_prefix(&mod_folder_name, true);
        new_status = ItemStatus::Enabled;
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

    let mod_id_status =
        crate::repo::mod_repo::get_mod_id_and_status_by_path_any(pool, &old_rel, game_id).await?;
    if let Some((mod_id, _, _)) = mod_id_status {
        crate::repo::mod_repo::set_mod_object(pool, &mod_id, target_object_id).await?;
    }

    crate::repo::mod_repo::update_mod_path_status_and_reason(
        pool,
        game_id,
        &old_rel,
        &new_rel,
        new_status,
        disabled_reason_for_status(new_status),
    )
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
        changed_object_ids: vec![
            old_object_id.unwrap_or_default(),
            target_object_id.to_string(),
        ],
        collection_impact,
        path_rewrites,
    })
}

fn disabled_reason_for_status(status: crate::database::models::ItemStatus) -> Option<&'static str> {
    if status == crate::database::models::ItemStatus::Disabled {
        return Some("User Disabled");
    }
    None
}
