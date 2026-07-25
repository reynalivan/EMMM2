//! Writes driven by scanner sync, runtime toggles, and object (re)linking.

use super::types::SyncModRowUpdate;
use crate::common::corridor_constants::DISABLED_REASON_USER;
use crate::common::path_key::folder_path_key;
use crate::domain::models::ItemStatus;
use sqlx::SqlitePool;

pub async fn update_mod_sync_row(
    conn: &mut sqlx::SqliteConnection,
    update: SyncModRowUpdate<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE mods
         SET id = ?, folder_path = ?, folder_path_key = ?, actual_name = ?, status = ?, is_safe = ?, corridor_source = ?, disabled_reason = ?, object_id = ?, object_type = ?
         WHERE folder_path_key = ? AND game_id = ?",
    )
    .bind(update.new_id)
    .bind(update.folder_path)
    .bind(folder_path_key(update.folder_path, Some(update.mods_path)))
    .bind(update.actual_name)
    .bind(update.status)
    .bind(update.is_safe)
    .bind(update.corridor_source)
    .bind(update.disabled_reason)
    .bind(update.object_id)
    .bind(update.object_type)
    .bind(folder_path_key(update.old_folder_path, Some(update.mods_path)))
    .bind(update.game_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Sync one mod row after a runtime enable/disable rename. Returns rows affected.
pub async fn update_mod_runtime_toggle(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mod_id: &str,
    new_rel: &str,
    mods_path: &str,
    enabled: bool,
    disabled_reason: Option<&str>,
) -> Result<u64, sqlx::Error> {
    let status = if enabled { 1i32 } else { 0i32 };
    let result = sqlx::query(
        r#"
        UPDATE mods
        SET folder_path = ?, folder_path_key = ?, status = ?, disabled_reason = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ? AND game_id = ?
        "#,
    )
    .bind(new_rel)
    .bind(folder_path_key(new_rel, Some(mods_path)))
    .bind(status)
    .bind(disabled_reason)
    .bind(mod_id)
    .bind(game_id)
    .execute(&mut *conn)
    .await?;
    Ok(result.rows_affected())
}

#[allow(clippy::too_many_arguments)] // Transactional identity update mirrors the mod row fields being rewritten.
pub async fn update_mod_identity_tx(
    conn: &mut sqlx::SqliteConnection,
    new_id: &str,
    new_folder_path: &str,
    new_actual_name: &str,
    new_status: ItemStatus,
    new_is_safe: bool,
    corridor_source: &str,
    old_id: &str,
    mods_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    let disabled_reason = if new_status.is_enabled() {
        None
    } else {
        Some(DISABLED_REASON_USER)
    };
    sqlx::query(
        "UPDATE mods
         SET id = ?, folder_path = ?, folder_path_key = ?, actual_name = ?, status = ?, is_safe = ?, corridor_source = ?, disabled_reason = ?
         WHERE id = ?",
    )
        .bind(new_id)
        .bind(new_folder_path)
        .bind(folder_path_key(new_folder_path, mods_path))
        .bind(new_actual_name)
        .bind(new_status as i64)
        .bind(new_is_safe)
        .bind(corridor_source)
        .bind(disabled_reason)
        .bind(old_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn set_mod_object<'c, E>(
    executor: E,
    mod_id: &str,
    object_id: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query("UPDATE mods SET object_id = ? WHERE id = ?")
        .bind(object_id)
        .bind(mod_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn set_object_type_for_object(
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
    object_type: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE mods SET object_type = ? WHERE game_id = ? AND object_id = ?")
        .bind(object_type)
        .bind(game_id)
        .bind(object_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn update_mod_object_id_and_type_tx(
    conn: &mut sqlx::SqliteConnection,
    id: &str,
    object_id: &str,
    object_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET object_id = ?, object_type = ? WHERE id = ?")
        .bind(object_id)
        .bind(object_type)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
