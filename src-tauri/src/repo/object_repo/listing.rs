use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::counts::{
    build_terminal_counts, load_game_mods_path, load_object_count_candidates, split_segments,
};
use super::types::*;
use crate::common::normalizer::is_disabled_folder;
use crate::common::path_key::object_name_key;
use crate::domain::models::ItemStatus;

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
            append_metadata_filter_condition(&mut qb, key, values);
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
                is_object_disabled: fallback_object_disabled(row),
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

        let _ = crate::repo::runtime_projection_repo::refresh_objects_projection(
            pool,
            &filter.game_id,
            &missing_projection_ids,
        )
        .await;
    }

    let mut objects: Vec<ObjectSummary> = rows
        .into_iter()
        .map(|row| {
            let is_object_disabled = if row.projection_available == 0 {
                fallback_object_disabled(&row)
            } else {
                row.is_object_disabled
            };

            ObjectSummary {
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
                is_object_disabled,
                has_naming_conflict: row.has_naming_conflict,
                active_mod_paths: row.active_mod_paths,
            }
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

pub(super) fn fallback_object_disabled(row: &ObjectSummaryRow) -> bool {
    row.is_object_disabled
        || split_segments(&row.folder_path)
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
