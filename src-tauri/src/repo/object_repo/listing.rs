use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::types::*;
use crate::common::normalizer::is_disabled_folder;
use crate::common::path_key::canonical_name_key;
use crate::domain::models::ItemStatus;
use crate::domain::objects::{ObjectFilter, ObjectSummary};
use crate::services::objects::terminal::split_segments;

pub async fn get_filtered_objects(
    pool: &SqlitePool,
    filter: &ObjectFilter,
) -> Result<ObjectPage, sqlx::Error> {
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
            let name_search_term = format!("%{}%", canonical_name_key(trimmed));
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
            append_metadata_filter_condition(&mut qb, key, values);
        }
    }

    match filter.sort_by.as_deref() {
        Some("date") => qb.push(" ORDER BY o.is_pinned DESC, o.created_at DESC"),
        Some("rarity") => qb.push(" ORDER BY o.is_pinned DESC, CAST(JSON_EXTRACT(o.metadata, '$.rarity') AS INTEGER) DESC, o.name ASC"),
        _ => qb.push(" ORDER BY o.is_pinned DESC, o.object_type, o.name ASC"),
    };

    let rows = qb
        .build_query_as::<ObjectSummaryRow>()
        .fetch_all(pool)
        .await?;
    let cold_ids: Vec<String> = rows
        .iter()
        .filter(|row| row.projection_available == 0)
        .map(|row| row.summary.id.clone())
        .collect();

    let objects: Vec<ObjectSummary> = rows
        .into_iter()
        .map(|row| {
            // A cold projection row carries zeroes; the caller fills the
            // counts in from disk. Disabled-ness is derivable from the path.
            let is_object_disabled = if row.projection_available == 0 {
                fallback_object_disabled(&row)
            } else {
                row.summary.is_object_disabled
            };

            ObjectSummary {
                is_object_disabled,
                ..row.summary
            }
        })
        .collect();

    Ok(ObjectPage {
        cold_object_ids: cold_ids,
        objects,
    })
}

/// Applies the caller's status filter once the counts are settled.
///
/// Split from the query because a cold projection is patched from disk in
/// between, and `is_object_disabled` decides the filter.
pub fn apply_status_filter(objects: &mut Vec<ObjectSummary>, status: Option<ItemStatus>) {
    let Some(status) = status else {
        return;
    };
    objects.retain(|object| match status {
        ItemStatus::Enabled => !object.is_object_disabled,
        ItemStatus::Disabled => object.is_object_disabled,
    });
}

pub(super) fn fallback_object_disabled(row: &ObjectSummaryRow) -> bool {
    row.summary.is_object_disabled
        || split_segments(&row.summary.folder_path)
            .iter()
            .any(|segment| is_disabled_folder(segment))
}

pub(super) fn append_metadata_filter_condition(
    qb: &mut QueryBuilder<'_, Sqlite>,
    key: &str,
    values: &[String],
) {
    if !is_valid_metadata_filter_key(key) {
        return;
    }

    let normalized_values = normalized_metadata_filter_values(values);
    if normalized_values.is_empty() {
        return;
    }

    let json_path = format!("$.{key}");
    qb.push(" AND (");
    qb.push("EXISTS (SELECT 1 FROM json_each(o.metadata, ");
    qb.push_bind(json_path.clone());
    qb.push(") WHERE json_valid(o.metadata) = 1 AND LOWER(CAST(json_each.value AS TEXT)) IN (");
    {
        let mut separated = qb.separated(", ");
        for value in &normalized_values {
            separated.push_bind(value.clone());
        }
    }
    qb.push("))");
    qb.push(" OR (json_valid(o.metadata) = 1 AND LOWER(CAST(JSON_EXTRACT(o.metadata, ");
    qb.push_bind(json_path);
    qb.push(") AS TEXT)) IN (");
    {
        let mut separated = qb.separated(", ");
        for value in &normalized_values {
            separated.push_bind(value.clone());
        }
    }
    qb.push(")))");
}

pub(super) fn is_valid_metadata_filter_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(super) fn normalized_metadata_filter_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}
