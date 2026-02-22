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
#[path = "tests/thumbnail_tests.rs"]
mod tests;
