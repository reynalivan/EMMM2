use crate::services::ini::document as ini_document;
use serde::Serialize;
use std::path::Path;

/// A keybinding entry extracted from an enabled mod's INI file.
#[derive(Debug, Clone, Serialize)]
pub struct ActiveKeyBinding {
    pub mod_name: String,
    pub section_name: String,
    pub key: Option<String>,
    pub back: Option<String>,
}

pub async fn get_active_keybindings_service(
    pool: &sqlx::SqlitePool,
    game_id: &str,
) -> Result<Vec<ActiveKeyBinding>, String> {
    // 1. Fetch enabled mods' folder paths and names for this game
    let rows = crate::database::mod_repo::get_enabled_mods_names_and_paths(pool, game_id)
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

/// Full dashboard payload struct (mirrors the command type).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DashboardPayload {
    pub stats: crate::database::dashboard_repo::DashboardStats,
    pub duplicate_waste_bytes: i64,
    pub category_distribution: Vec<crate::database::dashboard_repo::CategorySlice>,
    pub game_distribution: Vec<crate::database::dashboard_repo::GameSlice>,
    pub recent_mods: Vec<crate::database::dashboard_repo::RecentMod>,
}

/// Fetch all dashboard data in a single service call.
/// `safe_mode`: when true, stats/charts exclude mods with `is_safe = 0`.
pub async fn get_dashboard_payload(
    pool: &sqlx::SqlitePool,
    safe_mode: bool,
) -> Result<DashboardPayload, String> {
    use crate::database::dashboard_repo;

    let stats = dashboard_repo::fetch_global_stats(pool, safe_mode)
        .await
        .map_err(|e| format!("Dashboard stats error: {e}"))?;

    let duplicate_waste_bytes = dashboard_repo::fetch_duplicate_waste(pool)
        .await
        .map_err(|e| format!("Duplicate waste error: {e}"))?;

    let category_distribution = dashboard_repo::fetch_category_distribution(pool, safe_mode)
        .await
        .map_err(|e| format!("Category dist error: {e}"))?;

    let game_distribution = dashboard_repo::fetch_game_distribution(pool, safe_mode)
        .await
        .map_err(|e| format!("Game dist error: {e}"))?;

    let recent_mods = dashboard_repo::fetch_recent_mods(pool, safe_mode, 5)
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
