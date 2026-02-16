use emmm2_lib::services::config::{AppSettings, ConfigService};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

// Covers: TC-11.1-04 (save/load settings path)
#[test]
fn test_config_save_and_load() {
    let dir = tempdir().expect("temp dir should be created");
    let config_path = dir.path().join("config.json");
    let service = ConfigService::new(config_path.clone());

    let settings = service.get_settings();
    assert_eq!(settings.theme, "dark");

    let mut next_settings = settings.clone();
    next_settings.theme = "light".to_string();
    service
        .save_settings(next_settings)
        .expect("save should succeed");

    assert_eq!(service.get_settings().theme, "light");

    let service_reloaded = ConfigService::new(config_path);
    assert_eq!(service_reloaded.get_settings().theme, "light");
}

// Covers: TC-11.4-01, DI-11.03 (PIN hash storage + verify)
#[test]
fn test_config_pin_operations() {
    let dir = tempdir().expect("temp dir should be created");
    let config_path = dir.path().join("config.json");
    let service = ConfigService::new(config_path);

    assert!(service.verify_pin("anything"));

    service
        .set_pin("123456")
        .expect("setting a valid pin should succeed");

    assert!(service.verify_pin("123456"));
    assert!(!service.verify_pin("wrong"));

    let stored = fs::read_to_string(dir.path().join("config.json")).expect("config should exist");
    assert!(stored.contains("pin_hash"));
    assert!(!stored.contains("123456"));
}

// Covers: EC-11.01 (legacy migration path remains stable)
#[test]
fn test_config_legacy_migration() {
    let dir = tempdir().expect("temp dir should be created");
    let config_path = dir.path().join("config.json");

    let legacy_json = r#"{
        "games": [
            {
                "id": "1",
                "name": "Legacy Game",
                "game_type": "Genshin",
                "path": "C:\\Game\\Genshin.exe",
                "mods_path": "C:\\Mods",
                "launcher_path": null,
                "launch_args": null
            }
        ],
        "active_game": "1",
        "safe_mode": false
    }"#;
    fs::write(&config_path, legacy_json).expect("legacy config should be written");

    let service = ConfigService::new(config_path.clone());
    let settings = service.get_settings();

    assert_eq!(settings.games.len(), 1);
    assert_eq!(settings.games[0].name, "Legacy Game");

    let saved_content = fs::read_to_string(&config_path).expect("config should exist");
    let saved_settings: AppSettings =
        serde_json::from_str(&saved_content).expect("saved config should be valid app settings");

    assert_eq!(saved_settings.games.len(), 1);
    assert_eq!(saved_settings.games[0].mod_path, PathBuf::from("C:\\Mods"));
    assert!(!saved_content.contains("mods_path"));
}

// Covers: NC-11.4-01, DI-11.04 (5 failed attempts trigger lockout)
#[test]
fn test_pin_lockout_after_five_failures() {
    let dir = tempdir().expect("temp dir should be created");
    let config_path = dir.path().join("config.json");
    let service = ConfigService::new(config_path);

    service
        .set_pin("123456")
        .expect("setting a valid pin should succeed");

    for _ in 0..4 {
        let status = service.verify_pin_status("000000");
        assert!(!status.valid);
        assert_eq!(status.locked_seconds_remaining, 0);
    }

    let status = service.verify_pin_status("000000");
    assert!(!status.valid);
    assert_eq!(status.attempts_remaining, 0);
    assert!(status.locked_seconds_remaining > 0);
}

// Covers: EC-11.01 (corrupt config falls back to defaults)
#[test]
fn test_corrupt_config_falls_back_to_defaults() {
    let dir = tempdir().expect("temp dir should be created");
    let config_path = dir.path().join("config.json");
    fs::write(&config_path, "\u{0000}\u{0001}not-json").expect("corrupt config should be written");

    let service = ConfigService::new(config_path);
    let settings = service.get_settings();

    assert_eq!(settings.theme, "dark");
    assert_eq!(settings.language, "en");
    assert!(settings.games.is_empty());
}
