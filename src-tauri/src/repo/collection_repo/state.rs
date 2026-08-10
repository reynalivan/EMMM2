//! Whole-snapshot writes: replacing every member table in one transaction and
//! updating the cached signature / display counters.

use sqlx::{SqliteConnection, SqlitePool};

use super::mapping::serialize_warnings_json;
use crate::domain::collection::{CollectionMod, CollectionObject, CollectionRoot};
use crate::domain::errors::CollectionError;

pub struct CollectionStateSnapshot<'a> {
    pub collection_id: &'a str,
    pub mods: &'a [CollectionMod],
    pub objects: &'a [CollectionObject],
    pub roots: &'a [CollectionRoot],
    pub signature: Option<&'a str>,
    pub snapshot_json: Option<&'a str>,
    pub display_mod_count: i32,
}

pub async fn replace_all_state_tx(
    conn: &mut SqliteConnection,
    snapshot: CollectionStateSnapshot<'_>,
) -> Result<(), CollectionError> {
    clear_collection_state(conn, snapshot.collection_id).await?;
    insert_mods(conn, snapshot.mods).await?;
    insert_objects(conn, snapshot.objects).await?;
    insert_roots(conn, snapshot.roots).await?;
    update_collection_snapshot(conn, &snapshot).await
}

async fn clear_collection_state(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<(), CollectionError> {
    sqlx::query("DELETE FROM collection_mods WHERE collection_id = ?")
        .bind(collection_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM collection_objects WHERE collection_id = ?")
        .bind(collection_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM collection_roots WHERE collection_id = ?")
        .bind(collection_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn insert_mods(
    conn: &mut SqliteConnection,
    mods: &[CollectionMod],
) -> Result<(), CollectionError> {
    if mods.is_empty() {
        return Ok(());
    }

    let warnings_json = mods
        .iter()
        .map(|member| serialize_warnings_json(&member.warnings))
        .collect::<Result<Vec<_>, _>>()?;
    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_id, preview_path, node_type, warnings_json) ",
    );
    query_builder.push_values(
        mods.iter().zip(warnings_json.iter()),
        |mut bindings, (member, warnings)| {
            bindings
                .push_bind(&member.collection_id)
                .push_bind(&member.mod_id)
                .push_bind(&member.mod_path)
                .push_bind(&member.mod_path_key)
                .push_bind(&member.object_id)
                .push_bind(&member.preview_path)
                .push_bind(&member.node_type)
                .push_bind(warnings);
        },
    );
    query_builder.build().execute(&mut *conn).await?;
    Ok(())
}

async fn insert_objects(
    conn: &mut SqliteConnection,
    objects: &[CollectionObject],
) -> Result<(), CollectionError> {
    if objects.is_empty() {
        return Ok(());
    }

    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO collection_objects (collection_id, object_id, is_enabled) ",
    );
    query_builder.push_values(objects, |mut bindings, object| {
        bindings
            .push_bind(&object.collection_id)
            .push_bind(&object.object_id)
            .push_bind(if object.is_enabled { 1i32 } else { 0i32 });
    });
    query_builder.build().execute(&mut *conn).await?;
    Ok(())
}

async fn insert_roots(
    conn: &mut SqliteConnection,
    roots: &[CollectionRoot],
) -> Result<(), CollectionError> {
    if roots.is_empty() {
        return Ok(());
    }

    let mut query_builder = sqlx::QueryBuilder::new("INSERT INTO collection_roots (collection_id, root_path, root_path_key, display_name, display_name_key, object_id, object_name, object_type, root_kind, is_safe, is_enabled, thumbnail_hint, corridor_source) ");
    query_builder.push_values(roots, |mut bindings, root| {
        bindings
            .push_bind(&root.collection_id)
            .push_bind(&root.root_path)
            .push_bind(&root.root_path_key)
            .push_bind(&root.display_name)
            .push_bind(&root.display_name_key)
            .push_bind(&root.object_id)
            .push_bind(&root.object_name)
            .push_bind(&root.object_type)
            .push_bind(&root.root_kind)
            .push_bind(if root.is_safe { 1i32 } else { 0i32 })
            .push_bind(if root.is_enabled { 1i32 } else { 0i32 })
            .push_bind(&root.thumbnail_hint)
            .push_bind(&root.corridor_source);
    });
    query_builder.build().execute(&mut *conn).await?;
    Ok(())
}

async fn update_collection_snapshot(
    conn: &mut SqliteConnection,
    snapshot: &CollectionStateSnapshot<'_>,
) -> Result<(), CollectionError> {
    sqlx::query("UPDATE collections SET signature = ?, snapshot_json = ?, root_count = ?, display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(snapshot.signature)
        .bind(snapshot.snapshot_json)
        .bind(snapshot.roots.len() as i32)
        .bind(snapshot.display_mod_count)
        .bind(snapshot.collection_id)
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
