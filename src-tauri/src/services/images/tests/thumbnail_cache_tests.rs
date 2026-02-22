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

#[test]
fn test_get_thumbnail_generates_webp() {
    // Setup temp app data dir
    let tmp = TempDir::new().unwrap();
    let app_data = tmp.path().join("app_data");
    fs::create_dir_all(&app_data).unwrap();

    // Init cache
    ThumbnailCache::init(&app_data);

    // Create dummy source image
    let src_dir = tmp.path().join("Source");
    fs::create_dir(&src_dir).unwrap();
    let src_img = src_dir.join("test.png");
    create_dummy_image(&src_img);

    // Call get_thumbnail
    let result = ThumbnailCache::get_thumbnail(&src_img);
    assert!(result.is_ok());

    let thumb_path = result.unwrap();
    assert!(thumb_path.exists());
    assert_eq!(thumb_path.extension().unwrap(), "webp");

    // Verify it's in the cache dir
    let cache_dir = app_data.join("cache").join("thumbnails");
    assert!(thumb_path.starts_with(&cache_dir));
}
