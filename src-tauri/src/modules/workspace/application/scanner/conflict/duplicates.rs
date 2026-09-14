//! Duplicate/conflict resolution against the DB projection.
//!
//! Distinct from `hash_scan`: this half is async service orchestration
//! over the mods table, not filesystem INI parsing.

use std::path::{Component, Path, PathBuf};

use crate::shared::errors::AppError;

/// Finds the outermost disabled directory between `mods_root` and a terminal
/// mod. Renaming that directory is required to make a child mod active.
pub(crate) fn activation_path_for_disabled_ancestor(
    target_path: &Path,
    mods_root: &Path,
) -> PathBuf {
    let Ok(relative_path) = target_path.strip_prefix(mods_root) else {
        return target_path.to_path_buf();
    };

    let mut candidate = mods_root.to_path_buf();
    for component in relative_path.components() {
        match component {
            Component::Normal(name) => {
                candidate.push(name);
                if crate::modules::workspace::domain::normalizer::is_disabled_folder(
                    &name.to_string_lossy(),
                ) {
                    return candidate;
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return target_path.to_path_buf();
            }
        }
    }

    target_path.to_path_buf()
}

/// Every disabled directory between the Mods root and a terminal target.
/// The outer directory must be renamed first, so callers preserve this order
/// when preparing a multi-parent activation transaction.
pub(crate) fn disabled_ancestor_paths(target_path: &Path, mods_root: &Path) -> Vec<PathBuf> {
    let Ok(relative_path) = target_path.strip_prefix(mods_root) else {
        return Vec::new();
    };

    let components = relative_path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_os_string()),
            Component::CurDir => None,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => None,
        })
        .collect::<Vec<_>>();
    if components.len() < 2 {
        return Vec::new();
    }

    let mut candidate = mods_root.to_path_buf();
    let mut disabled = Vec::new();
    for component in components.iter().take(components.len() - 1) {
        candidate.push(component);
        if crate::modules::workspace::domain::normalizer::is_disabled_folder(
            &component.to_string_lossy(),
        ) {
            disabled.push(candidate.clone());
        }
    }
    disabled
}

/// Find all enabled mods in the same object as `folder_path` (i.e. duplicates/conflicts).
pub async fn get_duplicates_for_mod_service(
    pool: &sqlx::SqlitePool,
    folder_path: &str,
    game_id: &str,
) -> Result<Vec<crate::modules::library::domain::mods::DuplicateModInfo>, AppError> {
    let mods_path = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .unwrap_or_default();

    let target = crate::modules::library::adapters::sqlite::mods::get_mod_id_and_status_by_path(
        pool,
        folder_path,
        game_id,
    )
    .await
    .map_err(|error| AppError::Io(format!("DB query failed: {error}")))?;
    let (target_mod_id, object_id) = match target {
        Some((mod_id, Some(object_id), _)) => (mod_id, object_id),
        None => return Ok(vec![]), // No object — no duplicates possible
        Some((_, None, _)) => return Ok(vec![]),
    };

    let duplicates = crate::modules::library::adapters::sqlite::mods::get_enabled_duplicates(
        pool,
        &object_id,
        game_id,
        Some(&target_mod_id),
    )
    .await
    .map_err(|e| AppError::Io(format!("DB duplicate query failed: {e}")))?;

    let mut result = Vec::new();
    let mut relevant_mod_ids: Vec<String> = Vec::new();

    for (mod_id, path, name) in duplicates {
        // Variant Detection (Epic 11 Alignment)
        let mut is_variant = false;
        let mut parent_path = String::new();

        if let (Some(target_parent), Some(dup_parent)) = (
            Path::new(folder_path).parent(),
            Path::new(path.as_stored()).parent(),
        ) {
            if target_parent == dup_parent {
                let (node_type, _, _) =
                    crate::modules::workspace::domain::classifier::classify_folder(
                        &Path::new(&mods_path).join(target_parent),
                    );
                if node_type
                    == crate::modules::workspace::domain::classifier::NodeType::VariantContainer
                {
                    is_variant = true;
                    parent_path = target_parent.to_string_lossy().to_string();
                }
            }
        }

        result.push(crate::modules::library::domain::mods::DuplicateModInfo {
            mod_id: mod_id.clone(),
            object_id: object_id.clone(),
            folder_path: path.into_stored(),
            actual_name: name,
            is_variant,
            parent_path,
        });
        relevant_mod_ids.push(mod_id);
    }

    relevant_mod_ids.push(target_mod_id);

    // Check if this specific combination is ignored
    let ignored = crate::modules::workspace::adapters::sqlite::conflict::is_conflict_ignored(
        pool,
        game_id,
        &object_id,
        &relevant_mod_ids,
    )
    .await
    .unwrap_or(false);

    if ignored {
        return Ok(vec![]);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{activation_path_for_disabled_ancestor, disabled_ancestor_paths};
    use std::path::Path;

    #[test]
    fn uses_the_outermost_disabled_ancestor_for_activation() {
        let mods_root = Path::new("C:/Mods");
        let target = Path::new("C:/Mods/DISABLED Amber/Amber Skin");

        assert_eq!(
            activation_path_for_disabled_ancestor(target, mods_root),
            Path::new("C:/Mods/DISABLED Amber")
        );
    }

    #[test]
    fn keeps_a_terminal_path_without_a_disabled_ancestor() {
        let mods_root = Path::new("C:/Mods");
        let target = Path::new("C:/Mods/Amber/Amber Skin");

        assert_eq!(
            activation_path_for_disabled_ancestor(target, mods_root),
            target
        );
    }

    #[test]
    fn returns_all_nested_disabled_ancestors_in_activation_order() {
        let mods_root = Path::new("E:/Mods");
        let target = mods_root.join("DISABLED Group/DISABLED Alice/Blue");

        assert_eq!(
            disabled_ancestor_paths(&target, mods_root),
            vec![
                mods_root.join("DISABLED Group"),
                mods_root.join("DISABLED Group/DISABLED Alice"),
            ]
        );
    }
}
