use super::*;
use crate::modules::games::adapters::sqlite::game::{upsert_game, GameRow};
use crate::modules::settings::adapters::tauri::settings_cmds::persist_telemetry_consent;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::system::application::telemetry::TelemetryStore;
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

#[tokio::test]
async fn test_check_config_status_fresh_install() {
    let pool = setup_pool().await;

    // With 0 games, it should return FreshInstall
    let status = crate::modules::system::application::app::app_service::check_config_status(&pool)
        .await
        .unwrap();
    assert_eq!(status, ConfigStatus::FreshInstall);
}

#[tokio::test]
async fn test_check_config_status_has_config() {
    let pool = setup_pool().await;

    // Insert 1 game
    let game = GameRow {
        id: "game1".into(),
        name: "Test Game".into(),
        game_type: crate::modules::games::domain::models::GameType::Gimi,
        path: "C:\\Game".into(),
        mods_path: Some("C:\\Mods".into()),
        ready_to_move_path: None,
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_mode: "standalone".into(),
        xxmi_launcher_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    // With 1 game, it should return HasConfig
    let status = crate::modules::system::application::app::app_service::check_config_status(&pool)
        .await
        .unwrap();
    assert_eq!(status, ConfigStatus::HasConfig);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_error_completion_from_previous_revision_is_discarded_after_reopt_in() {
    let pool = setup_pool().await;
    let config = std::sync::Arc::new(ConfigService::new_for_test_async(pool.clone()).await);
    let telemetry = TelemetryStore::new(pool, false);
    let started = persist_telemetry_consent(true, &config, &telemetry)
        .await
        .unwrap();
    let task_config = std::sync::Arc::clone(&config);
    let task_telemetry = telemetry.clone();
    let (complete, completion) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        completion.await.unwrap();
        record_native_error_metric_inner(
            "launch",
            "io",
            Some(started.revision),
            &task_config,
            &task_telemetry,
        )
        .await;
    });

    persist_telemetry_consent(false, &config, &telemetry)
        .await
        .unwrap();
    persist_telemetry_consent(true, &config, &telemetry)
        .await
        .unwrap();
    complete.send(()).unwrap();
    task.await.unwrap();
    assert!(telemetry.list_rollups().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_error_without_origin_revision_is_discarded() {
    let pool = setup_pool().await;
    let config = ConfigService::new_for_test_async(pool.clone()).await;
    let telemetry = TelemetryStore::new(pool, false);
    persist_telemetry_consent(true, &config, &telemetry)
        .await
        .unwrap();

    record_native_error_metric_inner("launch", "io", None, &config, &telemetry).await;
    assert!(telemetry.list_rollups().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_error_with_current_origin_revision_records_bounded_metric() {
    let pool = setup_pool().await;
    let config = ConfigService::new_for_test_async(pool.clone()).await;
    let telemetry = TelemetryStore::new(pool, false);
    let started = persist_telemetry_consent(true, &config, &telemetry)
        .await
        .unwrap();

    record_native_error_metric_inner("launch", "io", Some(started.revision), &config, &telemetry)
        .await;
    let rollups = telemetry.list_rollups().await.unwrap();
    assert_eq!(rollups.len(), 1);
    assert_eq!(rollups[0].count, 1);
    assert_eq!(rollups[0].operation, "launch");
    assert_eq!(rollups[0].error_code, "io");
}

#[tokio::test]
async fn native_error_with_default_off_consent_is_discarded() {
    let pool = setup_pool().await;
    let config = ConfigService::new_for_test_async(pool.clone()).await;
    let telemetry = TelemetryStore::new(pool, false);

    record_native_error_metric_inner(
        "launch",
        "io",
        Some(config.get_settings().revision),
        &config,
        &telemetry,
    )
    .await;
    assert!(telemetry.list_rollups().await.unwrap().is_empty());
}
