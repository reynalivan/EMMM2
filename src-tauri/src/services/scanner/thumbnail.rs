//! Thumbnail crawler for mod preview images.
//!
//! Searches mod folders for preview images using a priority system:
//! 1. `preview*` file in root
//! 2. Any image file in root
//! 3. Any image file up to depth 2
//!
//! # Covers: Epic 2 §B.4

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Valid image extensions for thumbnail detection.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

/// Find the best thumbnail image for a mod folder.
///
/// Priority order:
/// 1. File named `preview*` in the root folder
/// 2. Any image file in the root folder
/// 3. Any image file within depth 2 (one subfolder deep)
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

    // Priority 2: Any image file in root
    if let Some(image) = find_any_image_in_root(mod_path) {
        return Some(image);
    }

    // Priority 3: Any image within depth 2
    find_image_recursive(mod_path, 2)
}

/// Search root directory for files starting with "preview".
fn find_preview_in_root(mod_path: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(mod_path).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let stem = path.file_stem()?.to_string_lossy().to_lowercase();
        let ext = path.extension()?.to_str()?.to_lowercase();

        if stem.starts_with("preview") && IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            return Some(path);
        }
    }

    None
}

/// Search root directory for any image file.
fn find_any_image_in_root(mod_path: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(mod_path).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        if is_image_file(&path) {
            return Some(path);
        }
    }

    None
}

/// Search recursively up to max_depth for any image file.
fn find_image_recursive(mod_path: &Path, max_depth: usize) -> Option<PathBuf> {
    for entry in WalkDir::new(mod_path)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path == mod_path || path.is_dir() {
            continue;
        }

        if is_image_file(path) {
            return Some(path.to_path_buf());
        }
    }

    None
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
}
