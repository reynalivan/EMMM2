use super::RuntimeRenamePlan;
use crate::shared::errors::AppError;
use std::collections::HashSet;
use std::path::{Component, Path};
pub(super) fn validate_plans(plans: &[RuntimeRenamePlan]) -> Result<(), AppError> {
    let mut old_paths = HashSet::new();
    let mut new_paths = HashSet::new();

    for plan in plans {
        if !old_paths.insert(normalize_for_collision(plan.old_path())) {
            return Err(AppError::Internal(format!(
                "Duplicate mutation source path detected: {}",
                plan.old_path().display()
            )));
        }

        if !new_paths.insert(normalize_for_collision(plan.new_path())) {
            return Err(AppError::Internal(format!(
                "Duplicate mutation target path detected: {}",
                plan.new_path().display()
            )));
        }
    }

    Ok(())
}

fn normalize_for_collision(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

pub(super) fn validate_relative_path(path: &str) -> Result<(), AppError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() {
        return Err(AppError::Internal("Mod folder path is empty".to_string()));
    }

    if path.is_absolute() {
        return Err(AppError::Internal(format!(
            "Absolute mod folder path is not allowed: {}",
            path.display()
        )));
    }

    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(AppError::Internal(format!(
                    "Unsafe mod folder path is not allowed: {}",
                    path.display()
                )));
            }
        }
    }

    Ok(())
}
