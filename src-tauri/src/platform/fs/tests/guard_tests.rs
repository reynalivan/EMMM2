use super::{
    validate_dir_in_configured_roots, validate_mod_toggle_paths, validate_mods_root, validate_path,
};
use crate::modules::settings::application::config::{ConfigService, GameConfig};
use std::fs;
use tempfile::TempDir;

async fn config_with_game(mod_path: &std::path::Path) -> ConfigService {
    let pool = crate::test_utils::init_test_db().await.pool;
    let config = ConfigService::new_for_test_async(pool).await;

    let mut settings = config.get_settings();
    settings.games.push(GameConfig {
        id: "game-1".to_string(),
        name: "Test Game".to_string(),
        game_type: crate::modules::games::domain::models::GameType::GIMI,
        instance_path: mod_path.to_path_buf(),
        mod_path: mod_path.to_path_buf(),
        ready_to_move_path: None,
        launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
        game_exe: Some(mod_path.join("game.exe")),
        loader_exe: None,
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    });
    config.save_settings(settings).expect("save settings");
    config
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn accepts_dir_inside_a_configured_root() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    let inside = mods_root.join("Character");
    fs::create_dir_all(&inside).unwrap();

    let config = config_with_game(&mods_root).await;

    let requested = inside.to_string_lossy().to_string();
    let result = validate_dir_in_configured_roots(&config, &requested)
        .expect("dir inside the mods root must pass");
    assert_eq!(result.original(), requested);
    assert_eq!(result.owner_game_id(), "game-1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn relative_and_canonical_spellings_resolve_to_the_same_entry() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    let inside = mods_root.join("Character");
    fs::create_dir_all(&inside).unwrap();
    let config = config_with_game(&mods_root).await;

    let relative = validate_path(&config, "game-1", "Character").unwrap();
    let canonical_path = inside.canonicalize().unwrap();
    let canonical = validate_path(&config, "game-1", &canonical_path.to_string_lossy()).unwrap();

    assert_eq!(relative.as_ref(), canonical.as_ref());
    assert_eq!(relative.owner_game_id(), "game-1");
}

#[cfg(windows)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn alternate_windows_casing_keeps_the_same_owner() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    let inside = mods_root.join("Character");
    fs::create_dir_all(&inside).unwrap();
    let config = config_with_game(&mods_root).await;

    let alternate_case = inside.to_string_lossy().to_uppercase();
    let validated = validate_dir_in_configured_roots(&config, &alternate_case).unwrap();

    assert_eq!(validated.as_ref(), inside.canonicalize().unwrap());
    assert_eq!(validated.owner_game_id(), "game-1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn most_specific_canonical_root_owns_nested_targets() {
    let tmp = TempDir::new().unwrap();
    let outer_root = tmp.path().join("OuterMods");
    let inner_root = outer_root.join("InnerMods");
    let target = inner_root.join("Character");
    fs::create_dir_all(&target).unwrap();
    let config = config_with_game(&outer_root).await;

    let mut settings = config.get_settings();
    settings.games.push(GameConfig {
        id: "game-2".to_string(),
        name: "Nested Game".to_string(),
        game_type: crate::modules::games::domain::models::GameType::GIMI,
        instance_path: target.clone(),
        mod_path: inner_root,
        ready_to_move_path: None,
        launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
        game_exe: Some(target.join("game.exe")),
        loader_exe: None,
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    });
    config.save_settings(settings).expect("save settings");

    let validated = validate_dir_in_configured_roots(&config, &target.to_string_lossy()).unwrap();
    assert_eq!(validated.owner_game_id(), "game-2");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn rejects_identical_canonical_roots_with_different_owners() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    let target = mods_root.join("Character");
    fs::create_dir_all(&target).unwrap();
    let config = config_with_game(&mods_root).await;

    let mut settings = config.get_settings();
    settings.games.push(GameConfig {
        id: "game-2".to_string(),
        name: "Duplicate Root Game".to_string(),
        game_type: crate::modules::games::domain::models::GameType::GIMI,
        instance_path: target.clone(),
        mod_path: mods_root,
        ready_to_move_path: None,
        launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
        game_exe: Some(target.join("other-game.exe")),
        loader_exe: None,
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    });
    config.save_settings(settings).expect("save settings");

    let error = validate_dir_in_configured_roots(&config, &target.to_string_lossy())
        .expect_err("an identical root cannot have two implicit owners");
    assert!(matches!(
        error,
        crate::shared::errors::AppError::Security(_)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn unique_inner_root_wins_over_ambiguous_outer_roots() {
    let tmp = TempDir::new().unwrap();
    let outer_root = tmp.path().join("Mods");
    let inner_root = outer_root.join("NestedGame");
    let target = inner_root.join("Character");
    fs::create_dir_all(&target).unwrap();
    let config = config_with_game(&outer_root).await;

    let mut settings = config.get_settings();
    settings.games.push(GameConfig {
        id: "game-2".to_string(),
        name: "Duplicate Outer Root".to_string(),
        game_type: crate::modules::games::domain::models::GameType::GIMI,
        instance_path: tmp.path().join("GameTwo"),
        mod_path: outer_root,
        ready_to_move_path: None,
        launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
        game_exe: Some(target.join("outer-game.exe")),
        loader_exe: None,
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    });
    settings.games.push(GameConfig {
        id: "game-3".to_string(),
        name: "Nested Game".to_string(),
        game_type: crate::modules::games::domain::models::GameType::GIMI,
        instance_path: tmp.path().join("GameThree"),
        mod_path: inner_root,
        ready_to_move_path: None,
        launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
        game_exe: Some(target.join("inner-game.exe")),
        loader_exe: None,
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    });
    config.save_settings(settings).expect("save settings");

    let validated = validate_dir_in_configured_roots(&config, &target.to_string_lossy()).unwrap();
    assert_eq!(validated.owner_game_id(), "game-3");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn rejects_dir_outside_every_configured_root() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    fs::create_dir_all(&mods_root).unwrap();
    let outside = tmp.path().join("Elsewhere");
    fs::create_dir_all(&outside).unwrap();

    let config = config_with_game(&mods_root).await;

    let result = validate_dir_in_configured_roots(&config, &outside.to_string_lossy());
    assert!(
        result.is_err(),
        "dir outside the mods root must be rejected"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn rejects_traversal_escaping_the_root() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    fs::create_dir_all(&mods_root).unwrap();

    let config = config_with_game(&mods_root).await;

    let sneaky = mods_root.join("..");
    let result = validate_dir_in_configured_roots(&config, &sneaky.to_string_lossy());
    assert!(result.is_err(), "`..` escape must be rejected");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn exact_mods_root_guard_rejects_a_child_directory() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    let child = mods_root.join("Character");
    fs::create_dir_all(&child).unwrap();
    let config = config_with_game(&mods_root).await;

    let error = validate_mods_root(&config, "game-1", &child.to_string_lossy())
        .expect_err("watcher/scanner roots must match the configured root exactly");

    assert!(matches!(
        error,
        crate::shared::errors::AppError::Security(_)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn bulk_toggle_keeps_missing_items_local_but_rejects_the_mods_root() {
    let tmp = TempDir::new().unwrap();
    let mods_root = tmp.path().join("Mods");
    let enabled = mods_root.join("Enabled");
    fs::create_dir_all(&enabled).unwrap();
    let config = config_with_game(&mods_root).await;

    let requested = vec!["Enabled".to_string(), "Missing".to_string()];
    let (valid, failures) = validate_mod_toggle_paths(&config, "game-1", &requested)
        .expect("a stale item must not invalidate an otherwise safe batch");
    assert_eq!(valid.len(), 1);
    assert_eq!(valid[0].original(), "Enabled");
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].0, "Missing");

    let root_error = validate_mod_toggle_paths(
        &config,
        "game-1",
        &[mods_root.to_string_lossy().into_owned()],
    )
    .expect_err("the configured root must never be toggled");
    assert!(matches!(
        root_error,
        crate::shared::errors::AppError::Security(_)
    ));
}
