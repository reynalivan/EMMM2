use super::*;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn paste_thumbnail_rejects_oversize() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let config = crate::services::config::ConfigService::new_for_test(pool);

    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModThumb");
    fs::create_dir(&mod_dir).unwrap();

    let oversized = vec![0_u8; 10 * 1024 * 1024 + 1];

    let result = paste_thumbnail_inner(
        &config,
        "test_game".to_string(),
        mod_dir.to_string_lossy().to_string(),
        oversized,
    )
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Image too large"));
}
