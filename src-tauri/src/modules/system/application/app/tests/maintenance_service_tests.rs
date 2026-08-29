use crate::modules::system::application::app::maintenance_service::run_maintenance_counts;
use crate::platform::images::thumbnail_cache::ThumbnailCache;
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

#[tokio::test]
async fn test_run_maintenance() {
    let temp_dir = TempDir::new().unwrap();
    let app_data_dir = temp_dir.path();
    ThumbnailCache::init(app_data_dir); // Ensure thumbnail dir exists
    let pool = setup_test_db().await;

    let pruned = run_maintenance_counts(&pool, app_data_dir).await.unwrap();
    assert_eq!(pruned, 0); // Initially empty
}

/// Maintenance used to keep only cache entries whose key matched an
/// `objects.thumbnail_path`. The cache is keyed by the image found inside a
/// mod folder, which is a different population, so a freshly resolved
/// thumbnail was deleted on every run.
#[tokio::test]
async fn maintenance_keeps_a_freshly_cached_thumbnail() {
    let temp_dir = TempDir::new().unwrap();
    let app_data_dir = temp_dir.path();
    ThumbnailCache::init(app_data_dir);

    // A cache entry keyed the way `generate` names them: blake3 of the source
    // image path, which no `objects.thumbnail_path` row will ever mention.
    let cache_dir = crate::platform::images::thumbnail_cache::thumbnail_cache_dir(app_data_dir);
    fs::create_dir_all(&cache_dir).unwrap();
    let entry = cache_dir.join(format!(
        "{}.webp",
        blake3::hash(b"E:/Mods/Alice/Blue Dress/preview.png")
    ));
    fs::write(&entry, b"webp").unwrap();

    let pool = setup_test_db().await;
    let pruned = run_maintenance_counts(&pool, app_data_dir).await.unwrap();

    assert!(
        entry.exists(),
        "a just-cached thumbnail must survive maintenance"
    );
    assert_eq!(pruned, 0);
}
