//! Writes driven by scanner sync, runtime toggles, and object (re)linking.

use super::types::SyncModRowUpdate;
use crate::common::path_key::folder_path_key;
use crate::domain::models::ItemStatus;

pub async fn update_mod_sync_row(
    conn: &mut sqlx::SqliteConnection,
    update: SyncModRowUpdate<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE mods
         SET id = ?, folder_path = ?, folder_path_key = ?, actual_name = ?, status = ?, is_safe = ?, safety_source = ?, object_id = ?, object_type = ?
         WHERE folder_path_key = ? AND game_id = ?",
    )
    .bind(update.new_id)
    .bind(update.folder_path)
    .bind(folder_path_key(update.folder_path, Some(update.mods_path)))
    .bind(update.actual_name)
    .bind(update.status)
    .bind(update.is_safe)
    .bind(update.safety_source)
    .bind(update.object_id)
    .bind(update.object_type)
    .bind(folder_path_key(update.old_folder_path, Some(update.mods_path)))
    .bind(update.game_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Transactional identity update mirrors the mod row fields being rewritten.
pub async fn update_mod_identity_tx(
    conn: &mut sqlx::SqliteConnection,
    new_id: &str,
    new_folder_path: &str,
    new_actual_name: &str,
    new_status: ItemStatus,
    new_is_safe: bool,
    safety_source: &str,
    old_id: &str,
    mods_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE mods
         SET id = ?, folder_path = ?, folder_path_key = ?, actual_name = ?, status = ?, is_safe = ?, safety_source = ?
         WHERE id = ?",
    )
    .bind(new_id)
    .bind(new_folder_path)
    .bind(folder_path_key(new_folder_path, mods_path))
    .bind(new_actual_name)
    .bind(new_status as i64)
    .bind(new_is_safe)
    .bind(safety_source)
    .bind(old_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn defer_foreign_keys_tx(conn: &mut sqlx::SqliteConnection) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA defer_foreign_keys = ON")
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn stage_mod_identity_tx(
    conn: &mut sqlx::SqliteConnection,
    temp_id: &str,
    temp_path: &str,
    old_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mods SET id = ?, folder_path = ?, folder_path_key = ? WHERE id = ?")
        .bind(temp_id)
        .bind(temp_path)
        .bind(folder_path_key(temp_path, None))
        .bind(old_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn rewrite_dependent_mod_ids_tx(
    conn: &mut sqlx::SqliteConnection,
    old_id: &str,
    new_id: &str,
) -> Result<(), sqlx::Error> {
    for (table, column) in [
        ("mod_hash_index", "mod_id"),
        ("dedup_group_members", "folder_id"),
        ("duplicate_whitelist", "folder_a_id"),
        ("duplicate_whitelist", "folder_b_id"),
    ] {
        let sql = format!("UPDATE {table} SET {column} = ? WHERE {column} = ?");
        sqlx::query(&sql)
            .bind(new_id)
            .bind(old_id)
            .execute(&mut *conn)
            .await?;
    }
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

pub async fn set_object_type_for_object<'c, E>(
    executor: E,
    game_id: &str,
    object_id: &str,
    object_type: &str,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    let result = sqlx::query("UPDATE mods SET object_type = ? WHERE game_id = ? AND object_id = ?")
        .bind(object_type)
        .bind(game_id)
        .bind(object_id)
        .execute(executor)
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
