use super::*;
use tempfile::TempDir;

// A folder that vanished mid-batch must surface as a failure, not be swallowed:
// its DB flag is already set, so a dropped write error hides a disk/DB mismatch.
#[test]
fn missing_folders_are_reported_not_swallowed() {
    let tmp = TempDir::new().unwrap();
    let present = tmp.path().join("Raiden_Outfit");
    std::fs::create_dir(&present).unwrap();
    let vanished = tmp.path().join("Renamed_By_Concurrent_Toggle");

    let update = info_json::ModInfoUpdate {
        is_favorite: Some(true),
        ..Default::default()
    };

    let result = partition_info_json_writes(
        vec![
            present.to_string_lossy().to_string(),
            vanished.to_string_lossy().to_string(),
        ],
        &update,
    );

    assert_eq!(result.success, vec![present.to_string_lossy().to_string()]);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].path, vanished.to_string_lossy());
    assert!(present.join("info.json").exists());
}
