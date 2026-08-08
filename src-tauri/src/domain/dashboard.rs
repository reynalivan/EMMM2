//! Dashboard read-model shapes.
//!
//! Query results rather than table rows — the field names are SQL aliases,
//! not columns — but they cross IPC, so they belong here rather than in the
//! repo that computes them.

use serde::Serialize;

/// Global overview statistics for the dashboard tiles.
#[derive(Debug, Clone, Serialize, serde::Deserialize, sqlx::FromRow, specta::Type)]
pub struct DashboardStats {
    #[specta(type = f64)]
    pub total_mods: i64,
    #[specta(type = f64)]
    pub enabled_mods: i64,
    #[specta(type = f64)]
    pub disabled_mods: i64,
    #[specta(type = f64)]
    pub total_size_bytes: i64,
    #[specta(type = f64)]
    pub total_games: i64,
    #[specta(type = f64)]
    pub total_collections: i64,
}

/// A single slice of the category distribution pie chart.
#[derive(Debug, Clone, Serialize, serde::Deserialize, sqlx::FromRow, specta::Type)]
pub struct CategorySlice {
    pub category: String,
    #[specta(type = f64)]
    pub count: i64,
}

/// A single bar of the game distribution bar chart.
#[derive(Debug, Clone, Serialize, serde::Deserialize, sqlx::FromRow, specta::Type)]
pub struct GameSlice {
    pub game_id: String,
    pub game_name: String,
    #[specta(type = f64)]
    pub count: i64,
}

/// A recently indexed mod for the activity widget.
#[derive(Debug, Clone, Serialize, serde::Deserialize, sqlx::FromRow, specta::Type)]
pub struct RecentMod {
    pub id: String,
    pub name: String,
    pub game_name: String,
    pub object_name: Option<String>,
    pub indexed_at: Option<String>,
}
