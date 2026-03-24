use std::path::{Path, PathBuf};

pub(crate) fn canonical_collection_path_key(
    folder_path: &str,
    mods_path: Option<&str>,
) -> Option<String> {
    let resolved = resolve_collection_path(folder_path, mods_path)?;
    Some(canonical_path_key_for_path(&resolved))
}

pub(crate) fn canonical_path_key_for_path(path: &Path) -> String {
    normalize_path(path)
        .components()
        .map(|component| canonical_name_key(&component.as_os_str().to_string_lossy()))
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn canonical_name_key(value: &str) -> String {
    let normalized = crate::services::scanner::core::normalizer::normalize_display_name(value);
    normalized
        .chars()
        .map(|ch| {
            if ch.is_ascii_uppercase() {
                ch.to_ascii_lowercase()
            } else {
                ch
            }
        })
        .collect()
}

pub(crate) fn names_equal_by_key(left: &str, right: &str) -> bool {
    canonical_name_key(left) == canonical_name_key(right)
}

pub fn folder_path_key(folder_path: &str, mods_path: Option<&str>) -> String {
    canonical_collection_path_key(folder_path, mods_path)
        .unwrap_or_else(|| canonical_path_key_for_path(Path::new(folder_path)))
}

pub(crate) fn object_name_key(name: &str) -> String {
    canonical_name_key(name.trim())
}

pub(crate) fn collection_name_key(name: &str) -> String {
    canonical_name_key(name.trim())
}

pub(crate) fn resolve_collection_path(
    folder_path: &str,
    mods_path: Option<&str>,
) -> Option<PathBuf> {
    let path = Path::new(folder_path);
    if path.is_absolute() {
        return Some(normalize_path(path));
    }

    let mods_root = mods_path.map(Path::new)?;
    Some(normalize_path(&mods_root.join(path)))
}

pub(crate) fn path_file_name_lossy(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
}

pub(crate) fn path_starts_with_key(path: &str, prefix: &str, mods_path: Option<&str>) -> bool {
    let path_components = normalized_components(path, mods_path);
    let prefix_components = normalized_components(prefix, mods_path);
    if prefix_components.is_empty() || prefix_components.len() > path_components.len() {
        return false;
    }

    path_components.iter().zip(prefix_components.iter()).all(
        |(path_component, prefix_component)| {
            canonical_name_key(path_component) == canonical_name_key(prefix_component)
        },
    )
}

pub(crate) fn strip_path_prefix_preserve_display(
    path: &str,
    prefix: &str,
    mods_path: Option<&str>,
) -> Option<String> {
    let path_components = normalized_components(path, mods_path);
    let prefix_components = normalized_components(prefix, mods_path);
    if prefix_components.is_empty() || prefix_components.len() > path_components.len() {
        return None;
    }

    let matches_prefix = path_components.iter().zip(prefix_components.iter()).all(
        |(path_component, prefix_component)| {
            canonical_name_key(path_component) == canonical_name_key(prefix_component)
        },
    );
    if !matches_prefix {
        return None;
    }

    Some(path_components[prefix_components.len()..].join("/"))
}

fn normalize_path(path: &Path) -> PathBuf {
    path.components().collect()
}

fn normalized_components(path: &str, mods_path: Option<&str>) -> Vec<String> {
    let resolved =
        resolve_collection_path(path, mods_path).unwrap_or_else(|| normalize_path(Path::new(path)));

    resolved
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_collection_path_key, canonical_name_key, canonical_path_key_for_path,
        names_equal_by_key, path_starts_with_key, strip_path_prefix_preserve_display,
    };

    #[test]
    fn canonical_collection_path_key_matches_relative_and_absolute_unicode_paths() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let unicode_relative = std::path::Path::new("캐릭터").join("日本語MOD");
        let unicode_absolute = mods_root.join(&unicode_relative);
        std::fs::create_dir_all(&unicode_absolute).expect("create unicode path");

        let mods_root_str = mods_root.to_string_lossy().to_string();
        let relative_key = canonical_collection_path_key(
            &unicode_relative.to_string_lossy(),
            Some(&mods_root_str),
        )
        .expect("relative key");
        let absolute_key = canonical_collection_path_key(
            &unicode_absolute.to_string_lossy(),
            Some(&mods_root_str),
        )
        .expect("absolute key");

        assert_eq!(relative_key, absolute_key);
        assert!(relative_key.contains("캐릭터"));
        assert!(relative_key.contains("日本語"));
        assert!(relative_key.contains("mod"));
    }

    #[test]
    fn canonical_path_key_only_folds_ascii_case() {
        let path = std::path::Path::new("C:\\Mods\\CHARACTER\\한글모드");
        let key = canonical_path_key_for_path(path);

        assert!(key.contains("character"));
        assert!(key.contains("한글모드"));
        assert!(!key.contains("CHARACTER"));
    }

    #[test]
    fn canonical_name_key_preserves_unicode_characters() {
        let key = canonical_name_key("한글_日本語_中文_MOD");
        assert!(key.contains("한글_日本語_中文"));
        assert!(key.ends_with("_mod"));
    }

    #[test]
    fn names_equal_by_key_only_folds_ascii_case() {
        assert!(names_equal_by_key("Preset_日本語", "preset_日本語"));
        assert!(names_equal_by_key("한글MOD", "한글mod"));
        assert!(!names_equal_by_key("한글모드", "일본어모드"));
    }

    #[test]
    fn path_prefix_compare_only_folds_ascii_case() {
        assert!(path_starts_with_key(
            "한국Character/日本語모드/VariantA",
            "한국character/日本語모드",
            None,
        ));
        assert!(!path_starts_with_key(
            "한국Character/日本語모드/VariantA",
            "다른Character/日本語모드",
            None,
        ));
    }

    #[test]
    fn strip_path_prefix_preserves_unicode_display_segments() {
        let suffix = strip_path_prefix_preserve_display(
            "한국Character/日本語모드/VariantA",
            "한국character/日本語모드",
            None,
        )
        .expect("unicode suffix");

        assert_eq!(suffix, "VariantA");
    }
}
