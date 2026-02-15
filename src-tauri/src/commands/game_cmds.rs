use crate::database::models::{GameConfig, GameType};
use crate::services::validator;
use serde_json::json;
use std::path::Path;
use tauri_plugin_store::StoreExt;
use uuid::Uuid;
use log;

/// Auto-detect games by scanning an XXMI root folder.
/// Validates each subfolder, saves valid games to config store + DB.
#[tauri::command]
pub async fn auto_detect_games(
    app: tauri::AppHandle,
    root_path: String,
) -> Result<Vec<GameConfig>, String> {
    let root = Path::new(&root_path);
    if !root.exists() {
        return Err(format!("Path does not exist: {root_path}"));
    }

    let found = validator::scan_xxmi_root(root);
    if found.is_empty() {
        return Err("No valid 3DMigoto instances found in standard XXMI folders.".to_string());
    }

    let store = app
        .store("config.json")
        .map_err(|e| format!("Failed to open config store: {e}"))?;

    let mut games: Vec<GameConfig> = Vec::new();

    for (info, game_type_str, display_name) in &found {
        let id = Uuid::new_v4().to_string();
        let game = GameConfig {
            id,
            name: display_name.to_string(),
            game_type: game_type_str.to_string(),
            path: info.path.clone(),
            mods_path: info.mods_path.clone(),
            launcher_path: info.launcher_path.clone(),
            launch_args: None,
        };
        games.push(game);
    }

    // Save to tauri-plugin-store
    let games_json: Vec<serde_json::Value> = games
        .iter()
        .filter_map(|g| serde_json::to_value(g).ok())
        .collect();
    store.set("games", json!(games_json));
    store
        .save()
        .map_err(|e| format!("Failed to save config: {e}"))?;

    log::info!("Auto-detect complete: found {} game(s)", games.len());

    Ok(games)
}

/// Manually add a single game by path and type.
/// Validates the folder, checks for duplicates, saves to config store.
#[tauri::command]
pub async fn add_game_manual(
    app: tauri::AppHandle,
    game_type: String,
    path: String,
) -> Result<GameConfig, String> {
    // Parse game type
    let gt: GameType = game_type.parse().map_err(|e: String| e)?;
    let folder = Path::new(&path);

    // Validate folder structure
    let info = validator::validate_instance(folder)?;

    // Check duplicates in store
    let store = app
        .store("config.json")
        .map_err(|e| format!("Failed to open config store: {e}"))?;

    let mut existing_games: Vec<GameConfig> = match store.get("games") {
        Some(val) => serde_json::from_value(val.clone()).unwrap_or_default(),
        None => Vec::new(),
    };

    // Duplicate path check (TC-1.5-01, NC-1.3-02)
    let normalized_path = info.path.replace('\\', "/").to_lowercase();
    for g in &existing_games {
        let existing_normalized = g.path.replace('\\', "/").to_lowercase();
        if existing_normalized == normalized_path {
            return Err(format!(
                "This game path is already registered as '{}'.",
                g.name
            ));
        }
    }

    let id = Uuid::new_v4().to_string();
    let game = GameConfig {
        id,
        name: gt.display_name().to_string(),
        game_type: gt.to_string(),
        path: info.path,
        mods_path: info.mods_path,
        launcher_path: info.launcher_path,
        launch_args: None,
    };

    existing_games.push(game.clone());

    // Save updated list
    let games_json: Vec<serde_json::Value> = existing_games
        .iter()
        .filter_map(|g| serde_json::to_value(g).ok())
        .collect();
    store.set("games", json!(games_json));
    store
        .save()
        .map_err(|e| format!("Failed to save config: {e}"))?;

    log::info!("Game added manually: {} ({})", game.name, game.game_type);

    Ok(game)
}

/// Get all configured games from the store.
#[tauri::command]
pub async fn get_games(app: tauri::AppHandle) -> Result<Vec<GameConfig>, String> {
    let store = app
        .store("config.json")
        .map_err(|e| format!("Failed to open config store: {e}"))?;

    match store.get("games") {
        Some(val) => {
            let games: Vec<GameConfig> = serde_json::from_value(val.clone()).unwrap_or_default();
            Ok(games)
        }
        None => Ok(Vec::new()),
    }
}
