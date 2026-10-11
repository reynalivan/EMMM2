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
/// The one image-extension set for the whole crate. Other modules must
/// reuse this rather than restating it.
pub use crate::platform::images::IMAGE_EXTENSIONS;

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

    list_preview_images(mod_path)
        .into_iter()
        .min_by(|left, right| {
            thumbnail_priority(mod_path, left)
                .cmp(&thumbnail_priority(mod_path, right))
                .then_with(|| left.cmp(right))
        })
}

fn thumbnail_priority(root: &Path, path: &Path) -> (u8, u8) {
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if path.parent() != Some(root) || !stem.starts_with("preview") {
        return (2, 0);
    }
    let weight = match path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
        .as_str()
    {
        "png" => 0,
        "jpg" | "jpeg" => 1,
        "webp" => 2,
        "gif" => 3,
        _ => 4,
    };
    (u8::from(stem != "preview_custom"), weight)
}

/// List preview images for gallery rendering with deterministic ordering.
///
/// Order:
/// 1. `preview_custom.*` in root
/// 2. other `preview*` in root
/// 3. fallback to all images up to depth 3 (if no root `preview*` exists)
pub fn list_preview_images(mod_path: &Path) -> Vec<PathBuf> {
    let entries = match crate::platform::images::preview_visibility::apply_visibility(
        mod_path,
        list_all_preview_images(mod_path),
    ) {
        Ok(entries) => entries,
        Err(error) => {
            log::warn!(
                "Could not read preview visibility for {}: {error}",
                mod_path.display()
            );
            return Vec::new();
        }
    };
    entries
        .into_iter()
        .filter(|entry| !entry.is_hidden)
        .map(|entry| entry.path)
        .collect()
}

pub fn list_all_preview_images(mod_path: &Path) -> Vec<PathBuf> {
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

/// Check if a file has an image extension.
pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "tests/thumbnail_tests.rs"]
mod tests;
