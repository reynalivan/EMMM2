//! Snapshot of the DB rows a projection write compares against, indexed by
//! every lookup key the passes need.

use crate::domain::errors::AppError;
use std::collections::HashMap;

use crate::repo::mod_repo::ReconcileModRow as DbModRow;
use crate::repo::object_repo::ReconcileObjectRow as DbObjectRow;

use super::keys::runtime_logical_path_key;

async fn load_db_objects(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<DbObjectRow>, AppError> {
    Ok(crate::repo::object_repo::get_rows_for_reconcile(conn, game_id).await?)
}

async fn load_db_mods(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<Vec<DbModRow>, AppError> {
    Ok(crate::repo::mod_repo::get_rows_for_reconcile(conn, game_id).await?)
}

/// Every lookup table stores a position into the owned row vector, so a row is
/// held once regardless of how many keys resolve to it.
pub(super) struct DbIndex {
    pub(super) objects: Vec<DbObjectRow>,
    objects_by_key: HashMap<String, usize>,
    objects_by_runtime_key: HashMap<String, usize>,
    objects_by_id: HashMap<String, usize>,
    pub(super) mods: Vec<DbModRow>,
    mods_by_key: HashMap<String, usize>,
    mods_by_path_lower: HashMap<String, usize>,
    mods_by_runtime_key: HashMap<String, usize>,
}

impl DbIndex {
    pub(super) async fn load(
        conn: &mut sqlx::SqliteConnection,
        game_id: &str,
    ) -> Result<Self, AppError> {
        let objects = load_db_objects(&mut *conn, game_id).await?;
        let mods = load_db_mods(&mut *conn, game_id).await?;

        let mut objects_by_key = HashMap::with_capacity(objects.len());
        let mut objects_by_runtime_key = HashMap::with_capacity(objects.len());
        let mut objects_by_id = HashMap::with_capacity(objects.len());
        for (position, row) in objects.iter().enumerate() {
            objects_by_key.insert(row.folder_path_key.clone(), position);
            objects_by_runtime_key
                .entry(runtime_logical_path_key(&row.folder_path))
                .or_insert(position);
            objects_by_id.insert(row.id.clone(), position);
        }

        let mut mods_by_key = HashMap::with_capacity(mods.len());
        let mut mods_by_path_lower = HashMap::with_capacity(mods.len());
        let mut mods_by_runtime_key = HashMap::with_capacity(mods.len());
        for (position, row) in mods.iter().enumerate() {
            mods_by_key.insert(row.folder_path_key.clone(), position);
            mods_by_path_lower.insert(row.folder_path.to_ascii_lowercase(), position);
            mods_by_runtime_key
                .entry(runtime_logical_path_key(&row.folder_path))
                .or_insert(position);
        }

        Ok(Self {
            objects,
            objects_by_key,
            objects_by_runtime_key,
            objects_by_id,
            mods,
            mods_by_key,
            mods_by_path_lower,
            mods_by_runtime_key,
        })
    }

    pub(super) fn object_by_key(&self, folder_path_key: &str) -> Option<&DbObjectRow> {
        self.objects_by_key
            .get(folder_path_key)
            .map(|&position| &self.objects[position])
    }

    pub(super) fn object_by_runtime_key(&self, runtime_key: &str) -> Option<&DbObjectRow> {
        self.objects_by_runtime_key
            .get(runtime_key)
            .map(|&position| &self.objects[position])
    }

    pub(super) fn object_by_id(&self, id: &str) -> Option<&DbObjectRow> {
        self.objects_by_id
            .get(id)
            .map(|&position| &self.objects[position])
    }

    pub(super) fn mod_by_key(&self, folder_path_key: &str) -> Option<&DbModRow> {
        self.mods_by_key
            .get(folder_path_key)
            .map(|&position| &self.mods[position])
    }

    pub(super) fn mod_by_path_lower(&self, folder_path_lower: &str) -> Option<&DbModRow> {
        self.mods_by_path_lower
            .get(folder_path_lower)
            .map(|&position| &self.mods[position])
    }

    pub(super) fn mod_by_runtime_key(&self, runtime_key: &str) -> Option<&DbModRow> {
        self.mods_by_runtime_key
            .get(runtime_key)
            .map(|&position| &self.mods[position])
    }
}
