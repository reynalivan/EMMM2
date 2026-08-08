//! Which object a folder on disk *is*, and what a match may overwrite.
//!
//! This used to sit in `repo::object_repo::sync`, where the resolution order,
//! the folder-conflict guard and the overwrite rules were interleaved with the
//! SQL that carried them out. They are domain policy: every scan and every
//! disk reconcile funnels through them, and getting one wrong silently merges
//! two objects or splits one in two. The repo now only looks rows up and
//! writes them; the rules live here, where they can be read in one screen.

use crate::common::path_key::{canonical_name_key, folder_path_key};
use crate::domain::objects::EnsureObjectInput;
use crate::repo::object_repo::{self, ObjectIdentityRow};

/// Resolve `input` to an object id, creating the object if it is new.
///
/// Resolution order is name first, then folder: identity follows the object,
/// not the directory it happens to sit in. A user who renames a folder still
/// has the same object; a user who puts a different mod in that folder does
/// not.
pub async fn ensure_object_exists(
    conn: &mut sqlx::SqliteConnection,
    input: EnsureObjectInput<'_>,
    new_objects_count: &mut usize,
) -> Result<String, sqlx::Error> {
    let name_key = canonical_name_key(input.obj_name);
    let folder_key = folder_path_key(input.folder_path, None);

    let by_name = object_repo::find_by_name_key(conn, input.game_id, &name_key).await?;
    let by_folder = object_repo::find_by_folder_key(conn, input.game_id, &folder_key).await?;

    if let Some(existing) = by_name {
        return merge_into_name_match(conn, &input, existing, by_folder).await;
    }

    if let Some(existing) = by_folder {
        return merge_into_folder_match(conn, &input, existing).await;
    }

    let id = object_repo::insert_object(conn, &input).await?;
    *new_objects_count += 1;
    Ok(id)
}

/// The row matched by name keeps its identity; the incoming folder, spelling
/// and type update it where they are allowed to.
async fn merge_into_name_match(
    conn: &mut sqlx::SqliteConnection,
    input: &EnsureObjectInput<'_>,
    existing: ObjectIdentityRow,
    by_folder: Option<ObjectIdentityRow>,
) -> Result<String, sqlx::Error> {
    // Two rows cannot hold one folder. When the folder already belongs to a
    // different object, refuse the move rather than pointing both at it.
    let folder_taken = by_folder.is_some_and(|row| row.id != existing.id);

    if existing.folder_path != input.folder_path && !folder_taken {
        object_repo::update_object_location(conn, &existing.id, input.folder_path).await?;
    }

    if existing.name != input.obj_name {
        object_repo::update_object_name(conn, &existing.id, input.obj_name).await?;
    }

    if existing.object_type != input.obj_type && input.source.type_is_authoritative() {
        object_repo::update_object_type(conn, &existing.id, input.obj_type).await?;
    }

    object_repo::backfill_empty_columns(conn, &existing.id, input).await?;
    Ok(existing.id)
}

/// No name matched, so the folder is the only link. The row is the same
/// physical object and takes the name that is on disk.
async fn merge_into_folder_match(
    conn: &mut sqlx::SqliteConnection,
    input: &EnsureObjectInput<'_>,
    existing: ObjectIdentityRow,
) -> Result<String, sqlx::Error> {
    if existing.folder_path != input.folder_path {
        object_repo::update_object_location(conn, &existing.id, input.folder_path).await?;
    }

    object_repo::update_object_name(conn, &existing.id, input.obj_name).await?;

    if input.source.type_is_authoritative() {
        object_repo::update_object_type(conn, &existing.id, input.obj_type).await?;
    }

    object_repo::backfill_empty_columns(conn, &existing.id, input).await?;
    Ok(existing.id)
}
