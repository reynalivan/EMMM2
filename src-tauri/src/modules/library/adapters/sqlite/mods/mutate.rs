//! Row lifecycle: inserts and deletes.

use crate::modules::games::domain::models::ItemStatus;
use sqlx::SqlitePool;

// Eleven bound fields per row stay below even SQLite's historical 999 limit.
pub(crate) const RECONCILE_INSERT_BATCH_SIZE: usize = 64;

pub(crate) struct ReconcileModInsert<'a> {
    pub id: String,
    pub object_id: &'a str,
    pub actual_name: String,
    pub folder_path: &'a str,
    pub folder_path_key: String,
    pub status: ItemStatus,
    pub object_type: &'a str,
    pub is_safe: bool,
    pub safety_source: &'static str,
    pub filesystem_identity: Option<&'a str>,
}

pub(crate) async fn insert_reconcile_mods_tx(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods: &[ReconcileModInsert<'_>],
) -> Result<(), sqlx::Error> {
    for chunk in mods.chunks(RECONCILE_INSERT_BATCH_SIZE) {
        let mut query = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            "INSERT INTO mods (id,game_id,object_id,actual_name,folder_path,folder_path_key,status,object_type,is_favorite,is_safe,safety_source,size_bytes,filesystem_identity) "
        );
        query.push_values(chunk, |mut row, entry| {
            row.push_bind(&entry.id)
                .push_bind(game_id)
                .push_bind(entry.object_id)
                .push_bind(&entry.actual_name)
                .push_bind(entry.folder_path)
                .push_bind(&entry.folder_path_key)
                .push_bind(entry.status as i64)
                .push_bind(entry.object_type)
                .push("0")
                .push_bind(entry.is_safe)
                .push_bind(entry.safety_source)
                .push("0")
                .push_bind(entry.filesystem_identity);
        });
        query.build().execute(&mut *conn).await?;
    }
    Ok(())
}

pub async fn delete_mod_by_id(pool: &SqlitePool, mod_id: &str) -> Result<(), sqlx::Error> {
    let mut conn = pool.acquire().await?;
    delete_mod_tx(&mut conn, mod_id).await
}

pub async fn delete_mod_tx(conn: &mut sqlx::SqliteConnection, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mods WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
