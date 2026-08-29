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

// Covers: TC-41-003 (Image-first priority sorting: PNG > JPG > WEBP)
#[test]
fn test_find_thumbnail_image_extension_priority() {
    let dir = TempDir::new().unwrap();
    let mod_dir = dir.path().join("test_mod");
    fs::create_dir(&mod_dir).unwrap();

    fs::write(mod_dir.join("preview.webp"), "fake webp").unwrap();
    fs::write(mod_dir.join("preview.jpg"), "fake jpg").unwrap();
    fs::write(mod_dir.join("preview.png"), "fake png").unwrap();

    let result = find_thumbnail(&mod_dir).unwrap();

    // PNG should win over JPG and WEBP
    assert_eq!(
        result.file_name().unwrap().to_string_lossy(),
        "preview.png",
        "PNG should have highest priority"
    );

    // Remove PNG, JPG should win
    fs::remove_file(mod_dir.join("preview.png")).unwrap();
    let result = find_thumbnail(&mod_dir).unwrap();
    assert_eq!(
        result.file_name().unwrap().to_string_lossy(),
        "preview.jpg",
        "JPG should win over WEBP"
    );
}
