use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn top_level_root_from_path(path: &Path, mods_path: &Path) -> Option<String> {
    let relative = path.strip_prefix(mods_path).ok()?;
    let first = relative.components().next()?;
    let value = first.as_os_str().to_string_lossy().trim().to_string();
    if value.is_empty() || value.starts_with('.') {
        return None;
    }
    Some(value)
}

pub fn is_thumbnail_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "webp"
    )
}

pub fn is_runtime_relevant_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    if file_name.eq_ignore_ascii_case("info.json") {
        return true;
    }

    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    ext.eq_ignore_ascii_case("ini")
}

pub fn collect_changed_roots(mods_path: &Path, changed_paths: &[String]) -> Vec<String> {
    let mut roots = BTreeSet::new();

    for changed_path in changed_paths {
        let path = PathBuf::from(changed_path);
        if let Some(root) = top_level_root_from_path(&path, mods_path) {
            roots.insert(root);
        }
    }

    roots.into_iter().collect()
}

pub fn collect_thumbnail_roots(mods_path: &Path, changed_paths: &[String]) -> Vec<String> {
    let mut roots = BTreeSet::new();

    for changed_path in changed_paths {
        let path = PathBuf::from(changed_path);
        if !is_thumbnail_path(&path) {
            continue;
        }

        if let Some(root) = top_level_root_from_path(&path, mods_path) {
            roots.insert(root);
        }
    }

    roots.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{collect_changed_roots, collect_thumbnail_roots, is_runtime_relevant_file};
    use std::path::Path;

    #[test]
    fn info_json_is_runtime_relevant() {
        assert!(is_runtime_relevant_file(Path::new(
            "E:/Mods/Alice/Blue/info.json"
        )));
        assert!(is_runtime_relevant_file(Path::new(
            "E:/Mods/Alice/Blue/mod.ini"
        )));
        assert!(!is_runtime_relevant_file(Path::new(
            "E:/Mods/Alice/Blue/notes.txt"
        )));
    }

    #[test]
    fn changed_roots_collect_top_level_roots() {
        let roots = collect_changed_roots(
            Path::new("E:/Mods"),
            &[
                "E:/Mods/Alice/Blue/mod.ini".to_string(),
                "E:/Mods/Alice/Green".to_string(),
                "E:/Mods/Bob".to_string(),
            ],
        );

        assert_eq!(roots, vec!["Alice".to_string(), "Bob".to_string()]);
    }

    #[test]
    fn thumbnail_roots_only_collect_image_roots() {
        let roots = collect_thumbnail_roots(
            Path::new("E:/Mods"),
            &[
                "E:/Mods/Alice/Blue/thumb.png".to_string(),
                "E:/Mods/Alice/Blue/mod.ini".to_string(),
                "E:/Mods/Bob/Red/poster.webp".to_string(),
            ],
        );

        assert_eq!(roots, vec!["Alice".to_string(), "Bob".to_string()]);
    }
}
