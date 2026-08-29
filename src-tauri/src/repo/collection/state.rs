//! Whole-snapshot writes: replacing canonical members in one transaction and
//! updating the projected snapshot, signature, and list count.

use sqlx::{SqliteConnection, SqlitePool};
use std::collections::HashSet;

use super::mapping::serialize_warnings_json;
use crate::domain::collection::{CollectionMod, CollectionObject};
use crate::domain::errors::CollectionError;

fn object_ref_key_for_mod(member: &CollectionMod) -> String {
    let key = crate::common::path_key::folder_path_key(&member.mod_path, None);
    key.split('/').next().unwrap_or(&key).to_string()
}

pub struct CollectionStateSnapshot<'a> {
    pub collection_id: &'a str,
    pub mods: &'a [CollectionMod],
    pub objects: &'a [CollectionObject],
    pub signature: Option<&'a str>,
    pub update_signature: bool,
    pub snapshot_json: Option<&'a str>,
    pub display_mod_count: i32,
}

pub async fn replace_all_state_tx(
    conn: &mut SqliteConnection,
    snapshot: CollectionStateSnapshot<'_>,
) -> Result<(), CollectionError> {
    let existing_mod_ids: HashSet<String> = sqlx::query_scalar("SELECT id FROM mods")
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .collect();
    let existing_object_ids: HashSet<String> = sqlx::query_scalar("SELECT id FROM objects")
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .collect();

    clear_collection_state(conn, snapshot.collection_id).await?;
    insert_mods(conn, snapshot.mods, &existing_mod_ids, &existing_object_ids).await?;
    insert_objects(conn, snapshot.objects, &existing_object_ids).await?;
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
    Ok(())
}

async fn insert_mods(
    conn: &mut SqliteConnection,
    mods: &[CollectionMod],
    existing_mod_ids: &HashSet<String>,
    existing_object_ids: &HashSet<String>,
) -> Result<(), CollectionError> {
    if mods.is_empty() {
        return Ok(());
    }

    let warnings_json = mods
        .iter()
        .map(|member| serialize_warnings_json(&member.warnings))
        .collect::<Result<Vec<_>, _>>()?;
    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_ref_key, object_id, preview_path, node_type, warnings_json, is_safe, safety_source) ",
    );
    query_builder.push_values(
        mods.iter().zip(warnings_json.iter()),
        |mut bindings, (member, warnings)| {
            bindings
                .push_bind(&member.collection_id)
                .push_bind(
                    member
                        .mod_id
                        .as_ref()
                        .filter(|id| existing_mod_ids.contains(*id)),
                )
                .push_bind(&member.mod_path)
                .push_bind(&member.mod_path_key)
                .push_bind(object_ref_key_for_mod(member))
                .push_bind(
                    existing_object_ids
                        .contains(&member.object_id)
                        .then_some(member.object_id.as_str()),
                )
                .push_bind(&member.preview_path)
                .push_bind(&member.node_type)
                .push_bind(warnings)
                .push_bind(member.is_safe)
                .push_bind(&member.safety_source);
        },
    );
    query_builder.build().execute(&mut *conn).await?;
    Ok(())
}

async fn insert_objects(
    conn: &mut SqliteConnection,
    objects: &[CollectionObject],
    existing_object_ids: &HashSet<String>,
) -> Result<(), CollectionError> {
    if objects.is_empty() {
        return Ok(());
    }

    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO collection_objects (collection_id, object_ref_key, object_id, object_display_name, is_enabled) ",
    );
    query_builder.push_values(objects, |mut bindings, object| {
        bindings
            .push_bind(&object.collection_id)
            .push_bind(
                object
                    .path_key
                    .as_deref()
                    .unwrap_or(object.object_id.as_str()),
            )
            .push_bind(
                existing_object_ids
                    .contains(&object.object_id)
                    .then_some(object.object_id.as_str()),
            )
            .push_bind(&object.display_name)
            .push_bind(if object.is_enabled { 1i32 } else { 0i32 });
    });
    query_builder.build().execute(&mut *conn).await?;
    Ok(())
}

async fn update_collection_snapshot(
    conn: &mut SqliteConnection,
    snapshot: &CollectionStateSnapshot<'_>,
) -> Result<(), CollectionError> {
    sqlx::query("UPDATE collections SET signature = CASE WHEN ? THEN ? ELSE signature END, snapshot_json = ?, display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(snapshot.update_signature)
        .bind(snapshot.signature)
        .bind(snapshot.snapshot_json)
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
        "UPDATE collections SET display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
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
        "UPDATE collections SET snapshot_json = ?, signature = ?, display_mod_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(snapshot_json)
    .bind(signature)
    .bind(active_root_count)
    .bind(collection_id)
    .execute(pool)
    .await?;
    Ok(())
}
