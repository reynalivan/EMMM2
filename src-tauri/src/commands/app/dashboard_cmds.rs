use crate::database::dashboard_repo;
use crate::services::ini::document as ini_document;
use serde::Serialize;
use std::path::Path;

/// Combined dashboard payload returned to the frontend in a single IPC call.
#[derive(Debug, Clone, Serialize)]
pub struct DashboardPayload {
    pub stats: dashboard_repo::DashboardStats,
    pub duplicate_waste_bytes: i64,
    pub category_distribution: Vec<dashboard_repo::CategorySlice>,
    pub game_distribution: Vec<dashboard_repo::GameSlice>,
    pub recent_mods: Vec<dashboard_repo::RecentMod>,
}

/// A keybinding entry extracted from an enabled mod's INI file.
#[derive(Debug, Clone, Serialize)]
pub struct ActiveKeyBinding {
    pub mod_name: String,
    pub section_name: String,
    pub key: Option<String>,
    pub back: Option<String>,
}

/// Fetch all dashboard data in a single command for minimal IPC overhead.
///
/// `safe_mode`: when true, dashboard stats/charts exclude mods with `is_safe = 0`.
#[tauri::command]
pub async fn get_dashboard_stats(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    safe_mode: bool,
) -> Result<DashboardPayload, String> {
    let db = pool.inner();

    let stats = dashboard_repo::fetch_global_stats(db, safe_mode)
        .await
        .map_err(|e| format!("Dashboard stats error: {e}"))?;

    let duplicate_waste_bytes = dashboard_repo::fetch_duplicate_waste(db)
        .await
        .map_err(|e| format!("Duplicate waste error: {e}"))?;

    let category_distribution = dashboard_repo::fetch_category_distribution(db, safe_mode)
        .await
        .map_err(|e| format!("Category dist error: {e}"))?;

    let game_distribution = dashboard_repo::fetch_game_distribution(db, safe_mode)
        .await
        .map_err(|e| format!("Game dist error: {e}"))?;

    let recent_mods = dashboard_repo::fetch_recent_mods(db, safe_mode, 5)
        .await
        .map_err(|e| format!("Recent mods error: {e}"))?;

    Ok(DashboardPayload {
        stats,
        duplicate_waste_bytes,
        category_distribution,
        game_distribution,
        recent_mods,
    })
}

/// Scan all enabled mods for a game and return their keybindings.
///
/// This is a filesystem-heavy operation (reads INI files from disk),
/// so it's a separate command from the main dashboard payload.
#[tauri::command]
pub async fn get_active_keybindings(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<ActiveKeyBinding>, String> {
    // 1. Fetch enabled mods' folder paths and names for this game
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT actual_name, folder_path FROM mods WHERE game_id = ? AND status = 'ENABLED'",
    )
    .bind(&game_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("Failed to query enabled mods: {e}"))?;

    let mut bindings: Vec<ActiveKeyBinding> = Vec::new();

    // 2. For each enabled mod, scan its INI files for keybindings
    for (mod_name, folder_path) in &rows {
        let mod_path = Path::new(folder_path);
        if !mod_path.is_dir() {
            continue;
        }

        let ini_files = match ini_document::list_ini_files(mod_path) {
            Ok(files) => files,
            Err(_) => continue,
        };

        for ini_path in ini_files {
            let doc = match ini_document::read_ini_document(&ini_path) {
                Ok(d) => d,
                Err(_) => continue,
            };

            for kb in &doc.key_bindings {
                if kb.key.is_some() || kb.back.is_some() {
                    bindings.push(ActiveKeyBinding {
                        mod_name: mod_name.clone(),
                        section_name: kb.section_name.clone(),
                        key: kb.key.clone(),
                        back: kb.back.clone(),
                    });
                }
            }
        }
    }

    Ok(bindings)
}
