//! Thumbnail crawler for mod preview images.
//!
//! Searches mod folders for preview images using a priority system:
//! 1. `preview_custom.*` file in root
//! 2. `preview*` file in root
//! 3. Any image file up to depth 3 (fallback when no root `preview*`)
//!
//! # Covers: Epic 2 §B.4

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Valid image extensions for thumbnail detection.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

/// Find the best thumbnail image for a mod folder.
///
/// Priority order:
/// 1. File named `preview_custom.*` in the root folder
/// 2. File named `preview*` in the root folder
/// 3. Any image file within depth 3
///
/// Returns `None` if no suitable image is found.
pub fn find_thumbnail(mod_path: &Path) -> Option<PathBuf> {
    if !mod_path.exists() || !mod_path.is_dir() {
        return None;
    }

    // Priority 1: Check root for "preview.*"
    if let Some(preview) = find_preview_in_root(mod_path) {
        return Some(preview);
    }

    // Priority 2: Any image within depth 3
    find_image_recursive(mod_path, 3)
}

/// List preview images for gallery rendering with deterministic ordering.
///
/// Order:
/// 1. `preview_custom.*` in root
/// 2. other `preview*` in root
/// 3. fallback to all images up to depth 3 (if no root `preview*` exists)
pub fn list_preview_images(mod_path: &Path) -> Vec<PathBuf> {
    if !mod_path.exists() || !mod_path.is_dir() {
        return Vec::new();
    }

    let mut root_files: Vec<PathBuf> = match std::fs::read_dir(mod_path) {
        Ok(read_dir) => read_dir
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| is_image_file(path))
            .collect(),
        Err(_) => Vec::new(),
    };

    root_files.sort();

    let mut custom: Vec<PathBuf> = Vec::new();
    let mut preview: Vec<PathBuf> = Vec::new();

    for file in root_files {
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if stem == "preview_custom" {
            custom.push(file);
        } else if stem.starts_with("preview") {
            preview.push(file);
        }
    }

    if !custom.is_empty() || !preview.is_empty() {
        let mut ordered = Vec::new();
        ordered.extend(custom);
        ordered.extend(preview);
        return ordered;
    }

    let mut fallback: Vec<PathBuf> = WalkDir::new(mod_path)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|path| path.is_file() && is_image_file(path))
        .collect();
    fallback.sort();
    fallback
}

/// Search root directory for files starting with "preview".
fn find_preview_in_root(mod_path: &Path) -> Option<PathBuf> {
    let entries: Vec<PathBuf> = std::fs::read_dir(mod_path)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();

    // Priority 1: exact preview_custom first
    for path in &entries {
        let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_lowercase()) else {
            continue;
        };
        let Some(ext) = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
        else {
            continue;
        };

        if stem == "preview_custom" && IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            return Some(path.clone());
        }
    }

    // Priority 2: other preview* files
    for path in &entries {
        let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_lowercase()) else {
            continue;
        };
        let Some(ext) = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
        else {
            continue;
        };

        if stem.starts_with("preview") && IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            return Some(path.clone());
        }
    }

    None
}

/// Search recursively up to max_depth for any image file.
fn find_image_recursive(mod_path: &Path, max_depth: usize) -> Option<PathBuf> {
    let mut images: Vec<PathBuf> = WalkDir::new(mod_path)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|path| path.is_file() && is_image_file(path))
        .collect();
    images.sort();
    images.into_iter().next()
}

/// Check if a file has an image extension.
fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_thumbnail_preview_priority() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        fs::create_dir(&mod_dir).unwrap();

        // Create a preview image and a regular image
        fs::write(mod_dir.join("preview.png"), "fake png").unwrap();
        fs::write(mod_dir.join("texture.jpg"), "fake jpg").unwrap();

        let result = find_thumbnail(&mod_dir);
        assert!(result.is_some());
        let name = result
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(name.starts_with("preview"));
    }

    #[test]
    fn test_find_thumbnail_any_root_image() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        fs::create_dir(&mod_dir).unwrap();

        // No preview, but has an image
        fs::write(mod_dir.join("screenshot.png"), "fake png").unwrap();
        fs::write(mod_dir.join("config.ini"), "[Section]").unwrap();

        let result = find_thumbnail(&mod_dir);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().file_name().unwrap().to_string_lossy(),
            "screenshot.png"
        );
    }

    #[test]
    fn test_find_thumbnail_nested() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        let sub_dir = mod_dir.join("images");
        fs::create_dir_all(&sub_dir).unwrap();

        // Image only in subfolder
        fs::write(sub_dir.join("thumb.webp"), "fake webp").unwrap();

        let result = find_thumbnail(&mod_dir);
        assert!(result.is_some());
    }

    // Covers: TC-6.2-01 (Discovery priority for custom preview)
    #[test]
    fn test_find_thumbnail_prefers_preview_custom_over_preview() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        fs::create_dir(&mod_dir).unwrap();

        fs::write(mod_dir.join("preview.png"), "fake png").unwrap();
        fs::write(mod_dir.join("preview_custom.png"), "fake custom").unwrap();

        let result = find_thumbnail(&mod_dir).unwrap();
        assert_eq!(
            result.file_name().unwrap().to_string_lossy(),
            "preview_custom.png"
        );
    }

    // Covers: TC-6.2-01 (Depth-3 recursive discovery)
    #[test]
    fn test_find_thumbnail_depth_three_nested_image() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        let deep = mod_dir.join("a").join("b");
        fs::create_dir_all(&deep).unwrap();

        fs::write(deep.join("deep.png"), "fake deep image").unwrap();

        let result = find_thumbnail(&mod_dir);
        assert!(result.is_some(), "Depth-3 image should be discoverable");
        assert_eq!(
            result
                .unwrap()
                .file_name()
                .expect("filename")
                .to_string_lossy(),
            "deep.png"
        );
    }

    #[test]
    fn test_find_thumbnail_no_image() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        fs::create_dir(&mod_dir).unwrap();

        fs::write(mod_dir.join("config.ini"), "[Section]").unwrap();
        fs::write(mod_dir.join("readme.txt"), "text").unwrap();

        let result = find_thumbnail(&mod_dir);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_thumbnail_nonexistent_path() {
        let result = find_thumbnail(Path::new("/nonexistent/path"));
        assert!(result.is_none());
    }

    // Covers: TC-6.2-01 (Ordered gallery list)
    #[test]
    fn test_list_preview_images_ordering() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        let nested = mod_dir.join("images");
        fs::create_dir_all(&nested).unwrap();

        fs::write(mod_dir.join("preview_custom.png"), "custom").unwrap();
        fs::write(mod_dir.join("preview_a.png"), "preview").unwrap();
        fs::write(nested.join("nested.webp"), "nested image").unwrap();

        let ordered = list_preview_images(&mod_dir);
        let names: Vec<String> = ordered
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["preview_custom.png", "preview_a.png"]);
    }

    // Covers: TC-6.2-01 (Fallback deep scan when no root preview)
    #[test]
    fn test_list_preview_images_fallback_deep_scan() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        let nested = mod_dir.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();

        fs::write(mod_dir.join("root.jpg"), "root image").unwrap();
        fs::write(nested.join("deep.webp"), "deep image").unwrap();

        let ordered = list_preview_images(&mod_dir);
        let names: Vec<String> = ordered
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["deep.webp", "root.jpg"]);
    }

    // Covers: Bug #1 — extensionless files (README, LICENSE) must not abort thumbnail search
    #[test]
    fn test_find_thumbnail_skips_extensionless_files() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        fs::create_dir(&mod_dir).unwrap();

        // Create extensionless files that would trip the old `?` operator
        fs::write(mod_dir.join("README"), "readme content").unwrap();
        fs::write(mod_dir.join("LICENSE"), "license content").unwrap();
        fs::write(mod_dir.join(".gitkeep"), "").unwrap();
        // Create a valid preview image
        fs::write(mod_dir.join("preview.png"), "image data").unwrap();

        let result = find_thumbnail(&mod_dir);
        assert!(
            result.is_some(),
            "Should find preview.png despite extensionless files"
        );
        assert_eq!(
            result.unwrap().file_name().unwrap().to_string_lossy(),
            "preview.png"
        );
    }

    // Covers: TC-6.2-01 (Depth-limited recursive gallery)
    #[test]
    fn test_list_preview_images_ignores_depth_over_three() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test_mod");
        let too_deep = mod_dir.join("a").join("b").join("c").join("d");
        fs::create_dir_all(&too_deep).unwrap();

        fs::write(too_deep.join("depth4.png"), "too deep").unwrap();

        let ordered = list_preview_images(&mod_dir);
        assert!(
            ordered
                .iter()
                .all(|p| p.file_name().unwrap().to_string_lossy() != "depth4.png"),
            "Depth > 3 images should not be included"
        );
    }
}
