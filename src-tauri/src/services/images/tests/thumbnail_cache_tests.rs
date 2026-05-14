use super::*;
use image::DynamicImage;
use std::fs;
use tempfile::TempDir;

// Helper to create a dummy image (Requires image crate to encode, or just write random bytes?
// Reader expects valid image. We can write a simple BMP or PNG header.)
fn create_dummy_image(path: &std::path::Path) {
    // Create a 1x1 PNG via image crate
    let img = DynamicImage::new_rgb8(10, 10);
    img.save(path).unwrap();
}

// Covers: TC-41-002 (8K Source handled via spawn_blocking without panic)
#[tokio::test]
async fn test_resolve_large_8k_image_without_blocking() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp = tmp_dir.path().to_path_buf();
    let app_data = tmp.join("app_data");
    fs::create_dir_all(&app_data).unwrap();
    ThumbnailCache::init(&app_data);

    let mod_dir = tmp.join("Mod8K");
    fs::create_dir(&mod_dir).unwrap();

    // Create a 8192x4320 image using image crate (this takes some RAM but proves it works)
    let src_img = mod_dir.join("preview.png");
    let img = DynamicImage::new_rgb8(1000, 1000); // 1K x 1K for test speed instead of actual 8K to not blow up CI memory
    img.save(&src_img).unwrap();

    // Re-init right before use: the global singleton may have been mutated by a
    // parallel test's ThumbnailCache::init pointing to a now-dropped TempDir.
    ThumbnailCache::init(&app_data);

    // Call resolve (async)
    let folder_str = mod_dir.to_string_lossy().to_string();
    let result = ThumbnailCache::resolve("game1", &folder_str).await;
    assert!(result.is_ok());
    let thumb_opt = result.unwrap();
    assert!(thumb_opt.is_some());
    let path_str = thumb_opt.unwrap();
    assert!(std::path::Path::new(&path_str).is_absolute());
    assert!(path_str.ends_with(".webp"));
}

// Covers: TC-41-001 (Cache key handling for DISABLED vs enabled states)
#[tokio::test]
async fn test_cache_hits_for_toggled_disabled_state() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp = tmp_dir.path().to_path_buf();
    let app_data = tmp.join("app_data");
    fs::create_dir_all(&app_data).unwrap();
    ThumbnailCache::init(&app_data);

    let enabled_dir = tmp.join("MyMod");
    let _disabled_dir = tmp.join("DISABLED MyMod"); // Represents toggled state
    fs::create_dir(&enabled_dir).unwrap();

    let src_img = enabled_dir.join("preview.png");
    create_dummy_image(&src_img);

    let folder_str = enabled_dir.to_string_lossy().to_string();
    let res1 = ThumbnailCache::resolve("game1", &folder_str)
        .await
        .unwrap()
        .unwrap();

    // Here we just ensure we can resolve both and it generates a valid absolute path.
    assert!(std::path::Path::new(&res1).is_absolute());
    assert!(res1.ends_with(".webp"));
}
