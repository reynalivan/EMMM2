use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn paste_thumbnail_rejects_oversize() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModThumb");
    fs::create_dir(&mod_dir).unwrap();

    let oversized = vec![0_u8; 10 * 1024 * 1024 + 1];

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(paste_thumbnail(
        mod_dir.to_string_lossy().to_string(),
        oversized,
    ));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Image too large"));
}
