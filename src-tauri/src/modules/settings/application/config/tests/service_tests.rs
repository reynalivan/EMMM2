use super::*;
use crate::modules::games::domain::models::GameType;
use crate::modules::settings::application::config::GameConfig;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};

fn game(mod_path: &str) -> GameConfig {
    GameConfig {
        id: "game-a".to_string(),
        name: "Game A".to_string(),
        game_type: GameType::GIMI,
        instance_path: PathBuf::from(mod_path),
        mod_path: PathBuf::from(mod_path),
        ready_to_move_path: None,
        launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
        game_exe: Some(PathBuf::from("C:/Games/A/game.exe")),
        loader_exe: None,
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_external_tools_setting_defaults_to_no_mod_viewer_executable() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool);

    assert_eq!(
        service.get_settings().external_tools.mod_viewer_executable,
        None
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn catalog_auto_install_is_opt_in_and_persists_without_credentials() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool.clone());

    let defaults = service.get_settings().catalog_updates;
    assert!(defaults.auto_check);
    assert!(!defaults.auto_install);
    assert_eq!(defaults.last_successful_check_unix_seconds, None);

    let saved = service
        .set_catalog_auto_install(true)
        .expect("catalog preference should save");
    assert!(saved.catalog_updates.auto_check);
    assert!(saved.catalog_updates.auto_install);

    service
        .record_catalog_update_check(1_700_000_000)
        .expect("catalog check timestamp should save");
    let reloaded = ConfigService::new_for_test(pool);
    assert!(reloaded.get_settings().catalog_updates.auto_install);
    assert_eq!(
        reloaded
            .get_settings()
            .catalog_updates
            .last_successful_check_unix_seconds,
        Some(1_700_000_000)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mod_viewer_executable_set_replace_clear_and_reload() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool.clone());
    let temp = tempfile::TempDir::new().expect("temporary tool directory should create");
    let first = temp.path().join("mod-viewer.EXE");
    let replacement = temp.path().join("mod-viewer-replacement.exe");
    std::fs::write(&first, "first viewer").expect("first viewer fixture should write");
    std::fs::write(&replacement, "replacement viewer")
        .expect("replacement viewer fixture should write");

    let saved = service
        .set_mod_viewer_executable(Some(first.clone()))
        .expect("first executable should save");
    assert_eq!(saved.external_tools.mod_viewer_executable, Some(first));

    let replaced = service
        .set_mod_viewer_executable(Some(replacement.clone()))
        .expect("replacement executable should save");
    assert_eq!(
        replaced.external_tools.mod_viewer_executable,
        Some(replacement.clone())
    );

    let reloaded = ConfigService::new_for_test(pool.clone());
    assert_eq!(
        reloaded.get_settings().external_tools.mod_viewer_executable,
        Some(replacement)
    );

    service
        .set_mod_viewer_executable(None)
        .expect("clearing executable should save");
    let cleared = ConfigService::new_for_test(pool);
    assert_eq!(
        cleared.get_settings().external_tools.mod_viewer_executable,
        None
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mod_viewer_executable_rejects_relative_missing_directory_and_non_exe_paths() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool);
    let temp = tempfile::TempDir::new().expect("temporary tool directory should create");
    let missing = temp.path().join("missing.exe");
    let non_executable = temp.path().join("mod-viewer.txt");
    std::fs::write(&non_executable, "not an executable")
        .expect("non-executable fixture should write");

    for invalid in [
        PathBuf::from("mod-viewer.exe"),
        missing,
        temp.path().to_path_buf(),
        non_executable,
    ] {
        assert!(
            service.set_mod_viewer_executable(Some(invalid)).is_err(),
            "invalid Mod Viewer executable should be rejected"
        );
    }
    assert_eq!(
        service.get_settings().external_tools.mod_viewer_executable,
        None
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_full_snapshot_cannot_restore_previous_active_game() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = Arc::new(ConfigService::new_for_test(pool));
    service
        .set_active_game(Some("game-a".to_string()))
        .expect("initial active game should persist");

    let writer_ready = Arc::new(Barrier::new(2));
    let activation_done = Arc::new(Barrier::new(2));
    let stale_writer = {
        let service = Arc::clone(&service);
        let writer_ready = Arc::clone(&writer_ready);
        let activation_done = Arc::clone(&activation_done);
        std::thread::spawn(move || {
            let mut stale = service.get_settings();
            writer_ready.wait();
            activation_done.wait();
            stale.auto_close_launcher = true;
            service.save_settings(stale)
        })
    };

    writer_ready.wait();
    service
        .set_active_game(Some("game-b".to_string()))
        .expect("new active game should persist");
    activation_done.wait();

    assert!(
        stale_writer
            .join()
            .expect("stale writer thread should finish")
            .is_err(),
        "a stale full snapshot must not overwrite the recovered active game"
    );
    assert_eq!(
        service.get_settings().active_game_id.as_deref(),
        Some("game-b")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clearing_active_game_survives_config_reload() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool.clone());
    service
        .set_active_game(Some("game-a".to_string()))
        .expect("active game should persist");
    service
        .set_active_game(None)
        .expect("active game should clear");

    let reloaded = ConfigService::new_for_test(pool);
    assert_eq!(reloaded.get_settings().active_game_id, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_same_active_snapshot_cannot_restore_previous_mods_path() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool);
    let mut initial = service.get_settings();
    initial.games.push(game("C:/Mods/Old"));
    service
        .save_settings(initial)
        .expect("initial game should persist");
    service
        .set_active_game(Some("game-a".to_string()))
        .expect("game should become active");

    let mut stale = service.get_settings();
    service
        .update_settings(|settings| {
            settings.games[0].mod_path = PathBuf::from("C:/Mods/New");
            Ok(())
        })
        .expect("source repair should persist");
    stale.theme = "light".to_string();

    assert!(service.save_settings(stale).is_err());
    assert_eq!(
        service.get_settings().games[0].mod_path,
        PathBuf::from("C:/Mods/New")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_settings_save_cannot_change_existing_mods_path() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool);
    let mut initial = service.get_settings();
    initial.games.push(game("C:/Mods/Original"));
    service
        .save_settings(initial)
        .expect("initial game should persist");

    let mut edited = service.get_settings();
    edited.games[0].mod_path = PathBuf::from("C:/Mods/Replacement");
    let error = service
        .save_settings(edited)
        .expect_err("existing mods path must use source recovery");

    assert!(error.to_string().contains("source recovery"));
    assert_eq!(
        service.get_settings().games[0].mod_path,
        PathBuf::from("C:/Mods/Original")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_late_settings_write_rolls_back_database_and_memory() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool.clone());
    let mut initial = service.get_settings();
    initial.games.push(game("C:/Mods/A"));
    service
        .save_settings(initial)
        .expect("initial game should persist");

    sqlx::query(
        "CREATE TRIGGER fail_config_game_update BEFORE UPDATE ON games \
         BEGIN SELECT RAISE(ABORT, 'forced late settings write failure'); END",
    )
    .execute(&pool)
    .await
    .expect("fault trigger should install");

    assert!(service.set_auto_close_launcher(true).is_err());
    assert!(!service.get_settings().auto_close_launcher);
    let stored: String =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'auto_close_launcher'")
            .fetch_one(&pool)
            .await
            .expect("stored setting should load");
    assert_eq!(stored, "false");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn removing_game_survives_config_reload() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = ConfigService::new_for_test(pool.clone());
    let mut initial = service.get_settings();
    initial.games.push(game("C:/Mods/A"));
    service
        .save_settings(initial)
        .expect("initial game should persist");
    service
        .set_active_game(Some("game-a".to_string()))
        .expect("game should become active");

    let mut without_game = service.get_settings();
    without_game.games.clear();
    service
        .save_settings(without_game)
        .expect("game removal should persist");

    let reloaded = ConfigService::new_for_test(pool);
    assert!(reloaded.get_settings().games.is_empty());
    assert_eq!(reloaded.get_settings().active_game_id, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_authoritative_empty_snapshot_cannot_delete_persisted_game_children() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let persisted_game = game("C:/Mods/A");
    crate::modules::games::adapters::sqlite::game::upsert_game(
        &pool,
        &crate::modules::settings::application::config::models::config_to_game_row(&persisted_game),
    )
    .await
    .expect("game fixture should persist");
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, status) \
         VALUES ('object-a', 'game-a', 'Object A', 'Object A', 1)",
    )
    .execute(&pool)
    .await
    .expect("child fixture should persist");

    // Models the fail-closed cleanup boundary after a startup read failure:
    // memory has no authoritative games, while SQLite still has valid rows.
    let service = ConfigService {
        pool: pool.clone(),
        settings: Mutex::new(AppSettings::default()),
        settings_authoritative: AtomicBool::new(false),
    };
    assert!(service.set_auto_close_launcher(true).is_err());

    let game_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games WHERE id = 'game-a'")
        .fetch_one(&pool)
        .await
        .expect("game count should load");
    let object_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE id = 'object-a'")
            .fetch_one(&pool)
            .await
            .expect("object count should load");
    assert_eq!(game_count, 1);
    assert_eq!(object_count, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn database_reset_serializes_with_paused_settings_writer() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let service = Arc::new(ConfigService::new_for_test(pool.clone()));
    let mut initial = service.get_settings();
    initial.games.push(game("C:/Mods/A"));
    service
        .save_settings(initial)
        .expect("initial game should persist");

    let writer_entered = Arc::new(Barrier::new(2));
    let release_writer = Arc::new(Barrier::new(2));
    let writer = {
        let service = Arc::clone(&service);
        let writer_entered = Arc::clone(&writer_entered);
        let release_writer = Arc::clone(&release_writer);
        std::thread::spawn(move || {
            service.update_settings(|settings| {
                writer_entered.wait();
                release_writer.wait();
                settings.auto_close_launcher = true;
                Ok(())
            })
        })
    };
    writer_entered.wait();

    let app_data = tempfile::TempDir::new().expect("temp app data should create");
    let reset = {
        let service = Arc::clone(&service);
        let app_data_path = app_data.path().to_path_buf();
        std::thread::spawn(move || service.reset_database(&app_data_path))
    };
    release_writer.wait();

    writer
        .join()
        .expect("writer thread should finish")
        .expect("writer should commit before reset");
    reset
        .join()
        .expect("reset thread should finish")
        .expect("reset should succeed");

    let current = service.get_settings();
    assert!(current.revision > 0);
    assert!(current.games.is_empty());
    assert!(!current.auto_close_launcher);
    let game_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games")
        .fetch_one(&pool)
        .await
        .expect("game count should load");
    assert_eq!(game_count, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_captured_before_reset_cannot_resurrect_legacy_revision_zero_data() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let persisted_game = game("C:/Mods/A");
    crate::modules::games::adapters::sqlite::game::upsert_game(
        &pool,
        &crate::modules::settings::application::config::models::config_to_game_row(&persisted_game),
    )
    .await
    .expect("legacy game should persist without a revision key");
    let service = ConfigService::new_for_test(pool.clone());
    let stale = service.get_settings();
    assert_eq!(stale.revision, 0);

    let app_data = tempfile::TempDir::new().expect("temp app data should create");
    service
        .reset_database(app_data.path())
        .expect("reset should succeed");

    assert!(service.get_settings().revision > stale.revision);
    assert!(service.save_settings(stale).is_err());
    let game_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games")
        .fetch_one(&pool)
        .await
        .expect("game count should load");
    assert_eq!(game_count, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn late_reset_delete_failure_rolls_back_database_and_preserves_memory() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let persisted_game = game("C:/Mods/A");
    crate::modules::games::adapters::sqlite::game::upsert_game(
        &pool,
        &crate::modules::settings::application::config::models::config_to_game_row(&persisted_game),
    )
    .await
    .expect("game fixture should persist");
    crate::modules::system::adapters::sqlite::settings::set_setting(&pool, "theme", "light")
        .await
        .expect("theme fixture should persist");
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, status) \
         VALUES ('object-a', 'game-a', 'Object A', 'Object A', 1)",
    )
    .execute(&pool)
    .await
    .expect("child fixture should persist");
    sqlx::query(
        "CREATE TRIGGER fail_reset_game_delete BEFORE DELETE ON games \
         BEGIN SELECT RAISE(ABORT, 'forced late reset failure'); END",
    )
    .execute(&pool)
    .await
    .expect("fault trigger should install");
    let service = ConfigService::new_for_test(pool.clone());
    let before = service.get_settings();

    let app_data = tempfile::TempDir::new().expect("temp app data should create");
    let reset_error = service
        .reset_database(app_data.path())
        .expect_err("forced late delete failure must abort reset");
    assert!(reset_error
        .to_string()
        .contains("forced late reset failure"));

    let after = service.get_settings();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.theme, before.theme);
    assert_eq!(after.games.len(), 1);
    let game_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games")
        .fetch_one(&pool)
        .await
        .expect("game count should load");
    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects")
        .fetch_one(&pool)
        .await
        .expect("object count should load");
    let stored_theme: String =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'theme'")
            .fetch_one(&pool)
            .await
            .expect("theme should remain stored");
    assert_eq!(game_count, 1);
    assert_eq!(object_count, 1);
    assert_eq!(stored_theme, "light");
}
