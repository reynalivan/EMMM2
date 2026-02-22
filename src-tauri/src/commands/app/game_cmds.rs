use crate::database::models::{GameConfig, GameType};
use crate::database::settings_repo;
use crate::services::game::validator;
use std::path::Path;
use uuid::Uuid;

/// Auto-detect games by scanning an XXMI root folder.
/// Validates each subfolder, saves valid games to SQLite DB.
#[tauri::command]
pub async fn auto_detect_games(
    pool: tauri::State<'_, sqlx::SqlitePool>,
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

        // Save to SQLite
        let row = settings_repo::GameRow {
            id: game.id.clone(),
            name: game.name.clone(),
            game_type: game.game_type.clone(),
            path: game.path.clone(),
            mod_path: Some(game.mods_path.clone()),
            game_exe: Some(game.path.clone()),
            launcher_path: Some(game.launcher_path.clone()),
            loader_exe: Some(game.launcher_path.clone()),
            launch_args: game.launch_args.clone(),
        };
        settings_repo::upsert_game(pool.inner(), &row)
            .await
            .map_err(|e| format!("Failed to save game to DB: {e}"))?;

        games.push(game);
    }

    log::info!("Auto-detect complete: found {} game(s)", games.len());

    Ok(games)
}

/// Manually add a single game by path and type.
/// Validates the folder, checks for duplicates, saves to SQLite DB.
#[tauri::command]
pub async fn add_game_manual(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_type: String,
    path: String,
) -> Result<GameConfig, String> {
    // Parse game type
    let gt: GameType = game_type.parse().map_err(|e: String| e)?;
    let folder = Path::new(&path);

    // Validate folder structure
    let info = validator::validate_instance(folder)?;

    // Check duplicates in DB
    let existing_games = settings_repo::get_all_games(pool.inner())
        .await
        .map_err(|e| format!("Failed to read games from DB: {e}"))?;

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

    // Save to SQLite
    let row = settings_repo::GameRow {
        id: game.id.clone(),
        name: game.name.clone(),
        game_type: game.game_type.clone(),
        path: game.path.clone(),
        mod_path: Some(game.mods_path.clone()),
        game_exe: Some(game.path.clone()),
        launcher_path: Some(game.launcher_path.clone()),
        loader_exe: Some(game.launcher_path.clone()),
        launch_args: game.launch_args.clone(),
    };
    settings_repo::upsert_game(pool.inner(), &row)
        .await
        .map_err(|e| format!("Failed to save game to DB: {e}"))?;

    log::info!("Game added manually: {} ({})", game.name, game.game_type);

    Ok(game)
}

/// Get all configured games from SQLite DB.
#[tauri::command]
pub async fn get_games(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<Vec<GameConfig>, String> {
    let rows = settings_repo::get_all_games(pool.inner())
        .await
        .map_err(|e| format!("Failed to read games from DB: {e}"))?;

    let games: Vec<GameConfig> = rows
        .into_iter()
        .map(|row| GameConfig {
            id: row.id,
            name: row.name,
            game_type: row.game_type,
            path: row.path.clone(),
            mods_path: row.mod_path.unwrap_or_else(|| row.path.clone()),
            launcher_path: row.launcher_path.unwrap_or_default(),
            launch_args: row.launch_args,
        })
        .collect();

    Ok(games)
}

/// Launch the 3DMigoto Loader (if not running) and then the Game.
/// Covers: US-10.1, TC-10.1-01
#[tauri::command]
pub async fn launch_game(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<(), String> {
    use sysinfo::System;

    // 1. Get Game Config
    let games = get_games(pool).await?;
    let game = games
        .into_iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| "Game config not found".to_string())?;

    // 2. Check if Loader is valid
    let launcher_path = Path::new(&game.launcher_path);
    if !launcher_path.exists() {
        return Err(format!("Launcher not found at: {}", game.launcher_path));
    }

    let game_path = Path::new(&game.path);
    if !game_path.exists() {
        return Err(format!("Game executable not found at: {}", game.path));
    }

    // 3. Process Check (sysinfo)
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let launcher_name = launcher_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    // exact name match might be tricky with extension, usually contains .exe
    let is_loader_running = sys.processes().values().any(|p| {
        p.name()
            .to_string_lossy()
            .eq_ignore_ascii_case(&launcher_name)
    });

    // 4. Launch Loader if needed
    if !is_loader_running {
        log::info!("Starting Loader: {}", game.launcher_path);

        // Use directory of the launcher as CWD
        let launcher_dir = launcher_path.parent().unwrap_or(launcher_path);

        std::process::Command::new(&game.launcher_path)
            .current_dir(launcher_dir)
            .spawn()
            .map_err(|e| format!("Failed to start loader: {e}"))?;

        // Small delay to let loader initialize
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    } else {
        log::info!("Loader already running: {}", launcher_name);
    }

    // 5. Launch Game
    log::info!("Starting Game: {}", game.path);
    let game_dir = game_path.parent().unwrap_or(game_path);

    let mut cmd = std::process::Command::new(&game.path);
    cmd.current_dir(game_dir);

    // Apply args
    if let Some(args_str) = game.launch_args {
        if !args_str.trim().is_empty() {
            for arg in args_str.split_whitespace() {
                cmd.arg(arg);
            }
        }
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to start game: {e}"))?;

    Ok(())
}
