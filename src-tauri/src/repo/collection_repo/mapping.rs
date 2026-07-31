//! Row -> domain mapping helpers shared by the other submodules.

use crate::domain::collection::{Collection, CollectionSummary};

pub(super) fn row_to_collection(r: &sqlx::sqlite::SqliteRow) -> Collection {
    use sqlx::Row;
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
        display_mod_count: r.try_get("display_mod_count").unwrap_or(0),
        member_count: r.try_get("member_count_computed").ok(),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

pub(super) fn parse_warnings_json(raw: Option<String>) -> Vec<String> {
    let Some(raw_json) = raw else {
        return Vec::new();
    };

    serde_json::from_str::<Vec<String>>(&raw_json).unwrap_or_default()
}

pub(super) fn serialize_warnings_json(warnings: &[String]) -> String {
    serde_json::to_string(warnings).unwrap_or_else(|_| "[]".to_string())
}

pub fn to_summary(c: &Collection, active_collection_id: Option<&str>) -> CollectionSummary {
    CollectionSummary {
        id: c.id.clone(),
        name: c.name.clone(),
        is_safe: c.is_safe,
        is_unsaved: c.is_unsaved,
        signature: c.signature.clone(),
        is_active: active_collection_id == Some(c.id.as_str()),
        updated_at: c.updated_at.clone(),
        raw_member_count: c.member_count.unwrap_or(0),
        mod_count: c.display_mod_count,
    }
}
