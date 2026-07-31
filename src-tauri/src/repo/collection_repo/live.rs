//! Live (unsaved) runtime rows shaped as collection members.

use sqlx::SqlitePool;

use crate::domain::collection::CollectionObject;
use crate::domain::errors::CollectionError;

/// All objects of a game shaped as live (unsaved) collection members.
pub async fn get_live_objects(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<CollectionObject>, CollectionError> {
    Ok(sqlx::query_as(
        r#"
        SELECT
            'object' as kind,
            '' as collection_id,
            id as object_id,
            1 as is_enabled,
            name as display_name,
            folder_path as path_key
        FROM objects
        WHERE game_id = ?
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LiveActiveModRow {
    pub mod_id: String,
    pub mod_path: String,
    pub mod_path_key: String,
    pub object_id: String,
    pub display_name: String,
}

/// Enabled mods of a game, optionally restricted to one corridor.
pub async fn get_live_active_mod_rows(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: Option<bool>,
) -> Result<Vec<LiveActiveModRow>, CollectionError> {
    let base = r#"
        SELECT
            id as mod_id,
            folder_path as mod_path,
            folder_path_key as mod_path_key,
            object_id,
            actual_name as display_name
        FROM mods
        WHERE game_id = ? AND status = 1
    "#;
    let rows = if let Some(is_safe) = is_safe {
        sqlx::query_as(&format!("{base} AND is_safe = ?"))
            .bind(game_id)
            .bind(is_safe)
            .fetch_all(pool)
            .await?
    } else {
        sqlx::query_as(base).bind(game_id).fetch_all(pool).await?
    };
    Ok(rows)
}
