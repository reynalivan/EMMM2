use crate::services::app::log_service::{open_log_folder_service, read_last_n_lines};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_read_last_n_lines_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    let missing_file = temp_dir.path().join("missing.log");

    let result = read_last_n_lines(&missing_file, 5).unwrap();
    assert_eq!(result, vec!["Log file not found."]);
}

#[test]
fn test_read_last_n_lines() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("test.log");

    let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6";
    fs::write(&log_file, content).unwrap();

    // Read last 3 lines
    let result = read_last_n_lines(&log_file, 3).unwrap();
    assert_eq!(result, vec!["Line 4", "Line 5", "Line 6"]);

    // Read more lines than exist
    let result = read_last_n_lines(&log_file, 10).unwrap();
    assert_eq!(result.len(), 6);
    assert_eq!(result[0], "Line 1");
}

#[test]
fn test_open_log_folder_service_creates_dir() {
    let temp_dir = TempDir::new().unwrap();
    let log_folder = temp_dir.path().join("logs_dir_test");

    assert!(!log_folder.exists());

    // NOTE: This test will actually attempt to spawn 'explorer' on Windows.
    // If explorer fails or succeeds, it should create the directory first.
    // We ignore the Result to avoid failing the test just because 'explorer'
    // might not execute flawlessly in a CI/headless environment.
    let _ = open_log_folder_service(&log_folder);

    // The key behavior is that it creates the folder if it doesn't exist
    assert!(log_folder.exists());
}
