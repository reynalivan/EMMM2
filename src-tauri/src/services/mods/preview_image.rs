use crate::services::images::thumbnail_cache::ThumbnailCache;
use crate::services::scanner::core::thumbnail;
use image::{imageops::FilterType, ImageFormat};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];
const MAX_WIDTH: u32 = 1920;
const MAX_HEIGHT: u32 = 1080;

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn sanitize_object_name(value: &str) -> String {
    let mut out = String::new();
    let mut previous_underscore = false;

    for ch in value.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch == '_' || ch == '-' || ch.is_whitespace() {
            Some('_')
        } else {
            None
        };

        let Some(c) = normalized else {
            continue;
        };

        if c == '_' {
            if previous_underscore {
                continue;
            }
            previous_underscore = true;
            out.push(c);
            continue;
        }

        previous_underscore = false;
        out.push(c);
    }

    let cleaned = out.trim_matches('_').to_string();
    if cleaned.is_empty() {
        "mod".to_string()
    } else {
        cleaned
    }
}

pub fn next_preview_filename(mod_root: &Path, object_name: &str) -> Result<String, String> {
    if !mod_root.exists() || !mod_root.is_dir() {
        return Err(format!("Invalid mod folder: {}", mod_root.display()));
    }

    let base = format!("preview_{}", sanitize_object_name(object_name));
    let mut used: HashSet<usize> = HashSet::new();

    let entries = fs::read_dir(mod_root)
        .map_err(|e| format!("Failed to read mod folder '{}': {e}", mod_root.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let lower = file_name.to_ascii_lowercase();
        let expected = format!("{}.webp", base);
        if lower == expected {
            used.insert(0);
            continue;
        }

        let prefix = base.to_string();
        if !lower.starts_with(&prefix) || !lower.ends_with(".webp") {
            continue;
        }

        let Some(stem) = Path::new(&lower).file_stem().and_then(|s| s.to_str()) else {
            continue;
        };

        if let Some(number_part) = stem.strip_prefix(&format!("{}_", base)) {
            if let Ok(number) = number_part.parse::<usize>() {
                used.insert(number);
            }
        }
    }

    if !used.contains(&0) {
        return Ok(format!("{}.webp", base));
    }

    for suffix in 1..10_000 {
        if !used.contains(&suffix) {
            let filename = format!("{}_{}.webp", base, suffix);
            if !mod_root.join(&filename).exists() {
                return Ok(filename);
            }
        }
    }

    Err("Unable to allocate preview image filename".to_string())
}

pub fn save_preview_image(
    mod_root: &Path,
    object_name: &str,
    image_data: &[u8],
) -> Result<PathBuf, String> {
    if !mod_root.exists() || !mod_root.is_dir() {
        return Err(format!("Invalid mod folder: {}", mod_root.display()));
    }

    let image =
        image::load_from_memory(image_data).map_err(|e| format!("Invalid image data: {e}"))?;

    let resized = if image.width() > MAX_WIDTH || image.height() > MAX_HEIGHT {
        image.resize(MAX_WIDTH, MAX_HEIGHT, FilterType::Lanczos3)
    } else {
        image
    };

    let filename = next_preview_filename(mod_root, object_name)?;
    let target_path = mod_root.join(filename);

    let mut encoded = Vec::new();
    resized
        .write_to(&mut Cursor::new(&mut encoded), ImageFormat::WebP)
        .map_err(|e| format!("Failed to encode preview image: {e}"))?;

    fs::write(&target_path, encoded).map_err(|e| format!("Failed to save preview image: {e}"))?;

    ThumbnailCache::invalidate(&target_path);
    Ok(target_path)
}

pub fn remove_preview_image(mod_root: &Path, image_path: &Path) -> Result<(), String> {
    let canonical_root = mod_root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize mod folder: {e}"))?;
    let canonical_target = image_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize image path: {e}"))?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err("Image path escapes mod folder".to_string());
    }

    if !canonical_target.is_file() || !is_image_file(&canonical_target) {
        return Err("Target is not a valid image file".to_string());
    }

    fs::remove_file(&canonical_target).map_err(|e| format!("Failed to remove image: {e}"))?;
    ThumbnailCache::invalidate(&canonical_target);
    Ok(())
}

pub fn clear_preview_images(mod_root: &Path) -> Result<Vec<String>, String> {
    if !mod_root.exists() || !mod_root.is_dir() {
        return Err(format!("Invalid mod folder: {}", mod_root.display()));
    }

    let mut unique = BTreeSet::new();
    for path in thumbnail::list_preview_images(mod_root) {
        unique.insert(path);
    }

    let mut removed = Vec::new();
    for path in unique {
        if !path.exists() {
            continue;
        }
        if !path.is_file() || !is_image_file(&path) {
            continue;
        }

        fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove '{}': {e}", path.display()))?;
        ThumbnailCache::invalidate(&path);
        removed.push(path.to_string_lossy().to_string());
    }

    Ok(removed)
}

#[cfg(test)]
#[path = "tests/preview_image_tests.rs"]
mod tests;
