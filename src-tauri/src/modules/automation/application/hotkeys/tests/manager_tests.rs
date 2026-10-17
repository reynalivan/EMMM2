//! Unit tests for parsing and registration boundaries. Plugin registration is
//! intentionally covered by the Tauri integration layer, not this module.

use crate::modules::automation::application::hotkeys::manager::{
    parse_hotkey, validate_binding_configuration, HotkeyGameContext, HotkeyManager,
    ShortcutRegistrationSink,
};
use crate::modules::automation::application::hotkeys::{
    HotkeyAction, HotkeyConfig, KeyViewerConfig,
};
use crate::modules::games::domain::models::{GameType, LaunchMode};
use crate::modules::settings::application::config::{AppSettings, ConfigService, GameConfig};
use crate::shared::errors::AppError;
use std::path::PathBuf;
use tauri_plugin_global_shortcut::Shortcut;

#[derive(Default)]
struct FakeShortcutSink {
    operations: usize,
    on_first_register: Option<Box<dyn FnOnce() + Send>>,
    reject_shortcut: Option<u32>,
}

impl ShortcutRegistrationSink for FakeShortcutSink {
    fn unregister_all(&mut self) -> Result<(), AppError> {
        self.operations += 1;
        Ok(())
    }

    fn register(&mut self, _shortcut: Shortcut) -> Result<(), AppError> {
        self.operations += 1;
        if self.reject_shortcut == Some(_shortcut.id()) {
            return Err(AppError::Internal(
                "injected restoration registration failure".to_string(),
            ));
        }
        if let Some(callback) = self.on_first_register.take() {
            callback();
        }
        Ok(())
    }

    fn unregister(&mut self, _shortcut: Shortcut) -> Result<(), AppError> {
        self.operations += 1;
        Ok(())
    }
}

#[test]
fn parses_single_key() {
    assert_eq!(parse_hotkey("F5").unwrap(), "F5");
}

#[test]
fn parses_modifier_key_case_insensitively() {
    assert_eq!(parse_hotkey("Shift+F6").unwrap(), "shift+F6");
    assert_eq!(parse_hotkey("SHIFT+f6").unwrap(), "shift+F6");
}

#[test]
fn os_shortcut_aliases_use_the_plugin_descriptor() {
    assert_eq!(parse_hotkey("Ctrl+F5").unwrap(), "control+F5");
    assert_eq!(
        parse_hotkey("Alt+Ctrl+A").unwrap(),
        parse_hotkey("control+alt+KeyA").unwrap()
    );
}

#[test]
fn rejects_invalid_or_injectable_os_hotkeys() {
    for invalid in ["", "F7\n[Constants]", "F7;run = Evil", "F7=1", "Ctrl+F6++"] {
        assert!(parse_hotkey(invalid).is_err(), "must reject '{invalid}'");
    }
}

#[test]
fn rejects_3dmigoto_negative_modifiers_for_os_registration() {
    assert!(parse_hotkey("NO_CTRL+F5").is_err());
}

#[test]
fn rejects_negative_modifiers_for_the_globally_registered_overlay_binding() {
    let config = HotkeyConfig {
        toggle_overlay: "NO_CTRL+F7".to_string(),
        ..Default::default()
    };
    assert!(validate_binding_configuration(&config).is_err());
}

#[test]
fn manager_has_no_actions_until_tauri_registers_shortcuts() {
    let manager = HotkeyManager::new(&HotkeyConfig::default());
    assert_eq!(manager.lookup_action("f5"), None);
    assert_eq!(manager.lookup_action("shift+f6"), None);
}

#[test]
fn manager_looks_up_plugin_descriptor_by_os_identity() {
    let manager = HotkeyManager::new(&HotkeyConfig::default());
    let lifecycle = manager.registration_lifecycle();
    let mut sink = FakeShortcutSink::default();
    manager
        .update_bindings_with_sink(&lifecycle, &mut sink, &HotkeyConfig::default())
        .unwrap();

    assert_eq!(
        manager.lookup_action("control+F5"),
        Some(HotkeyAction::NextPreset)
    );
}

#[test]
fn registration_lifecycle_serializes_overlay_restore_and_save() {
    let manager = std::sync::Arc::new(HotkeyManager::new(&HotkeyConfig::default()));
    let overlay_restore = manager.registration_lifecycle();
    let (sender, receiver) = std::sync::mpsc::channel();
    let concurrent_save = std::sync::Arc::clone(&manager);
    let worker = std::thread::spawn(move || {
        let _save = concurrent_save.registration_lifecycle();
        sender.send(()).unwrap();
    });

    assert!(receiver
        .recv_timeout(std::time::Duration::from_millis(50))
        .is_err());
    drop(overlay_restore);
    receiver
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    worker.join().unwrap();
}

#[test]
fn rejects_os_equivalent_duplicate_bindings() {
    let config = HotkeyConfig {
        next_preset: "Ctrl+A".to_string(),
        prev_preset: "control+KeyA".to_string(),
        ..Default::default()
    };
    assert!(validate_binding_configuration(&config).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_revision_is_rejected_before_runtime_registration() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool);
    let manager = HotkeyManager::new(&HotkeyConfig::default());
    let mut sink = FakeShortcutSink::default();

    let result = crate::modules::automation::adapters::tauri::hotkey_cmds::commit_hotkey_configuration_with_sink(
        service.get_settings().revision + 1,
        HotkeyConfig::default(),
        KeyViewerConfig::default(),
        &service,
        &manager,
        &mut sink,
    );

    assert!(result.is_err());
    assert_eq!(sink.operations, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persistence_race_restores_latest_committed_bindings() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = std::sync::Arc::new(ConfigService::new_for_test(pool));
    let expected_revision = service.get_settings().revision;
    let latest = HotkeyConfig {
        next_preset: "Ctrl+F8".to_string(),
        ..Default::default()
    };
    let requested = HotkeyConfig {
        next_preset: "Ctrl+F9".to_string(),
        ..Default::default()
    };
    let manager = HotkeyManager::new(&HotkeyConfig::default());
    let service_for_race = std::sync::Arc::clone(&service);
    let latest_for_race = latest.clone();
    let mut sink = FakeShortcutSink {
        operations: 0,
        reject_shortcut: None,
        on_first_register: Some(Box::new(move || {
            service_for_race
                .set_hotkey_configuration(
                    expected_revision,
                    latest_for_race,
                    KeyViewerConfig::default(),
                )
                .unwrap();
        })),
    };

    let result = crate::modules::automation::adapters::tauri::hotkey_cmds::commit_hotkey_configuration_with_sink(
        expected_revision,
        requested,
        KeyViewerConfig::default(),
        &service,
        &manager,
        &mut sink,
    );

    assert!(result.is_err());
    assert_eq!(service.get_settings().hotkeys.next_preset, "Ctrl+F8");
    assert_eq!(
        manager.lookup_action("control+F8"),
        Some(HotkeyAction::NextPreset)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persistence_failure_reports_failed_os_restoration() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = std::sync::Arc::new(ConfigService::new_for_test(pool));
    let revision = service.get_settings().revision;
    let latest = HotkeyConfig {
        next_preset: "Ctrl+F8".to_string(),
        ..Default::default()
    };
    let requested = HotkeyConfig {
        next_preset: "Ctrl+F9".to_string(),
        ..Default::default()
    };
    let manager = HotkeyManager::new(&HotkeyConfig::default());
    let concurrent = std::sync::Arc::clone(&service);
    let mut sink = FakeShortcutSink {
        operations: 0,
        reject_shortcut: Some("Ctrl+F8".parse::<Shortcut>().unwrap().id()),
        on_first_register: Some(Box::new(move || {
            concurrent
                .set_hotkey_configuration(revision, latest, KeyViewerConfig::default())
                .unwrap();
        })),
    };
    let error = crate::modules::automation::adapters::tauri::hotkey_cmds::commit_hotkey_configuration_with_sink(
        revision, requested, KeyViewerConfig::default(), &service, &manager, &mut sink,
    ).unwrap_err().to_string();
    assert!(error.contains("Settings changed"), "{error}");
    assert!(
        error.contains("injected restoration registration failure"),
        "{error}"
    );
}

fn game(id: &str, mods_path: PathBuf) -> GameConfig {
    GameConfig {
        id: id.to_string(),
        name: id.to_string(),
        game_type: GameType::Gimi,
        instance_path: mods_path.clone(),
        mod_path: mods_path,
        ready_to_move_path: None,
        launch_mode: LaunchMode::Standalone,
        game_exe: Some(PathBuf::from(format!("C:/Games/{id}.exe"))),
        loader_exe: None,
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    }
}

fn overlay_settings() -> AppSettings {
    AppSettings {
        games: vec![game("game", PathBuf::from("unused-overlay-root"))],
        active_game_id: Some("game".into()),
        ..AppSettings::default()
    }
}

#[test]
fn overlay_rejects_changed_binding_game_executable_and_disabled_hotkeys() {
    use super::super::manager::validate_overlay_replay_settings;
    let original = overlay_settings();
    assert!(validate_overlay_replay_settings(&original, &original).is_ok());
    let mut disabled = original.clone();
    disabled.hotkeys.enabled = false;
    let mut rebound = original.clone();
    rebound.hotkeys.toggle_overlay = "Ctrl+F8".into();
    let mut executable = original.clone();
    executable.games[0].game_exe = Some(PathBuf::from("other.exe"));
    let mut selected = original.clone();
    selected.active_game_id = None;
    let mut game_type = original.clone();
    game_type.games[0].game_type = GameType::Wwmi;
    for changed in [disabled, rebound, executable, selected, game_type] {
        assert!(validate_overlay_replay_settings(&original, &changed).is_err());
    }
}

#[test]
fn overlay_restores_registration_after_focus_loss_or_input_failure() {
    let settings = overlay_settings();
    let manager = HotkeyManager::new(&settings.hotkeys);
    let lifecycle = manager.registration_lifecycle();
    let mut sink = FakeShortcutSink::default();
    manager
        .update_bindings_with_sink(&lifecycle, &mut sink, &settings.hotkeys)
        .unwrap();
    let before = sink.operations;
    let error = manager
        .replay_overlay_with_sink(
            &lifecycle,
            &mut sink,
            &settings,
            || settings.clone(),
            |_, _| Err(AppError::Validation("active game is not focused".into())),
        )
        .unwrap_err();
    assert!(error.to_string().contains("active game is not focused"));
    assert_eq!(sink.operations - before, 6); // suspend, reset, four registrations
    assert_eq!(
        manager.lookup_action(&settings.hotkeys.toggle_overlay),
        Some(HotkeyAction::ToggleOverlay)
    );
}

#[test]
fn overlay_canceled_during_release_wait_never_suspends_or_replays() {
    let settings = overlay_settings();
    let manager = HotkeyManager::new(&settings.hotkeys);
    let lifecycle = manager.registration_lifecycle();
    let mut sink = FakeShortcutSink::default();
    let mut disabled = settings.clone();
    disabled.hotkeys.enabled = false;
    let result = manager.replay_overlay_with_sink(
        &lifecycle,
        &mut sink,
        &settings,
        || disabled.clone(),
        |_, _| panic!("Canceled overlay must not send input"),
    );
    assert!(result.is_err());
    assert_eq!(sink.operations, 0);
}

#[test]
fn press_context_allows_actions_without_a_foreground_game() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("mods");
    std::fs::create_dir_all(&root).unwrap();
    let mut selected_game = game("game", root);
    selected_game.game_exe = None;
    let settings = AppSettings {
        games: vec![selected_game],
        active_game_id: Some("game".to_string()),
        ..AppSettings::default()
    };
    let context = HotkeyGameContext::capture(&settings).unwrap();

    assert!(context.validate(&settings).is_ok());
}

#[test]
fn press_context_rejects_a_different_selected_game() {
    let temp = tempfile::TempDir::new().unwrap();
    let first_root = temp.path().join("first");
    let second_root = temp.path().join("second");
    std::fs::create_dir_all(&first_root).unwrap();
    std::fs::create_dir_all(&second_root).unwrap();
    let mut settings = AppSettings {
        games: vec![game("first", first_root), game("second", second_root)],
        active_game_id: Some("first".to_string()),
        ..AppSettings::default()
    };
    let context = HotkeyGameContext::capture(&settings).unwrap();
    settings.active_game_id = Some("second".to_string());

    assert!(context.validate(&settings).is_err());
}

#[test]
fn press_context_rejects_disabled_hotkeys() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("mods");
    std::fs::create_dir_all(&root).unwrap();
    let mut settings = AppSettings {
        games: vec![game("game", root)],
        active_game_id: Some("game".to_string()),
        ..AppSettings::default()
    };
    let context = HotkeyGameContext::capture(&settings).unwrap();

    settings.hotkeys.enabled = false;
    assert!(context.validate(&settings).is_err());
}

#[test]
fn press_context_rejects_a_replaced_mods_root() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("mods");
    std::fs::create_dir_all(&root).unwrap();
    let settings = AppSettings {
        games: vec![game("game", root.clone())],
        active_game_id: Some("game".to_string()),
        ..AppSettings::default()
    };
    let context = HotkeyGameContext::capture(&settings).unwrap();
    std::fs::remove_dir(&root).unwrap();
    std::fs::create_dir(&root).unwrap();

    assert!(context.validate(&settings).is_err());
}
