//! Whole-snapshot writes: replacing every member table in one transaction and
//! updating the cached signature / display counters.

use sqlx::{SqliteConnection, SqlitePool};

use super::mapping::serialize_warnings_json;
use crate::domain::collection::{CollectionMod, CollectionObject, CollectionRoot};
use crate::domain::errors::CollectionError;

/// Replace all members of a collection. Callers wrap this in their own transaction.
#[allow(clippy::too_many_arguments)] // Snapshot replacement keeps collection member groups explicit at the repo boundary.
pub async fn replace_all_state_tx(
    conn: &mut SqliteConnection,
    id: &str,
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    roots: &[CollectionRoot],
    signature: Option<&str>,
    snapshot_json: Option<&str>,
    display_mod_count: i32,
) -> Result<(), CollectionError> {
    // Clear existing
    sqlx::query("DELETE FROM collection_mods WHERE collection_id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM collection_objects WHERE collection_id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM collection_roots WHERE collection_id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;

    // Insert Mods
    if !mods.is_empty() {
        let mut qb = sqlx::QueryBuilder::new(
            "INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_id, preview_path, node_type, warnings_json) ",
        );
        qb.push_values(mods, |mut b, m| {
            b.push_bind(&m.collection_id)
                .push_bind(&m.mod_id)
                .push_bind(&m.mod_path)
                .push_bind(&m.mod_path_key)
                .push_bind(&m.object_id)
                .push_bind(&m.preview_path)
                .push_bind(&m.node_type)
                .push_bind(serialize_warnings_json(&m.warnings));
        });
        qb.build().execute(&mut *conn).await?;
    }

    // Insert Objects
    if !objects.is_empty() {
        let mut qb = sqlx::QueryBuilder::new(
            "INSERT INTO collection_objects (collection_id, object_id, is_enabled) ",
        );
        qb.push_values(objects, |mut b, o| {
            b.push_bind(&o.collection_id)
                .push_bind(&o.object_id)
                .push_bind(if o.is_enabled { 1i32 } else { 0i32 });
        });
        qb.build().execute(&mut *conn).await?;
    }

    // Insert Roots
    if !roots.is_empty() {
        let mut qb = sqlx::QueryBuilder::new("INSERT INTO collection_roots (collection_id, root_path, root_path_key, display_name, display_name_key, object_id, object_name, object_type, root_kind, is_safe, is_enabled, thumbnail_hint, corridor_source) ");
        qb.push_values(roots, |mut b, r| {
            b.push_bind(&r.collection_id)
                .push_bind(&r.root_path)
                .push_bind(&r.root_path_key)
                .push_bind(&r.display_name)
                .push_bind(&r.display_name_key)
                .push_bind(&r.object_id)
                .push_bind(&r.object_name)
                .push_bind(&r.object_type)
                .push_bind(&r.root_kind)
                .push_bind(if r.is_safe { 1i32 } else { 0i32 })
                .push_bind(if r.is_enabled { 1i32 } else { 0i32 })
                .push_bind(&r.thumbnail_hint)
                .push_bind(&r.corridor_source);
        });
        qb.build().execute(&mut *conn).await?;
    }

    // Update stats
    sqlx::query("UPDATE collections SET signature = ?, snapshot_json = ?, root_count = ?, display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(signature)
        .bind(snapshot_json)
        .bind(roots.len() as i32)
        .bind(display_mod_count)
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn update_display_counts(
    pool: &SqlitePool,
    collection_id: &str,
    active_root_count: i32,
) -> Result<(), CollectionError> {
    sqlx::query(
        "UPDATE collections SET root_count = ?, display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(active_root_count)
    .bind(active_root_count)
    .bind(collection_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_snapshot(
    pool: &SqlitePool,
    collection_id: &str,
    snapshot_json: Option<&str>,
    signature: &str,
    active_root_count: i32,
) -> Result<(), CollectionError> {
    sqlx::query(
        "UPDATE collections SET snapshot_json = ?, signature = ?, root_count = ?, display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(snapshot_json)
    .bind(signature)
    .bind(active_root_count)
    .bind(active_root_count)
    .bind(collection_id)
    .execute(pool)
    .await?;
    Ok(())
}
