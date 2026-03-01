use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

// Pull the private cleanup function through the service (it's pub(crate) or pub in the service module)
use crate::services::app::maintenance_service::cleanup_old_empty_trash_entries;

#[test]
fn test_cleanup_old_empty_trash_entries() {
    let tmp = TempDir::new().unwrap();
    let trash_dir = tmp.path();

    // Setup 1: Empty folder that is ancient
    let ancient_empty = trash_dir.join("ancient_empty");
    fs::create_dir_all(&ancient_empty).unwrap();

    // Explicitly modify the timestamp to 40 days ago
    let forty_days_ago = SystemTime::now() - Duration::from_secs(40 * 24 * 60 * 60);
    let ftime = filetime::FileTime::from_system_time(forty_days_ago);
    filetime::set_file_mtime(&ancient_empty, ftime).unwrap();

    // Setup 2: Empty folder that is recent (5 days ago)
    let recent_empty = trash_dir.join("recent_empty");
    fs::create_dir_all(&recent_empty).unwrap();
    let five_days_ago = SystemTime::now() - Duration::from_secs(5 * 24 * 60 * 60);
    let ftime2 = filetime::FileTime::from_system_time(five_days_ago);
    filetime::set_file_mtime(&recent_empty, ftime2).unwrap();

    // Setup 3: Ancient folder that is NOT empty (has metadata.json)
    let ancient_full = trash_dir.join("ancient_full");
    fs::create_dir_all(&ancient_full).unwrap();
    fs::write(ancient_full.join("metadata.json"), "{}").unwrap();
    filetime::set_file_mtime(&ancient_full, ftime).unwrap();

    // Run the cleanup
    let removed = cleanup_old_empty_trash_entries(trash_dir).unwrap();

    // Assertions
    assert_eq!(removed, 1);
    // ancient_empty should be deleted
    assert!(!ancient_empty.exists());
    // recent_empty should be retained
    assert!(recent_empty.exists());
    // ancient_full should be retained because of metadata.json
    assert!(ancient_full.exists());
}
