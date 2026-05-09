use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::database::models::ItemStatus;
use crate::services::corridor_constants::{CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN};
use crate::services::explorer::classifier::{classify_folder, NodeType};
use crate::services::path_key::{
    canonical_name_key, folder_path_key, object_name_key, resolve_collection_path,
};
use crate::services::scanner::core::normalizer::is_disabled_folder;

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct ObjectFilter {
    pub game_id: String,
    pub search_query: Option<String>,
    pub object_type: Option<String>,
    pub safe_mode: bool,
    pub meta_filters: Option<HashMap<String, Vec<String>>>,
    pub sort_by: Option<String>,
    pub status_filter: Option<ItemStatus>,
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct GetObjectsResult {
    pub objects: Vec<ObjectSummary>,
    pub lost_objects: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct ObjectSummary {
    pub id: String,
    pub name: String,
    pub folder_path: String,
    pub matched_entry_key: Option<String>,
    pub matched_alias_name: Option<String>,
    pub matched_confidence: Option<f64>,
    pub matched_reason: Option<String>,
    pub matched_source: Option<String>,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub status: ItemStatus, // 1: ENABLED, 0: DISABLED
    pub metadata: String,
    pub tags: String,
    pub hash_db: Option<crate::database::models::HashDbPayload>,
    pub custom_skins: Option<crate::database::models::CustomSkinsPayload>,
    pub is_pinned: bool,
    pub is_auto_sync: bool,
    pub thumbnail_path: Option<String>,
    pub created_at: Option<String>,
    #[specta(type = f64)]
    pub mod_count: i64,
    #[specta(type = f64)]
    pub enabled_count: i64,
    pub is_object_disabled: bool,
    pub has_naming_conflict: bool,
    pub active_mod_paths: Option<String>,
}

#[derive(Clone, sqlx::FromRow)]
struct ObjectSummaryRow {
    id: String,
    name: String,
    folder_path: String,
    matched_entry_key: Option<String>,
    matched_alias_name: Option<String>,
    matched_confidence: Option<f64>,
    matched_reason: Option<String>,
    matched_source: Option<String>,
    object_type: String,
    sub_category: Option<String>,
    status: ItemStatus,
    metadata: String,
    tags: String,
    hash_db: Option<crate::database::models::HashDbPayload>,
    custom_skins: Option<crate::database::models::CustomSkinsPayload>,
    is_pinned: bool,
    is_auto_sync: bool,
    thumbnail_path: Option<String>,
    created_at: Option<String>,
    mod_count: i64,
    enabled_count: i64,
    is_object_disabled: bool,
    has_naming_conflict: bool,
    active_mod_paths: Option<String>,
    projection_available: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct ObjectRuntimeDescriptor {
    pub id: String,
    pub name: String,
    pub folder_path: String,
    pub folder_path_key: String,
    pub matched_entry_key: Option<String>,
    pub matched_alias_name: Option<String>,
    pub object_type: String,
    pub thumbnail_path: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct CategoryCount {
    pub object_type: String,
    #[specta(type = f64)]
    pub count: i64,
}

#[derive(Clone, Debug, sqlx::FromRow)]
struct ObjectCountCandidate {
    object_id: String,
    folder_path: String,
    actual_name: String,
    status: ItemStatus,
}

#[derive(Clone, Debug)]
struct TerminalDescriptor {
    display_path: String,
    display_segments: Vec<String>,
}

pub async fn get_filtered_objects(
    pool: &SqlitePool,
    filter: &ObjectFilter,
) -> Result<Vec<ObjectSummary>, sqlx::Error> {
    let safe_mode = if filter.safe_mode { 1i64 } else { 0i64 };
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT
            o.id,
            o.name,
            o.folder_path,
            o.matched_entry_key,
            o.matched_alias_name,
            o.matched_confidence,
            o.matched_reason,
            o.matched_source,
            o.object_type,
            o.sub_category,
            o.status,
            COALESCE(o.metadata, '{{}}') as metadata,
            COALESCE(o.tags, '[]') as tags,
            CASE WHEN json_valid(o.hash_db) = 1 THEN o.hash_db ELSE NULL END as hash_db,
            CASE WHEN json_valid(o.custom_skins) = 1 THEN o.custom_skins ELSE NULL END as custom_skins,
            COALESCE(o.is_pinned, 0) as is_pinned,
            COALESCE(o.is_auto_sync, 0) as is_auto_sync,
            o.thumbnail_path,
            o.created_at,
            CASE WHEN "#,
    );
    qb.push_bind(safe_mode);
    qb.push(
        r#" = 1
                THEN COALESCE(p.mod_count_safe, 0)
                ELSE COALESCE(p.mod_count_unsafe, 0)
            END as mod_count,
            CASE WHEN "#,
    );
    qb.push_bind(safe_mode);
    qb.push(
        r#" = 1
                THEN COALESCE(p.enabled_count_safe, 0)
                ELSE COALESCE(p.enabled_count_unsafe, 0)
            END as enabled_count,
            CASE WHEN "#,
    );
    qb.push_bind(safe_mode);
    qb.push(
        r#" = 1
                THEN NULLIF(COALESCE(p.active_mod_paths_safe_json, '[]'), '[]')
                ELSE NULLIF(COALESCE(p.active_mod_paths_unsafe_json, '[]'), '[]')
            END as active_mod_paths,
            COALESCE(p.is_object_disabled, CASE WHEN o.status = 0 THEN 1 ELSE 0 END) as is_object_disabled,
            COALESCE(p.has_naming_conflict, 0) as has_naming_conflict,
            CASE WHEN p.object_id IS NULL THEN 0 ELSE 1 END as projection_available
        FROM objects o
        LEFT JOIN object_runtime_projection p
            ON p.game_id = o.game_id AND p.object_id = o.id
        WHERE o.game_id = "#,
    );
    qb.push_bind(&filter.game_id);

    if let Some(obj_type) = &filter.object_type {
        qb.push(" AND o.object_type = ");
        qb.push_bind(obj_type);
    }

    if let Some(sq) = &filter.search_query {
        let trimmed = sq.trim();
        if !trimmed.is_empty() {
            let name_search_term = format!("%{}%", object_name_key(trimmed));
            let tag_search_term = format!("%{}%", trimmed.to_lowercase());
            qb.push(" AND (o.name_key LIKE ");
            qb.push_bind(name_search_term);
            qb.push(" OR LOWER(o.tags) LIKE ");
            qb.push_bind(tag_search_term);
            qb.push(")");
        }
    }

    if let Some(meta_filters) = &filter.meta_filters {
        for (key, values) in meta_filters {
            if !values.is_empty() {
                let safe_key = key.replace(['\'', '"'], "");
                qb.push(format!(
                    " AND JSON_EXTRACT(o.metadata, '$.{}') IN (",
                    safe_key
                ));
                let mut separated = qb.separated(", ");
                for v in values {
                    separated.push_bind(v);
                }
                separated.push_unseparated(")");
            }
        }
    }

    match filter.sort_by.as_deref() {
        Some("date") => qb.push(" ORDER BY o.is_pinned DESC, o.created_at DESC"),
        Some("rarity") => qb.push(" ORDER BY o.is_pinned DESC, CAST(JSON_EXTRACT(o.metadata, '$.rarity') AS INTEGER) DESC, o.name ASC"),
        _ => qb.push(" ORDER BY o.is_pinned DESC, o.object_type, o.name ASC"),
    };

    let mut rows = qb
        .build_query_as::<ObjectSummaryRow>()
        .fetch_all(pool)
        .await?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let missing_projection_ids: Vec<String> = rows
        .iter()
        .filter(|row| row.projection_available == 0)
        .map(|row| row.id.clone())
        .collect();

    if !missing_projection_ids.is_empty() {
        let fallback_objects: Vec<ObjectSummary> = rows
            .iter()
            .filter(|row| row.projection_available == 0)
            .map(|row| ObjectSummary {
                id: row.id.clone(),
                name: row.name.clone(),
                folder_path: row.folder_path.clone(),
                matched_entry_key: row.matched_entry_key.clone(),
                matched_alias_name: row.matched_alias_name.clone(),
                matched_confidence: row.matched_confidence,
                matched_reason: row.matched_reason.clone(),
                matched_source: row.matched_source.clone(),
                object_type: row.object_type.clone(),
                sub_category: row.sub_category.clone(),
                status: row.status,
                metadata: row.metadata.clone(),
                tags: row.tags.clone(),
                hash_db: row.hash_db.clone(),
                custom_skins: row.custom_skins.clone(),
                is_pinned: row.is_pinned,
                is_auto_sync: row.is_auto_sync,
                thumbnail_path: row.thumbnail_path.clone(),
                created_at: row.created_at.clone(),
                mod_count: 0,
                enabled_count: 0,
                is_object_disabled: row.is_object_disabled,
                has_naming_conflict: row.has_naming_conflict,
                active_mod_paths: None,
            })
            .collect();

        let mods_path = load_game_mods_path(pool, &filter.game_id).await?;
        let count_candidates = load_object_count_candidates(
            pool,
            &filter.game_id,
            filter.safe_mode,
            &fallback_objects,
        )
        .await?;
        let counts_by_object =
            build_terminal_counts(&fallback_objects, &count_candidates, mods_path.as_deref());

        for row in &mut rows {
            let Some((mod_count, enabled_count, active_paths)) = counts_by_object.get(&row.id)
            else {
                continue;
            };
            row.mod_count = *mod_count;
            row.enabled_count = *enabled_count;
            row.active_mod_paths = active_paths.clone();
        }

        let _ = crate::services::runtime_projection_service::refresh_objects_projection(
            pool,
            &filter.game_id,
            &missing_projection_ids,
        )
        .await;
    }

    let mut objects: Vec<ObjectSummary> = rows
        .into_iter()
        .map(|row| ObjectSummary {
            id: row.id,
            name: row.name,
            folder_path: row.folder_path,
            matched_entry_key: row.matched_entry_key,
            matched_alias_name: row.matched_alias_name,
            matched_confidence: row.matched_confidence,
            matched_reason: row.matched_reason,
            matched_source: row.matched_source,
            object_type: row.object_type,
            sub_category: row.sub_category,
            status: row.status,
            metadata: row.metadata,
            tags: row.tags,
            hash_db: row.hash_db,
            custom_skins: row.custom_skins,
            is_pinned: row.is_pinned,
            is_auto_sync: row.is_auto_sync,
            thumbnail_path: row.thumbnail_path,
            created_at: row.created_at,
            mod_count: row.mod_count,
            enabled_count: row.enabled_count,
            is_object_disabled: row.is_object_disabled,
            has_naming_conflict: row.has_naming_conflict,
            active_mod_paths: row.active_mod_paths,
        })
        .collect();

    if let Some(status) = filter.status_filter {
        objects.retain(|object| match status {
            ItemStatus::Enabled => !object.is_object_disabled,
            ItemStatus::Disabled => object.is_object_disabled,
        });
    }

    Ok(objects)
}

async fn load_game_mods_path(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT mods_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(pool)
        .await
        .map(|value| value.flatten())
}

async fn load_object_count_candidates(
    pool: &SqlitePool,
    game_id: &str,
    safe_mode: bool,
    objects: &[ObjectSummary],
) -> Result<Vec<ObjectCountCandidate>, sqlx::Error> {
    if objects.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT m.object_id, m.folder_path, m.actual_name, m.status FROM mods m WHERE m.game_id = ",
    );
    qb.push_bind(game_id);
    qb.push(" AND m.object_id IN (");
    {
        let mut separated = qb.separated(", ");
        for object in objects {
            separated.push_bind(&object.id);
        }
    }
    qb.push(")");
    append_corridor_visibility_filter(&mut qb, safe_mode);

    qb.build_query_as::<ObjectCountCandidate>()
        .fetch_all(pool)
        .await
}

fn append_corridor_visibility_filter(qb: &mut QueryBuilder<Sqlite>, safe_mode: bool) {
    let expected_is_safe = if safe_mode { 1 } else { 0 };
    qb.push(" AND (COALESCE(m.is_safe, 1) = ");
    qb.push_bind(expected_is_safe);
    qb.push(" OR COALESCE(m.corridor_source, ");
    qb.push_bind(CORRIDOR_SOURCE_UNKNOWN);
    qb.push(") IN (");
    qb.push_bind(CORRIDOR_SOURCE_MANUAL);
    qb.push(", ");
    qb.push_bind(CORRIDOR_SOURCE_UNKNOWN);
    qb.push("))");
}

fn build_terminal_counts(
    objects: &[ObjectSummary],
    candidates: &[ObjectCountCandidate],
    mods_path: Option<&str>,
) -> HashMap<String, (i64, i64, Option<String>)> {
    let object_lookup: HashMap<&str, &ObjectSummary> = objects
        .iter()
        .map(|object| (object.id.as_str(), object))
        .collect();
    let mut totals_by_object: HashMap<String, HashSet<String>> = HashMap::new();
    let mut enabled_by_object: HashMap<String, HashSet<String>> = HashMap::new();
    let mut active_paths_by_object: HashMap<String, HashMap<String, String>> = HashMap::new();

    for candidate in candidates {
        let Some(object) = object_lookup.get(candidate.object_id.as_str()) else {
            continue;
        };
        let Some(descriptor) = resolve_terminal_descriptor(object, candidate, mods_path) else {
            continue;
        };

        let terminal_key = folder_path_key(&descriptor.display_path, mods_path);
        totals_by_object
            .entry(candidate.object_id.clone())
            .or_default()
            .insert(terminal_key.clone());

        if candidate.status != ItemStatus::Enabled {
            continue;
        }
        if has_disabled_ancestor(&descriptor.display_segments) {
            continue;
        }

        enabled_by_object
            .entry(candidate.object_id.clone())
            .or_default()
            .insert(terminal_key.clone());
        active_paths_by_object
            .entry(candidate.object_id.clone())
            .or_default()
            .entry(terminal_key)
            .or_insert(descriptor.display_path);
    }

    let mut counts = HashMap::new();
    for object in objects {
        let total = totals_by_object
            .get(&object.id)
            .map(|entries| entries.len() as i64)
            .unwrap_or(0);
        let enabled = enabled_by_object
            .get(&object.id)
            .map(|entries| entries.len() as i64)
            .unwrap_or(0);
        let active_paths = active_paths_by_object.get(&object.id).map(|entries| {
            let mut values: Vec<String> = entries.values().cloned().collect();
            values.sort_by_key(|value| canonical_name_key(value));
            values.join("|")
        });
        counts.insert(object.id.clone(), (total, enabled, active_paths));
    }

    counts
}

fn resolve_terminal_descriptor(
    object: &ObjectSummary,
    candidate: &ObjectCountCandidate,
    mods_path: Option<&str>,
) -> Option<TerminalDescriptor> {
    let relative_segments = relative_segments_for_path(
        &object.folder_path,
        &object.name,
        &candidate.folder_path,
        &candidate.actual_name,
    );
    if relative_segments.is_empty() {
        return None;
    }

    let terminal_path = resolve_collection_path(&candidate.folder_path, mods_path);
    let candidate_paths = cumulative_candidate_paths(&terminal_path, relative_segments.len());
    for candidate_path in candidate_paths {
        let Some(_node_type) = classify_terminal_type(candidate_path.as_deref()) else {
            continue;
        };
        let display_path = candidate_path
            .as_ref()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| candidate.folder_path.clone());
        let display_segments = relative_segments_for_path(
            &object.folder_path,
            &object.name,
            &display_path,
            &candidate.actual_name,
        );
        if display_segments.is_empty() {
            continue;
        }

        return Some(TerminalDescriptor {
            display_path,
            display_segments,
        });
    }

    let node_type = classify_terminal_type(terminal_path.as_deref())?;
    let display_path = terminal_path
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| candidate.folder_path.clone());
    let display_segments = relative_segments_for_path(
        &object.folder_path,
        &object.name,
        &display_path,
        &candidate.actual_name,
    );
    if display_segments.is_empty() {
        return None;
    }

    if !matches!(
        node_type,
        NodeType::FlatModRoot | NodeType::ModPackRoot | NodeType::VariantContainer
    ) {
        return None;
    }

    Some(TerminalDescriptor {
        display_path,
        display_segments,
    })
}

fn classify_terminal_type(path: Option<&Path>) -> Option<NodeType> {
    let target = path?;
    let (node_type, _reasons, _warnings) = classify_folder(target);
    if matches!(
        node_type,
        NodeType::FlatModRoot | NodeType::ModPackRoot | NodeType::VariantContainer
    ) {
        return Some(node_type);
    }
    None
}

fn has_disabled_ancestor(segments: &[String]) -> bool {
    segments
        .iter()
        .take(segments.len().saturating_sub(1))
        .any(|segment| is_disabled_folder(segment))
}

fn cumulative_candidate_paths(
    path: &Option<PathBuf>,
    segment_count: usize,
) -> Vec<Option<PathBuf>> {
    let Some(full_path) = path.clone() else {
        return Vec::new();
    };

    let mut current = full_path;
    let mut reversed = Vec::with_capacity(segment_count);
    reversed.push(Some(current.clone()));
    for _ in 1..segment_count {
        let Some(parent) = current.parent() else {
            reversed.push(None);
            continue;
        };
        let parent_path = parent.to_path_buf();
        reversed.push(Some(parent_path.clone()));
        current = parent_path;
    }
    reversed.reverse();
    reversed
}

fn relative_segments_for_path(
    object_folder_path: &str,
    object_name: &str,
    path: &str,
    fallback_name: &str,
) -> Vec<String> {
    let path_segments = split_segments(path);
    let anchors = [object_folder_path.to_string(), object_name.to_string()];

    for anchor in anchors {
        let anchor_segments = split_segments(&anchor);
        if anchor_segments.is_empty() || anchor_segments.len() > path_segments.len() {
            continue;
        }
        let Some(start_index) = find_anchor_start(&path_segments, &anchor_segments) else {
            continue;
        };
        let relative = path_segments[(start_index + anchor_segments.len())..].to_vec();
        if !relative.is_empty() {
            return relative;
        }
    }

    vec![path_leaf(path, fallback_name)]
}

fn split_segments(path: &str) -> Vec<String> {
    path.replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .collect()
}

fn find_anchor_start(path_segments: &[String], anchor_segments: &[String]) -> Option<usize> {
    for index in 0..=(path_segments.len() - anchor_segments.len()) {
        let matches = anchor_segments.iter().enumerate().all(|(offset, anchor)| {
            canonical_name_key(&path_segments[index + offset]) == canonical_name_key(anchor)
        });
        if matches {
            return Some(index);
        }
    }
    None
}

fn path_leaf(path: &str, fallback_name: &str) -> String {
    split_segments(path)
        .last()
        .cloned()
        .unwrap_or_else(|| fallback_name.to_string())
}

pub async fn get_runtime_descriptors(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<ObjectRuntimeDescriptor>, sqlx::Error> {
    sqlx::query_as::<_, ObjectRuntimeDescriptor>(
        r#"
        SELECT
            id,
            name,
            folder_path,
            folder_path_key,
            matched_entry_key,
            matched_alias_name,
            object_type,
            thumbnail_path
        FROM objects
        WHERE game_id = ?
        ORDER BY name ASC
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
}

pub async fn get_category_counts(
    pool: &SqlitePool,
    game_id: &str,
    _safe_mode: bool,
) -> Result<Vec<CategoryCount>, sqlx::Error> {
    // Phase 1 fix: always count ALL objects regardless of safe mode.
    // Category badges should show total counts; individual object counts
    // are zeroed for unsafe objects at the object level.
    let mut qb: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT object_type, COUNT(*) as count FROM objects WHERE game_id = ");
    qb.push_bind(game_id);

    qb.push(" GROUP BY object_type ORDER BY object_type");

    qb.build_query_as::<CategoryCount>().fetch_all(pool).await
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct CreateObjectInput {
    pub game_id: String,
    pub name: String,
    pub folder_path: Option<String>,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub status: Option<ItemStatus>,
    pub metadata: Option<serde_json::Value>,
    pub thumbnail_url: Option<String>,
    pub hash_db: Option<crate::database::models::HashDbPayload>,
    pub custom_skins: Option<crate::database::models::CustomSkinsPayload>,
}

#[allow(clippy::too_many_arguments)] // Repository insert keeps DB columns explicit at call sites.
pub async fn create_object(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    name: &str,
    folder_path: &str,
    object_type: &str,
    sub_category: Option<&String>,
    status: Option<ItemStatus>,
    metadata_str: &str,
    thumbnail_path: Option<&String>,
    hash_db_str: Option<&str>,
    custom_skins_str: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO objects (id, game_id, name, name_key, folder_path, folder_path_key, status, object_type, sub_category, is_auto_sync, tags, metadata, hash_db, custom_skins, thumbnail_path, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, '[]', ?, ?, ?, ?, datetime('now'))
        "#,
    )
    .bind(id)
    .bind(game_id)
    .bind(name)
    .bind(object_name_key(name))
    .bind(folder_path)
    .bind(folder_path_key(folder_path, None))
    .bind(status.unwrap_or(ItemStatus::Enabled) as i64) // DEFAULT ENABLED (1)
    .bind(object_type)
    .bind(sub_category)
    .bind(metadata_str)
    .bind(hash_db_str)
    .bind(custom_skins_str)
    .bind(thumbnail_path)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_object(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM objects WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Atomically delete an object folder and all its child mods from the DB.
///
/// Used when the watcher detects a depth=1 `Removed` event (an entire object
/// folder was deleted from disk). The operation runs inside a single transaction:
/// 1. Delete all `mods` rows whose `folder_path` starts with `{folder_path}/` or `{folder_path}\`
/// 2. Delete the `objects` row with `folder_path = folder_path AND game_id = game_id`
///
/// Idempotent — safe to call even if the object does not exist.
pub async fn delete_object_and_mods_by_folder(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path: &str,
) -> Result<u64, sqlx::Error> {
    let mods_path: Option<String> = sqlx::query_scalar("SELECT mods_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(&mut *conn)
        .await?
        .flatten();
    let child_prefix_key = format!("{}/%", folder_path_key(folder_path, mods_path.as_deref()));
    let mods_deleted = sqlx::query("DELETE FROM mods WHERE game_id = ? AND folder_path_key LIKE ?")
        .bind(game_id)
        .bind(child_prefix_key)
        .execute(&mut *conn)
        .await?
        .rows_affected();

    // Delete the object itself
    sqlx::query("DELETE FROM objects WHERE game_id = ? AND folder_path_key = ?")
        .bind(game_id)
        .bind(folder_path_key(folder_path, None))
        .execute(&mut *conn)
        .await?;

    log::info!(
        "delete_object_and_mods_by_folder: removed object folder='{}' game='{}', {} child mods deleted",
        folder_path, game_id, mods_deleted
    );
    Ok(mods_deleted)
}

pub async fn get_mod_count_for_object(pool: &SqlitePool, id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE object_id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

/// Delete all mod rows belonging to an object (cascade helper).
pub async fn delete_mods_for_object(
    pool: &SqlitePool,
    object_id: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM mods WHERE object_id = ?")
        .bind(object_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn get_objects_folder_paths(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query!("SELECT folder_path FROM objects WHERE game_id = ?", game_id)
        .fetch_all(pool)
        .await?;

    // Some folder_paths might be null if objects row is malformed or derived, but
    // DB schema likely has folder_path as TEXT nullable.
    // In original code, it iterated and checked `o.folder_path`.
    let mut paths = Vec::new();
    for row in rows {
        if let Some(fp) = row.folder_path {
            paths.push(fp);
        }
    }
    Ok(paths)
}

pub async fn update_object_folder_path<'c, E>(
    executor: E,
    game_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET folder_path = ?,
             folder_path_key = ?,
             name = CASE WHEN name = ? THEN ? ELSE name END,
             name_key = CASE WHEN name = ? THEN ? ELSE name_key END
         WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(new_path)
    .bind(folder_path_key(new_path, None))
    .bind(old_path)
    .bind(new_path)
    .bind(old_path)
    .bind(object_name_key(new_path))
    .bind(game_id)
    .bind(folder_path_key(old_path, None))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_object_runtime_folder_path<'c, E>(
    executor: E,
    game_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET folder_path = ?,
             folder_path_key = ?
         WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(new_path)
    .bind(folder_path_key(new_path, None))
    .bind(game_id)
    .bind(folder_path_key(old_path, None))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_object_runtime_state_by_path<'c, E>(
    executor: E,
    game_id: &str,
    old_path: &str,
    new_path: &str,
    status: ItemStatus,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET folder_path = ?,
             folder_path_key = ?,
             status = ?
         WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(new_path)
    .bind(folder_path_key(new_path, None))
    .bind(status as i64)
    .bind(game_id)
    .bind(folder_path_key(old_path, None))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_object_runtime_state_by_id<'c, E>(
    executor: E,
    object_id: &str,
    folder_path: &str,
    status: ItemStatus,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET folder_path = ?,
             folder_path_key = ?,
             status = ?
         WHERE id = ?",
    )
    .bind(folder_path)
    .bind(folder_path_key(folder_path, None))
    .bind(status as i64)
    .bind(object_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_object_status<'c, E>(
    executor: E,
    object_id: &str,
    status: ItemStatus,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query("UPDATE objects SET status = ? WHERE id = ?")
        .bind(status as i64)
        .bind(object_id)
        .execute(executor)
        .await?;
    Ok(())
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct UpdateObjectInput {
    pub name: Option<String>,
    pub object_type: Option<String>,
    pub sub_category: Option<String>,
    pub status: Option<ItemStatus>,
    pub metadata: Option<serde_json::Value>,
    pub hash_db: Option<crate::database::models::HashDbPayload>,
    pub custom_skins: Option<crate::database::models::CustomSkinsPayload>,
    pub thumbnail_path: Option<String>,
    pub is_auto_sync: Option<bool>,
    pub is_pinned: Option<bool>,
    pub tags: Option<Vec<String>>,
}

pub async fn update_object(
    pool: &SqlitePool,
    id: &str,
    updates: &UpdateObjectInput,
) -> Result<(), sqlx::Error> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("UPDATE objects SET ");
    let mut is_first = true;

    if let Some(name) = &updates.name {
        if !is_first {
            qb.push(", ");
        }
        qb.push("name = ");
        qb.push_bind(name.trim().to_string());
        qb.push(", name_key = ");
        qb.push_bind(object_name_key(name.trim()));
        is_first = false;
    }
    if let Some(obj_type) = &updates.object_type {
        if !is_first {
            qb.push(", ");
        }
        qb.push("object_type = ");
        qb.push_bind(obj_type);
        is_first = false;
    }
    if let Some(st) = &updates.status {
        if !is_first {
            qb.push(", ");
        }
        qb.push("status = ");
        qb.push_bind(*st as i64);
        is_first = false;
    }
    if let Some(sub) = &updates.sub_category {
        if !is_first {
            qb.push(", ");
        }
        qb.push("sub_category = ");
        qb.push_bind(sub);
        is_first = false;
    }
    if let Some(meta) = &updates.metadata {
        if !is_first {
            qb.push(", ");
        }
        qb.push("metadata = ");
        qb.push_bind(meta.to_string());
        is_first = false;
    }
    if let Some(hash) = &updates.hash_db {
        if !is_first {
            qb.push(", ");
        }
        qb.push("hash_db = ");
        qb.push_bind(serde_json::to_string(hash).unwrap_or_else(|_| "{}".to_string()));
        is_first = false;
    }
    if let Some(skins) = &updates.custom_skins {
        if !is_first {
            qb.push(", ");
        }
        qb.push("custom_skins = ");
        qb.push_bind(serde_json::to_string(skins).unwrap_or_else(|_| "{}".to_string()));
        is_first = false;
    }
    if let Some(thumb) = &updates.thumbnail_path {
        if !is_first {
            qb.push(", ");
        }
        qb.push("thumbnail_path = ");
        qb.push_bind(thumb);
        is_first = false;
    }
    if let Some(auto) = updates.is_auto_sync {
        if !is_first {
            qb.push(", ");
        }
        qb.push("is_auto_sync = ");
        qb.push_bind(auto);
        is_first = false;
    }
    if let Some(pinned) = updates.is_pinned {
        if !is_first {
            qb.push(", ");
        }
        qb.push("is_pinned = ");
        qb.push_bind(pinned);
        is_first = false;
    }

    if let Some(tags) = &updates.tags {
        if !is_first {
            qb.push(", ");
        }
        qb.push("tags = ");
        qb.push_bind(serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string()));
        is_first = false;
    }

    if is_first {
        return Ok(());
    }

    qb.push(" WHERE id = ");
    qb.push_bind(id);

    qb.build().execute(pool).await?;
    Ok(())
}

pub async fn get_characters_for_game(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    use sqlx::Row;
    let rows =
        sqlx::query("SELECT id, name FROM objects WHERE game_id = ? AND object_type = 'Character'")
            .bind(game_id)
            .fetch_all(pool)
            .await?;

    let mut result = Vec::new();
    for row in rows {
        result.push((row.try_get("id")?, row.try_get("name")?));
    }
    Ok(result)
}

pub async fn get_folder_path(pool: &SqlitePool, id: &str) -> Result<Option<String>, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query("SELECT folder_path FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        Ok(r.try_get("folder_path").ok())
    } else {
        Ok(None)
    }
}

pub async fn get_game_object_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<crate::services::scanner::core::types::GameObject>, sqlx::Error> {
    sqlx::query_as::<_, crate::services::scanner::core::types::GameObject>(
        "SELECT * FROM objects WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn set_is_pinned(
    pool: &SqlitePool,
    id: &str,
    is_pinned: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE objects SET is_pinned = ? WHERE id = ?")
        .bind(is_pinned)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub struct EnsureObjectInput<'a> {
    pub game_id: &'a str,
    pub folder_path: &'a str,
    pub obj_name: &'a str,
    pub obj_type: &'a str,
    pub db_thumbnail: Option<&'a str>,
    pub db_tags_json: &'a str,
    pub db_metadata_json: &'a str,
    pub db_hash_db_json: Option<&'a str>,
    pub db_custom_skins_json: Option<&'a str>,
}

pub async fn ensure_object_exists(
    conn: &mut sqlx::SqliteConnection,
    input: EnsureObjectInput<'_>,
    new_objects_count: &mut usize,
) -> Result<String, String> {
    use sqlx::Row;
    let EnsureObjectInput {
        game_id,
        folder_path,
        obj_name,
        obj_type,
        db_thumbnail,
        db_tags_json,
        db_metadata_json,
        db_hash_db_json,
        db_custom_skins_json,
    } = input;
    let name_key = object_name_key(obj_name);
    let folder_key = folder_path_key(folder_path, None);

    let match_name = sqlx::query(
        "SELECT id, name, folder_path, object_type, thumbnail_path, tags, metadata, hash_db, custom_skins
         FROM objects
         WHERE game_id = ? AND name_key = ?",
    )
    .bind(game_id)
    .bind(&name_key)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;

    let match_folder = sqlx::query(
        "SELECT id, name, folder_path, object_type, thumbnail_path, tags, metadata, hash_db, custom_skins
         FROM objects
         WHERE game_id = ? AND folder_path_key = ?",
    )
    .bind(game_id)
    .bind(&folder_key)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(row) = match_name {
        let id: String = row.try_get("id").unwrap_or_default();
        let existing_name: String = row.try_get("name").unwrap_or_default();
        let existing_fp: String = row.try_get("folder_path").unwrap_or_default();
        let existing_type: String = row
            .try_get("object_type")
            .unwrap_or_else(|_| "Other".to_string());
        let existing_thumb: Option<String> = row.try_get("thumbnail_path").unwrap_or(None);
        let existing_tags: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        let existing_meta: String = row.try_get("metadata").unwrap_or_else(|_| "{}".to_string());
        let existing_hash: Option<String> = row.try_get("hash_db").unwrap_or(None);
        let existing_skins: Option<String> = row.try_get("custom_skins").unwrap_or(None);

        let has_folder_conflict = match_folder
            .as_ref()
            .and_then(|folder_row| folder_row.try_get::<String, _>("id").ok())
            .is_some_and(|folder_id| folder_id != id);

        if existing_fp != folder_path && !has_folder_conflict {
            sqlx::query("UPDATE objects SET folder_path = ?, folder_path_key = ? WHERE id = ?")
                .bind(folder_path)
                .bind(&folder_key)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_name != obj_name {
            sqlx::query("UPDATE objects SET name = ?, name_key = ? WHERE id = ?")
                .bind(obj_name)
                .bind(&name_key)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_type != obj_type && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET object_type = ? WHERE id = ?")
                .bind(obj_type)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_thumb.is_none() && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET thumbnail_path = ? WHERE id = ?")
                .bind(db_thumbnail)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_tags == "[]" && db_tags_json != "[]" {
            sqlx::query("UPDATE objects SET tags = ? WHERE id = ?")
                .bind(db_tags_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_meta == "{}" && db_metadata_json != "{}" {
            sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
                .bind(db_metadata_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_hash.is_none() && db_hash_db_json.is_some() {
            sqlx::query("UPDATE objects SET hash_db = ? WHERE id = ?")
                .bind(db_hash_db_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_skins.is_none() && db_custom_skins_json.is_some() {
            sqlx::query("UPDATE objects SET custom_skins = ? WHERE id = ?")
                .bind(db_custom_skins_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        return Ok(id);
    } else if let Some(row) = match_folder {
        let id: String = row.try_get("id").unwrap_or_default();
        let existing_fp: String = row.try_get("folder_path").unwrap_or_default();
        let existing_thumb: Option<String> = row.try_get("thumbnail_path").unwrap_or(None);
        let existing_tags: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        let existing_meta: String = row.try_get("metadata").unwrap_or_else(|_| "{}".to_string());
        let existing_hash: Option<String> = row.try_get("hash_db").unwrap_or(None);
        let existing_skins: Option<String> = row.try_get("custom_skins").unwrap_or(None);

        if existing_fp != folder_path {
            sqlx::query("UPDATE objects SET folder_path = ?, folder_path_key = ? WHERE id = ?")
                .bind(folder_path)
                .bind(&folder_key)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET name = ?, name_key = ?, object_type = ? WHERE id = ?")
                .bind(obj_name)
                .bind(&name_key)
                .bind(obj_type)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            sqlx::query("UPDATE objects SET name = ?, name_key = ? WHERE id = ?")
                .bind(obj_name)
                .bind(&name_key)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_thumb.is_none() && db_thumbnail.is_some() {
            sqlx::query("UPDATE objects SET thumbnail_path = ? WHERE id = ?")
                .bind(db_thumbnail)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_tags == "[]" && db_tags_json != "[]" {
            sqlx::query("UPDATE objects SET tags = ? WHERE id = ?")
                .bind(db_tags_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_meta == "{}" && db_metadata_json != "{}" {
            sqlx::query("UPDATE objects SET metadata = ? WHERE id = ?")
                .bind(db_metadata_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_hash.is_none() && db_hash_db_json.is_some() {
            sqlx::query("UPDATE objects SET hash_db = ? WHERE id = ?")
                .bind(db_hash_db_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        if existing_skins.is_none() && db_custom_skins_json.is_some() {
            sqlx::query("UPDATE objects SET custom_skins = ? WHERE id = ?")
                .bind(db_custom_skins_json)
                .bind(&id)
                .execute(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
        }

        return Ok(id);
    }

    let new_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, name_key, folder_path, folder_path_key, object_type, thumbnail_path, tags, metadata, hash_db, custom_skins, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)"
    )
    .bind(&new_id)
    .bind(game_id)
    .bind(obj_name)
    .bind(&name_key)
    .bind(folder_path)
    .bind(&folder_key)
    .bind(obj_type)
    .bind(db_thumbnail)
    .bind(db_tags_json)
    .bind(db_metadata_json)
    .bind(db_hash_db_json)
    .bind(db_custom_skins_json)
    .execute(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;

    *new_objects_count += 1;
    Ok(new_id)
}

pub async fn delete_ghost_objects_gc(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM objects WHERE game_id = $1 AND NOT EXISTS (SELECT 1 FROM mods WHERE object_id = objects.id)"
    )
    .bind(game_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn get_object_name_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT name FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_matched_entry_key_by_id(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT matched_entry_key FROM objects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_object_folder_by_matched_entry_key<'c, E>(
    executor: E,
    game_id: &str,
    matched_entry_key: &str,
) -> Result<Option<String>, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query_scalar(
        "SELECT folder_path FROM objects
         WHERE game_id = ? AND matched_entry_key = ?
         ORDER BY updated_at DESC
         LIMIT 1",
    )
    .bind(game_id)
    .bind(matched_entry_key)
    .fetch_optional(executor)
    .await
}

pub async fn get_object_id_by_matched_entry_key<'c, E>(
    executor: E,
    game_id: &str,
    matched_entry_key: &str,
) -> Result<Option<String>, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query_scalar(
        "SELECT id FROM objects
         WHERE game_id = ? AND matched_entry_key = ?
         ORDER BY updated_at DESC
         LIMIT 1",
    )
    .bind(game_id)
    .bind(matched_entry_key)
    .fetch_optional(executor)
    .await
}

#[allow(clippy::too_many_arguments)] // Canonical match patch mirrors nullable DB columns at the repo boundary.
pub async fn apply_canonical_match<'c, E>(
    executor: E,
    object_id: &str,
    matched_entry_key: Option<&str>,
    matched_alias_name: Option<&str>,
    matched_confidence: Option<f64>,
    matched_reason: Option<&str>,
    matched_source: Option<&str>,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "UPDATE objects
         SET matched_entry_key = ?,
             matched_alias_name = ?,
             matched_confidence = ?,
             matched_reason = ?,
             matched_source = ?,
             matched_at = CASE WHEN ? IS NULL THEN matched_at ELSE CURRENT_TIMESTAMP END
         WHERE id = ?",
    )
    .bind(matched_entry_key)
    .bind(matched_alias_name)
    .bind(matched_confidence)
    .bind(matched_reason)
    .bind(matched_source)
    .bind(matched_entry_key)
    .bind(object_id)
    .execute(executor)
    .await?;
    Ok(())
}

/// Fetch all objects that could be relevant for KeyViewer matching (primarily Characters).
/// Returns a list of (Name, HashDb, CustomSkins).
pub async fn get_kv_matching_objects(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<
    Vec<(
        String,
        crate::database::models::HashDbPayload,
        crate::database::models::CustomSkinsPayload,
    )>,
    sqlx::Error,
> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT name, hash_db, custom_skins FROM objects WHERE game_id = ? AND object_type = 'Character'"
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        let name: String = row.try_get("name")?;
        let hash_db: crate::database::models::HashDbPayload = row.try_get("hash_db")?;
        let custom_skins: crate::database::models::CustomSkinsPayload =
            row.try_get("custom_skins")?;
        result.push((name, hash_db, custom_skins));
    }
    Ok(result)
}
