use super::*;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_detect_conflicts_in_folder_integration() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    let mod_disabled = dir.path().join("DISABLED ModC");

    fs::create_dir(&mod_a).unwrap();
    fs::create_dir(&mod_b).unwrap();
    fs::create_dir(&mod_disabled).unwrap();

    // Conflict between ModA and ModB
    fs::write(
        mod_a.join("config.ini"),
        "[TextureOverrideBody]\nhash = abc123\n",
    )
    .unwrap();
    fs::write(
        mod_b.join("config.ini"),
        "[TextureOverrideBody]\nhash = abc123\n",
    )
    .unwrap();

    // ModC has same hash but is DISABLED, so should be ignored
    fs::write(
        mod_disabled.join("config.ini"),
        "[TextureOverrideBody]\nhash = abc123\n",
    )
    .unwrap();

    let conflicts = detect_conflicts_in_folder_cmd(dir.path().to_string_lossy().to_string())
        .await
        .unwrap();

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].hash, "abc123");
    assert_eq!(conflicts[0].mod_paths.len(), 2);
}
