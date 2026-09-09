//! Live (unsaved) runtime rows shaped as collection members.

use sqlx::{Row, SqliteConnection, SqlitePool};

use crate::modules::collections::domain::collection::CollectionObject;
use crate::shared::errors::CollectionError;

/// All objects of a game shaped as live (unsaved) collection members.
pub async fn get_live_objects(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<CollectionObject>, CollectionError> {
    let rows = sqlx::query(
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
    .await?;
    rows.iter().map(map_live_object).collect()
}

pub async fn get_live_objects_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<Vec<CollectionObject>, CollectionError> {
    let rows = sqlx::query(
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
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(map_live_object).collect()
}

fn map_live_object(row: &sqlx::sqlite::SqliteRow) -> Result<CollectionObject, CollectionError> {
    Ok(CollectionObject {
        kind: crate::modules::collections::domain::collection::MemberKind::Object,
        collection_id: row.try_get("collection_id")?,
        object_id: row.try_get("object_id")?,
        is_enabled: row.try_get::<i64, _>("is_enabled")? != 0,
        display_name: row.try_get("display_name")?,
        path_key: row.try_get("path_key")?,
    })
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LiveActiveModRow {
    pub mod_id: String,
    pub mod_path: String,
    pub mod_path_key: String,
    pub object_id: String,
    pub display_name: String,
    pub is_safe: bool,
    pub safety_source: Option<String>,
}

pub struct RuntimeCollectionMembership {
    pub mod_path_keys: Vec<String>,
    pub object_states: Vec<(String, bool)>,
}

/// Enabled mods of a game. Safety classification never changes collection membership.
pub async fn get_live_active_mod_rows(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<LiveActiveModRow>, CollectionError> {
    let base = r#"
        SELECT
            id as mod_id,
            folder_path as mod_path,
            folder_path_key as mod_path_key,
            object_id,
            actual_name as display_name,
            COALESCE(is_safe, 1) as is_safe,
            safety_source
        FROM mods
        WHERE game_id = ? AND status = 1
    "#;
    Ok(sqlx::query_as(base).bind(game_id).fetch_all(pool).await?)
}

pub async fn get_live_active_mod_rows_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<Vec<LiveActiveModRow>, CollectionError> {
    let base = r#"
        SELECT
            id as mod_id,
            folder_path as mod_path,
            folder_path_key as mod_path_key,
            object_id,
            actual_name as display_name,
            COALESCE(is_safe, 1) as is_safe,
            safety_source
        FROM mods
        WHERE game_id = ? AND status = 1
    "#;
    Ok(sqlx::query_as(base)
        .bind(game_id)
        .fetch_all(&mut *conn)
        .await?)
}

/// The active baseline fields required to compare runtime status. This avoids
/// loading preview metadata, warnings, or tree data for a global status read.
pub async fn get_runtime_collection_membership_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<RuntimeCollectionMembership, CollectionError> {
    let mod_path_keys = sqlx::query_scalar(
        "SELECT mod_path_key FROM collection_mods WHERE collection_id = ? AND mod_path_key IS NOT NULL",
    )
    .bind(collection_id)
    .fetch_all(&mut *conn)
    .await?;
    let object_states = sqlx::query_as(
        "SELECT object_ref_key, is_enabled FROM collection_objects WHERE collection_id = ?",
    )
    .bind(collection_id)
    .fetch_all(&mut *conn)
    .await?;

    Ok(RuntimeCollectionMembership {
        mod_path_keys,
        object_states,
    })
}
