use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn preview_filename_uses_first_available_suffix_gap() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path();

    fs::write(mod_dir.join("preview_keqing.webp"), "x").unwrap();
    fs::write(mod_dir.join("preview_keqing_2.webp"), "x").unwrap();

    let next = next_preview_filename(mod_dir, "Keqing").unwrap();
    assert_eq!(next, "preview_keqing_1.webp");
}

#[test]
fn save_preview_image_resizes_to_epic_limits_and_writes_webp() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();

    let source = image::DynamicImage::new_rgba8(4096, 2048);
    let mut bytes = Vec::new();
    source
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();

    let saved = save_preview_image(&mod_dir, "Keqing", &bytes).unwrap();
    assert_eq!(saved.extension().and_then(|e| e.to_str()), Some("webp"));

    let decoded = image::open(&saved).unwrap();
    assert!(decoded.width() <= MAX_WIDTH);
    assert!(decoded.height() <= MAX_HEIGHT);
}

#[test]
fn sanitize_object_name_fallbacks_to_mod() {
    assert_eq!(sanitize_object_name("***"), "mod");
    assert_eq!(sanitize_object_name("Raiden Shogun"), "raiden_shogun");
}

#[test]
fn clear_preview_images_removes_discovered_files() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("preview_custom.png"), "x").unwrap();
    fs::write(mod_dir.join("preview_custom_1.png"), "x").unwrap();

    let removed = clear_preview_images(&mod_dir).unwrap();
    assert_eq!(removed.len(), 2);
    assert!(!mod_dir.join("preview_custom.png").exists());
}
