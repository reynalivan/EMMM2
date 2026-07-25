//! Snapshot of the DB rows a projection write compares against, indexed by
//! every lookup key the passes need.

use std::collections::HashMap;

use crate::repo::mod_repo::ReconcileModRow as DbModRow;
use crate::repo::object_repo::ReconcileObjectRow as DbObjectRow;

use super::keys::runtime_logical_path_key;

async fn load_db_objects(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<DbObjectRow>, String> {
    crate::repo::object_repo::get_rows_for_reconcile(conn, game_id)
        .await
        .map_err(|error| error.to_string())
}

async fn load_db_mods(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<DbModRow>, String> {
    crate::repo::mod_repo::get_rows_for_reconcile(conn, game_id)
        .await
        .map_err(|error| error.to_string())
}

pub(super) struct DbIndex {
    pub(super) objects: Vec<DbObjectRow>,
    pub(super) objects_by_key: HashMap<String, DbObjectRow>,
    pub(super) objects_by_runtime_key: HashMap<String, DbObjectRow>,
    pub(super) objects_by_id: HashMap<String, DbObjectRow>,
    pub(super) mods: Vec<DbModRow>,
    pub(super) mods_by_key: HashMap<String, DbModRow>,
    pub(super) mods_by_path_lower: HashMap<String, DbModRow>,
    pub(super) mods_by_runtime_key: HashMap<String, DbModRow>,
}

impl DbIndex {
    pub(super) async fn load(
        conn: &mut sqlx::SqliteConnection,
        game_id: &str,
    ) -> Result<Self, String> {
        let db_objects = load_db_objects(&mut *conn, game_id).await?;
        let db_mods = load_db_mods(&mut *conn, game_id).await?;
        let db_objects_by_key = db_objects
            .iter()
            .cloned()
            .map(|row| (row.folder_path_key.clone(), row))
            .collect::<HashMap<_, _>>();
        let mut db_objects_by_runtime_key = HashMap::new();
        for row in &db_objects {
            db_objects_by_runtime_key
                .entry(runtime_logical_path_key(&row.folder_path))
                .or_insert_with(|| row.clone());
        }
        let db_objects_by_id = db_objects
            .iter()
            .cloned()
            .map(|row| (row.id.clone(), row))
            .collect::<HashMap<_, _>>();
        let db_mods_by_key = db_mods
            .iter()
            .cloned()
            .map(|row| (row.folder_path_key.clone(), row))
            .collect::<HashMap<_, _>>();
        let db_mods_by_path_lower = db_mods
            .iter()
            .cloned()
            .map(|row| (row.folder_path.to_ascii_lowercase(), row))
            .collect::<HashMap<_, _>>();
        let mut db_mods_by_runtime_key = HashMap::new();
        for row in &db_mods {
            db_mods_by_runtime_key
                .entry(runtime_logical_path_key(&row.folder_path))
                .or_insert_with(|| row.clone());
        }

        Ok(Self {
            objects: db_objects,
            objects_by_key: db_objects_by_key,
            objects_by_runtime_key: db_objects_by_runtime_key,
            objects_by_id: db_objects_by_id,
            mods: db_mods,
            mods_by_key: db_mods_by_key,
            mods_by_path_lower: db_mods_by_path_lower,
            mods_by_runtime_key: db_mods_by_runtime_key,
        })
    }
}
