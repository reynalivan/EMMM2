use crate::domain::errors::AppError;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceMoveTarget {
    pub object_id: String,
    pub object_name: String,
    pub object_folder_path: String,
    pub target_subpath: Option<String>,
    pub display_path: String,
    #[specta(type = f64)]
    pub depth: usize,
}

pub async fn list_move_targets_for_object_service(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<Vec<WorkspaceMoveTarget>, AppError> {
    let game_mod_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let target_obj = crate::repo::object_repo::get_game_object_by_id(pool, object_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Target object not found".to_string()))?;

    if target_obj.game_id != game_id {
        return Err(AppError::Validation(format!(
            "Target object '{}' belongs to a different game",
            object_id
        )));
    }

    let object_folder_path = target_obj.folder_path.clone();
    let object_name = target_obj.name.clone();
    let object_root = Path::new(&game_mod_path).join(&object_folder_path);
    let mut targets = vec![WorkspaceMoveTarget {
        object_id: object_id.to_string(),
        object_name: object_name.clone(),
        object_folder_path: object_folder_path.clone(),
        target_subpath: None,
        display_path: object_name.clone(),
        depth: 0,
    }];

    if object_root.is_dir() {
        collect_move_targets(
            &object_root,
            &object_root,
            object_id,
            &object_name,
            &object_folder_path,
            &mut targets,
            0,
        )?;
    }

    Ok(targets)
}

fn collect_move_targets(
    root: &Path,
    current: &Path,
    object_id: &str,
    object_name: &str,
    object_folder_path: &str,
    targets: &mut Vec<WorkspaceMoveTarget>,
    depth: usize,
) -> Result<(), AppError> {
    let entries = std::fs::read_dir(current).map_err(|error| AppError::Io(error.to_string()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || is_hidden_dir(&path) || !is_move_container(&path) {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let target_subpath = normalize_relative_path(relative);
        targets.push(WorkspaceMoveTarget {
            object_id: object_id.to_string(),
            object_name: object_name.to_string(),
            object_folder_path: object_folder_path.to_string(),
            display_path: format!("{object_name}/{target_subpath}"),
            target_subpath: Some(target_subpath),
            depth: depth + 1,
        });
        collect_move_targets(
            root,
            &path,
            object_id,
            object_name,
            object_folder_path,
            targets,
            depth + 1,
        )?;
    }

    Ok(())
}

fn is_hidden_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn is_move_container(path: &Path) -> bool {
    let (node_type, _reasons, _warnings) =
        crate::services::explorer::classifier::classify_folder(path);
    node_type == crate::services::explorer::classifier::NodeType::ContainerFolder
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str().map(ToString::to_string),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
