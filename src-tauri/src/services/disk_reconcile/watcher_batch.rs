use std::collections::BTreeSet;
use std::path::Path;

use crate::services::scanner::watcher::ModWatchEvent;

#[derive(Debug, Clone, Default)]
pub struct WatcherRenameHints {
    pub object_renames: Vec<(String, String)>,
    pub mod_renames: Vec<(String, String)>,
}

fn relative_path(path: &str, mods_path: &Path) -> Option<String> {
    let relative = Path::new(path).strip_prefix(mods_path).ok()?;
    Some(relative.to_string_lossy().to_string())
}

fn component_count(path: &str) -> usize {
    Path::new(path).components().count()
}

pub fn collect_changed_paths(events: &[ModWatchEvent]) -> Vec<String> {
    let mut changed_paths = Vec::new();

    for event in events {
        match event {
            ModWatchEvent::Created(path)
            | ModWatchEvent::Removed(path)
            | ModWatchEvent::Modified(path)
            | ModWatchEvent::StatusChanged { path, .. } => {
                changed_paths.push(path.clone());
            }
            ModWatchEvent::Renamed { from, to } => {
                changed_paths.push(from.clone());
                changed_paths.push(to.clone());
            }
            ModWatchEvent::Error(_) => {}
        }
    }

    changed_paths
}

pub fn collect_rename_hints(mods_path: &Path, events: &[ModWatchEvent]) -> WatcherRenameHints {
    let mut object_renames = BTreeSet::new();
    let mut mod_renames = BTreeSet::new();

    for event in events {
        let ModWatchEvent::Renamed { from, to } = event else {
            continue;
        };

        let Some(relative_from) = relative_path(from, mods_path) else {
            continue;
        };
        let Some(relative_to) = relative_path(to, mods_path) else {
            continue;
        };

        let from_depth = component_count(&relative_from);
        let to_depth = component_count(&relative_to);

        if from_depth == 1 && to_depth == 1 {
            object_renames.insert((relative_from, relative_to));
            continue;
        }

        if from_depth == 2 && to_depth == 2 {
            mod_renames.insert((relative_from, relative_to));
        }
    }

    WatcherRenameHints {
        object_renames: object_renames.into_iter().collect(),
        mod_renames: mod_renames.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{collect_changed_paths, collect_rename_hints};
    use crate::services::scanner::watcher::ModWatchEvent;
    use std::path::Path;

    #[test]
    fn collect_changed_paths_keeps_both_rename_sides() {
        let paths = collect_changed_paths(&[
            ModWatchEvent::Created("E:/Mods/Alice/New".to_string()),
            ModWatchEvent::Renamed {
                from: "E:/Mods/Alice/Old".to_string(),
                to: "E:/Mods/Alice/New".to_string(),
            },
        ]);

        assert_eq!(paths.len(), 3);
        assert!(paths.iter().any(|value| value.ends_with("Alice/New")));
        assert!(paths.iter().any(|value| value.ends_with("Alice/Old")));
    }

    #[test]
    fn collect_rename_hints_splits_object_and_mod_renames() {
        let expected_mod_from = std::path::PathBuf::from("Alice")
            .join("Old Mod")
            .to_string_lossy()
            .to_string();
        let expected_mod_to = std::path::PathBuf::from("Alice")
            .join("New Mod")
            .to_string_lossy()
            .to_string();
        let hints = collect_rename_hints(
            Path::new("E:/Mods"),
            &[
                ModWatchEvent::Renamed {
                    from: "E:/Mods/Alice".to_string(),
                    to: "E:/Mods/DISABLED Alice".to_string(),
                },
                ModWatchEvent::Renamed {
                    from: "E:/Mods/Alice/Old Mod".to_string(),
                    to: "E:/Mods/Alice/New Mod".to_string(),
                },
            ],
        );

        assert_eq!(
            hints.object_renames,
            vec![("Alice".to_string(), "DISABLED Alice".to_string())]
        );
        assert_eq!(
            hints.mod_renames,
            vec![(expected_mod_from, expected_mod_to)]
        );
    }
}
