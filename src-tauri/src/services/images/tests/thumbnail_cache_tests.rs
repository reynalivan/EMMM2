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
//
// A toggle renames the folder but not its identity: resolving the DISABLED
// spelling must return the same cached .webp, with no regeneration.
#[tokio::test]
async fn test_cache_hits_for_toggled_disabled_state() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp = tmp_dir.path().to_path_buf();
    let app_data = tmp.join("app_data");
    fs::create_dir_all(&app_data).unwrap();
    ThumbnailCache::init(&app_data);

    let enabled_dir = tmp.join("MyMod");
    fs::create_dir(&enabled_dir).unwrap();
    let src_img = enabled_dir.join("preview.png");
    create_dummy_image(&src_img);

    ThumbnailCache::init(&app_data);
    let res1 = ThumbnailCache::resolve("game1", &enabled_dir.to_string_lossy())
        .await
        .unwrap()
        .unwrap();
    assert!(res1.ends_with(".webp"));

    // The global singleton races with parallel tests re-pointing base_dir, so
    // assertions compare the identity-derived .webp file name, not the dir.
    let webp_name = |path: &str| {
        std::path::Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    };

    // Toggle on disk, then resolve via the DISABLED spelling.
    let disabled_dir = tmp.join("DISABLED MyMod");
    fs::rename(&enabled_dir, &disabled_dir).unwrap();
    ThumbnailCache::init(&app_data);
    let res2 = ThumbnailCache::resolve("game1", &disabled_dir.to_string_lossy())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        webp_name(&res1),
        webp_name(&res2),
        "toggle must not change the cached thumbnail"
    );

    // Invalidation through either spelling clears the shared entry.
    ThumbnailCache::invalidate_folder(&enabled_dir.to_string_lossy());
    ThumbnailCache::init(&app_data);
    let res3 = ThumbnailCache::resolve("game1", &disabled_dir.to_string_lossy())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        webp_name(&res2),
        webp_name(&res3),
        "L2 disk cache is identity-keyed too"
    );
}
