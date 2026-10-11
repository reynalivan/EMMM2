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
    let names = fs::read_dir(&mod_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        names
            .iter()
            .all(|name| !name.contains(".tmp.") && !name.contains(".recover.")),
        "successful atomic save must not leave staging artifacts: {names:?}"
    );
}

#[test]
fn sanitize_object_name_fallbacks_to_mod() {
    assert_eq!(sanitize_object_name("***"), "mod");
    assert_eq!(sanitize_object_name("Raiden Shogun"), "raiden_shogun");
}

#[test]
fn clear_preview_images_hides_discovered_files_without_deleting_assets() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("preview_custom.png"), "x").unwrap();
    fs::write(mod_dir.join("preview_custom_1.png"), "x").unwrap();
    fs::write(mod_dir.join("texture.png"), "asset").unwrap();

    let removed = clear_preview_images(&mod_dir).unwrap();
    assert_eq!(removed.len(), 2);
    assert_eq!(fs::read(mod_dir.join("preview_custom.png")).unwrap(), b"x");
    assert_eq!(fs::read(mod_dir.join("texture.png")).unwrap(), b"asset");
    assert!(thumbnail::list_preview_images(&mod_dir).is_empty());
}

#[test]
fn hiding_fallback_image_preserves_runtime_asset_bytes() {
    let tmp = TempDir::new().unwrap();
    let asset = tmp.path().join("texture.png");
    fs::write(&asset, b"runtime texture bytes").unwrap();
    remove_preview_image(tmp.path(), &asset).unwrap();
    assert_eq!(fs::read(&asset).unwrap(), b"runtime texture bytes");
    assert!(thumbnail::find_thumbnail(tmp.path()).is_none());
}
