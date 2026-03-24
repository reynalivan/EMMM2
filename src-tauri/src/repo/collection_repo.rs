use sqlx::{Row, SqlitePool};

use crate::domain::collection::{
    Collection, CollectionMod, CollectionObject, CollectionRoot, CollectionSummary,
};
use crate::domain::errors::CollectionError;
use crate::services::path_key::collection_name_key;

// ---------------------------------------------------------------------------
// collection_repo — CRUD for `collections` + split member tables
// ---------------------------------------------------------------------------

/// List all collections for a game. Ordered by name.
pub async fn list_for_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<Collection>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT id, game_id, name, name_key, is_safe, is_unsaved, is_last_unsaved,
                  last_active, snapshot_json, signature, root_count, created_at, updated_at
        FROM collections
        WHERE game_id = ?
        ORDER BY is_unsaved DESC, name ASC"#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_collection).collect())
}

/// List collections filtered by corridor and unsaved status.
pub async fn list_for_corridor(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
    include_unsaved: bool,
) -> Result<Vec<Collection>, CollectionError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };
    let unsaved_clause = if include_unsaved {
        ""
    } else {
        " AND is_unsaved = 0"
    };

    let query = format!(
        r#"SELECT id, game_id, name, name_key, is_safe, is_unsaved, is_last_unsaved,
                  last_active, snapshot_json, signature, root_count, created_at, updated_at
        FROM collections
        WHERE game_id = ? AND is_safe = ? {}
        ORDER BY name ASC"#,
        unsaved_clause
    );

    let rows = sqlx::query(&query)
        .bind(game_id)
        .bind(is_safe_i32)
        .fetch_all(pool)
        .await?;

    Ok(rows.iter().map(row_to_collection).collect())
}

/// Get a single collection by ID.
pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Collection>, CollectionError> {
    let row = sqlx::query(
        r#"SELECT id, game_id, name, name_key, is_safe, is_unsaved, is_last_unsaved,
                  last_active, snapshot_json, signature, root_count, created_at, updated_at
        FROM collections
        WHERE id = ?"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_collection))
}

/// Create a new collection.
pub async fn create(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    name: &str,
    is_safe: bool,
    is_unsaved: bool,
) -> Result<Collection, CollectionError> {
    let name_key = collection_name_key(name);
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };
    let is_unsaved_i32 = if is_unsaved { 1i32 } else { 0i32 };

    // Duplicate check for named collections
    if !is_unsaved {
        let existing: Option<String> = sqlx::query_scalar(
            r#"SELECT id FROM collections
            WHERE game_id = ? AND name_key = ? AND is_safe = ? AND is_unsaved = 0"#,
        )
        .bind(game_id)
        .bind(&name_key)
        .bind(is_safe_i32)
        .fetch_optional(pool)
        .await?;

        if existing.is_some() {
            return Err(CollectionError::DuplicateName {
                name: name.to_string(),
            });
        }
    }

    sqlx::query(
        r#"INSERT INTO collections (id, game_id, name, name_key, is_safe, is_unsaved, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"#,
    )
    .bind(id)
    .bind(game_id)
    .bind(name)
    .bind(&name_key)
    .bind(is_safe_i32)
    .bind(is_unsaved_i32)
    .execute(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| CollectionError::NotFound { id: id.to_string() })
}

/// Delete a collection (CASCADE handles members).
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), CollectionError> {
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Member CRUD (Split Tables)
// ---------------------------------------------------------------------------

pub async fn get_mods(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionMod>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT cm.collection_id, cm.mod_id, cm.mod_path, cm.mod_path_key, cm.object_id, 
                  m.actual_name as display_name
           FROM collection_mods cm
           LEFT JOIN mods m ON cm.mod_id = m.id
           WHERE cm.collection_id = ?"#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| CollectionMod {
            kind: crate::domain::collection::MemberKind::Mod,
            collection_id: r.get("collection_id"),
            mod_id: r.get("mod_id"),
            mod_path: r.get("mod_path"),
            mod_path_key: r.get("mod_path_key"),
            object_id: r.get("object_id"),
            display_name: r.get("display_name"),
            is_enabled: true,
        })
        .collect())
}

pub async fn get_objects(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionObject>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT co.collection_id, co.object_id, co.is_enabled, 
                  o.name as display_name, o.folder_path as path_key
           FROM collection_objects co
           LEFT JOIN objects o ON co.object_id = o.id
           WHERE co.collection_id = ?"#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| CollectionObject {
            kind: crate::domain::collection::MemberKind::Object,
            collection_id: r.get("collection_id"),
            object_id: r.get("object_id"),
            is_enabled: r.get::<i32, _>("is_enabled") != 0,
            display_name: r.get("display_name"),
            path_key: r.get("path_key"),
        })
        .collect())
}

pub async fn get_roots(
    pool: &SqlitePool,
    collection_id: &str,
) -> Result<Vec<CollectionRoot>, CollectionError> {
    let rows = sqlx::query(
        r#"SELECT collection_id, root_path, root_path_key, display_name, display_name_key,
                  object_id, object_name, object_type, root_kind, is_safe, is_enabled,
                  thumbnail_hint, corridor_source
        FROM collection_roots WHERE collection_id = ?"#,
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| CollectionRoot {
            kind: crate::domain::collection::MemberKind::Root,
            collection_id: r.get("collection_id"),
            root_path: r.get("root_path"),
            root_path_key: r.get("root_path_key"),
            display_name: r.get("display_name"),
            display_name_key: r.get("display_name_key"),
            object_id: r.get("object_id"),
            object_name: r.get("object_name"),
            object_type: r.get("object_type"),
            root_kind: r.get("root_kind"),
            is_safe: r.get::<i32, _>("is_safe") != 0,
            is_enabled: r.get::<i32, _>("is_enabled") != 0,
            thumbnail_hint: r.get("thumbnail_hint"),
            corridor_source: r.get("corridor_source"),
        })
        .collect())
}

/// Replace all members in a single transaction.
pub async fn replace_all_state(
    pool: &SqlitePool,
    id: &str,
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    roots: &[CollectionRoot],
    signature: Option<&str>,
    snapshot_json: Option<&str>,
) -> Result<(), CollectionError> {
    let mut tx = pool.begin().await?;

    // Clear existing
    sqlx::query("DELETE FROM collection_mods WHERE collection_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM collection_objects WHERE collection_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM collection_roots WHERE collection_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Insert Mods
    if !mods.is_empty() {
        let mut qb = sqlx::QueryBuilder::new("INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_id) ");
        qb.push_values(mods, |mut b, m| {
            b.push_bind(&m.collection_id)
                .push_bind(&m.mod_id)
                .push_bind(&m.mod_path)
                .push_bind(&m.mod_path_key)
                .push_bind(&m.object_id);
        });
        qb.build().execute(&mut *tx).await?;
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
        qb.build().execute(&mut *tx).await?;
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
        qb.build().execute(&mut *tx).await?;
    }

    // Update stats
    sqlx::query("UPDATE collections SET signature = ?, snapshot_json = ?, root_count = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(signature)
        .bind(snapshot_json)
        .bind(roots.len() as i32)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Auto-heal: update mod_path across all collections when a mod is moved/renamed.
pub async fn update_member_paths(
    conn: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    old_mod_path: &str,
    new_mod_path: &str,
    new_object_id: Option<&str>,
) -> Result<u64, CollectionError> {
    let result = sqlx::query(
        r#"UPDATE collection_mods
        SET mod_path = ?, object_id = COALESCE(?, object_id)
        WHERE mod_path = ?"#,
    )
    .bind(new_mod_path)
    .bind(new_object_id)
    .bind(old_mod_path)
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn row_to_collection(r: &sqlx::sqlite::SqliteRow) -> Collection {
    Collection {
        id: r.get("id"),
        game_id: r.get("game_id"),
        name: r.get("name"),
        name_key: r.get("name_key"),
        is_safe: r.get::<i32, _>("is_safe") != 0,
        is_unsaved: r.get::<i32, _>("is_unsaved") != 0,
        is_last_unsaved: r.get::<i32, _>("is_last_unsaved") != 0,
        last_active: r.get::<i32, _>("last_active") != 0,
        snapshot_json: r.get("snapshot_json"),
        signature: r.get("signature"),
        root_count: r.get("root_count"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

pub fn to_summary(
    c: &Collection,
    active_collection_id: Option<&str>,
    undo_collection_id: Option<&str>,
    member_count: i32,
) -> CollectionSummary {
    CollectionSummary {
        id: c.id.clone(),
        name: c.name.clone(),
        is_safe: c.is_safe,
        is_unsaved: c.is_unsaved,
        signature: c.signature.clone(),
        is_active: active_collection_id == Some(c.id.as_str()),
        is_undo_target: undo_collection_id == Some(c.id.as_str()),
        updated_at: c.updated_at.clone(),
        member_count,
    }
}
