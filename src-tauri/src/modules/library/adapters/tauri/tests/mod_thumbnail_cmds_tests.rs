use super::*;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn paste_thumbnail_rejects_oversize() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let config =
        crate::modules::settings::application::config::ConfigService::new_for_test_async(pool)
            .await;

    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModThumb");
    fs::create_dir(&mod_dir).unwrap();

    let oversized = vec![0_u8; 10 * 1024 * 1024 + 1];

    let result = paste_thumbnail_inner(
        &config,
        "test_game".to_string(),
        mod_dir.to_string_lossy().to_string(),
        oversized,
    )
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Image too large"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn paste_thumbnail_restores_the_replaced_hidden_preview() {
    use crate::modules::settings::application::config::GameConfig;
    use crate::modules::workspace::application::scanner::core::thumbnail;

    let temp = TempDir::new().unwrap();
    let root = temp.path().join("Mods");
    let folder = root.join("Mod");
    fs::create_dir_all(&folder).unwrap();
    let pool = crate::test_utils::init_test_db().await.pool;
    let config = ConfigService::new_for_test_async(pool).await;
    let mut settings = config.get_settings();
    settings.games.push(GameConfig {
        id: "game-1".to_string(),
        name: "Test Game".to_string(),
        game_type: crate::modules::games::domain::models::GameType::Gimi,
        instance_path: temp.path().to_path_buf(),
        mod_path: root,
        ready_to_move_path: None,
        launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
        game_exe: Some(temp.path().join("game.exe")),
        loader_exe: None,
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    });
    config.save_settings(settings).unwrap();
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(1, 1)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let bytes = encoded.into_inner();
    let paste = || {
        paste_thumbnail_inner(
            &config,
            "game-1".to_string(),
            folder.to_string_lossy().to_string(),
            bytes.clone(),
        )
    };
    let saved = std::path::PathBuf::from(paste().await.unwrap());
    crate::platform::images::preview_visibility::set_hidden(&folder, &[saved], true).unwrap();
    assert!(thumbnail::find_thumbnail(&folder).is_none());
    paste().await.unwrap();
    assert_eq!(
        thumbnail::find_thumbnail(&folder),
        Some(folder.join("preview_custom.png"))
    );
}
