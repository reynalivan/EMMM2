use crate::services::collections::types::{CollectionStateKind, CorridorRuntimeSnapshot};
use sqlx::SqlitePool;

fn state_kind_label(kind: CollectionStateKind) -> &'static str {
    match kind {
        CollectionStateKind::Named => "named",
        CollectionStateKind::Unsaved => "unsaved",
        CollectionStateKind::None => "none",
    }
}

pub async fn upsert_runtime_snapshot(
    pool: &SqlitePool,
    snapshot: &CorridorRuntimeSnapshot,
) -> Result<(), sqlx::Error> {
    let snapshot_json = serde_json::to_string(snapshot).map_err(|error| {
        sqlx::Error::Protocol(format!(
            "Failed to serialize corridor runtime snapshot: {error}"
        ))
    })?;

    sqlx::query(
        r#"
        INSERT INTO corridor_runtime_cache (
            game_id,
            is_safe,
            matched_collection_id,
            state_kind,
            state_name,
            signature,
            snapshot_json,
            snapshot_source,
            updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(game_id, is_safe) DO UPDATE SET
            matched_collection_id = excluded.matched_collection_id,
            state_kind = excluded.state_kind,
            state_name = excluded.state_name,
            signature = excluded.signature,
            snapshot_json = excluded.snapshot_json,
            snapshot_source = excluded.snapshot_source,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&snapshot.game_id)
    .bind(snapshot.is_safe)
    .bind(&snapshot.active_collection_id)
    .bind(state_kind_label(snapshot.state_kind))
    .bind(&snapshot.state_name)
    .bind(&snapshot.signature)
    .bind(snapshot_json)
    .bind(&snapshot.snapshot_source)
    .execute(pool)
    .await?;

    Ok(())
}
