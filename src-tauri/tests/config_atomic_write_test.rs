use emmm2_lib::services::config::{AppSettings, ConfigService};
use std::fs;
use tempfile::tempdir;

// Covers: TC-11.1-04, DI-11.02 (config updates remain writable atomically after first save)
#[test]
fn save_settings_can_overwrite_existing_config_file() {
    let dir = tempdir().expect("temp dir should be created");
    let config_path = dir.path().join("config.json");
    let service = ConfigService::new(config_path.clone());

    let mut first = service.get_settings();
    first.theme = "light".to_string();
    service
        .save_settings(first)
        .expect("first save should create config file");

    let mut second = service.get_settings();
    second.language = "id".to_string();

    let second_save = service.save_settings(second.clone());
    assert!(
        second_save.is_ok(),
        "second save should replace existing config file without failing: {second_save:?}"
    );

    let saved_content = fs::read_to_string(&config_path).expect("config should exist");
    let saved_settings: AppSettings =
        serde_json::from_str(&saved_content).expect("saved config should be valid JSON");

    assert_eq!(saved_settings.language, "id");
}
