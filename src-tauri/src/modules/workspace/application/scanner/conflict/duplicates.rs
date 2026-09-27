//! Disabled ancestor discovery for mod activation.

use std::path::{Component, Path, PathBuf};

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
