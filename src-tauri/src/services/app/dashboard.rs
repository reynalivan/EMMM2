use crate::services::keyviewer::harvester;

use crate::domain::errors::AppError;

/// A keybinding entry extracted from an enabled mod's INI file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct ActiveKeyBinding {
    pub mod_name: String,
    pub section_name: String,
    pub key: Option<String>,
    pub back: Option<String>,
}

pub async fn get_active_keybindings_service(
    pool: &sqlx::SqlitePool,
    game_id: &str,
) -> Result<Vec<ActiveKeyBinding>, AppError> {
    let mods_root = crate::repo::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} has no mods path")))?;
    let mods_root = std::path::Path::new(&mods_root);
    // 1. Fetch enabled mods' folder paths and names for this game
    let rows = crate::repo::mods::get_enabled_mods_names_and_paths(pool, game_id).await?;

    let mut bindings: Vec<ActiveKeyBinding> = Vec::new();

    // 2. For each enabled mod, scan its INI files for keybindings
    for (mod_name, folder_path) in &rows {
        // `harvest_keybinds_from_mod` owns the list-then-parse walk and its
        // skip-on-error policy; this used to be a second copy of both.
        let Ok(keybinds) = harvester::harvest_keybinds_from_mod(&folder_path.resolve(mods_root))
        else {
            continue;
        };

        let named = keybinds
            .into_iter()
            .filter(|kb| kb.key.is_some() || kb.back.is_some())
            .map(|kb| ActiveKeyBinding {
                mod_name: mod_name.clone(),
                section_name: kb.section_name,
                key: kb.key,
                back: kb.back,
            });
        bindings.extend(named);
    }

    Ok(bindings)
}

/// Full dashboard payload struct (mirrors the command type).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct DashboardPayload {
    pub stats: crate::domain::dashboard::DashboardStats,
    #[specta(type = f64)]
    pub duplicate_waste_bytes: i64,
    pub category_distribution: Vec<crate::domain::dashboard::CategorySlice>,
    pub game_distribution: Vec<crate::domain::dashboard::GameSlice>,
    pub recent_mods: Vec<crate::domain::dashboard::RecentMod>,
}

/// Fetch all dashboard data in a single service call.
pub async fn get_dashboard_payload(pool: &sqlx::SqlitePool) -> Result<DashboardPayload, AppError> {
    use crate::repo::dashboard;

    let stats = dashboard::fetch_global_stats(pool).await?;

    // Independent reads. Serially they cost four extra round trips; WAL
    // readers do not block each other, so the pool can serve them at once.
    let (duplicate_waste_bytes, category_distribution, game_distribution, recent_mods) = tokio::try_join!(
        async { dashboard::fetch_duplicate_waste(pool).await },
        async { dashboard::fetch_category_distribution(pool).await },
        async { dashboard::fetch_game_distribution(pool).await },
        async { dashboard::fetch_recent_mods(pool, 5).await },
    )?;

    Ok(DashboardPayload {
        stats,
        duplicate_waste_bytes,
        category_distribution,
        game_distribution,
        recent_mods,
    })
}
