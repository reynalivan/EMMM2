use sqlx::{Row, SqliteConnection, SqlitePool};

use crate::modules::collections::domain::collection::ProjectedCollectionState;
use crate::shared::errors::CollectionError;

#[derive(Clone)]
pub struct SafeModeIntent {
    pub previous_enabled: bool,
    pub target_enabled: bool,
    pub target: ProjectedCollectionState,
    pub rollback: ProjectedCollectionState,
    pub previous_restore: Option<ProjectedCollectionState>,
    pub rollback_requested: bool,
}

pub async fn get_snapshot(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Option<ProjectedCollectionState>, CollectionError> {
    let json: Option<String> =
        sqlx::query_scalar("SELECT snapshot_json FROM safe_mode_snapshots WHERE game_id = ?")
            .bind(game_id)
            .fetch_optional(pool)
            .await?;
    json.as_deref().map(parse).transpose()
}

pub async fn get_snapshot_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<Option<ProjectedCollectionState>, CollectionError> {
    let json: Option<String> =
        sqlx::query_scalar("SELECT snapshot_json FROM safe_mode_snapshots WHERE game_id = ?")
            .bind(game_id)
            .fetch_optional(&mut *conn)
            .await?;
    json.as_deref().map(parse).transpose()
}

pub async fn get_game_intents_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
) -> Result<Vec<(String, SafeModeIntent)>, CollectionError> {
    let rows = sqlx::query(
        "SELECT i.task_id, i.previous_enabled, i.target_enabled, i.target_snapshot_json, \
         i.rollback_snapshot_json, i.previous_restore_json, i.rollback_requested \
         FROM safe_mode_task_intents i JOIN tasks t ON t.id = i.task_id \
         WHERE t.game_id = ? AND t.status IN ('PENDING', 'RUNNING')",
    )
    .bind(game_id)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter()
        .map(|row| Ok((row.try_get("task_id")?, map_intent(row)?)))
        .collect()
}

pub async fn update_intent_snapshots_tx(
    conn: &mut SqliteConnection,
    task_id: &str,
    intent: &SafeModeIntent,
) -> Result<(), CollectionError> {
    sqlx::query("UPDATE safe_mode_task_intents SET target_snapshot_json = ?, rollback_snapshot_json = ?, previous_restore_json = ? WHERE task_id = ?")
        .bind(serialize(&intent.target)?).bind(serialize(&intent.rollback)?)
        .bind(intent.previous_restore.as_ref().map(serialize).transpose()?)
        .bind(task_id).execute(&mut *conn).await?;
    Ok(())
}

pub async fn set_snapshot_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    snapshot: Option<&ProjectedCollectionState>,
) -> Result<(), CollectionError> {
    let Some(snapshot) = snapshot else {
        sqlx::query("DELETE FROM safe_mode_snapshots WHERE game_id = ?")
            .bind(game_id)
            .execute(&mut *conn)
            .await?;
        return Ok(());
    };
    sqlx::query(
        "INSERT INTO safe_mode_snapshots (game_id, snapshot_json) VALUES (?, ?) \
         ON CONFLICT(game_id) DO UPDATE SET snapshot_json = excluded.snapshot_json",
    )
    .bind(game_id)
    .bind(serialize(snapshot)?)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn insert_intent_tx(
    conn: &mut SqliteConnection,
    task_id: &str,
    intent: &SafeModeIntent,
) -> Result<(), CollectionError> {
    sqlx::query(
        "INSERT INTO safe_mode_task_intents (task_id, previous_enabled, target_enabled, \
         target_snapshot_json, rollback_snapshot_json, previous_restore_json) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(task_id)
    .bind(intent.previous_enabled)
    .bind(intent.target_enabled)
    .bind(serialize(&intent.target)?)
    .bind(serialize(&intent.rollback)?)
    .bind(
        intent
            .previous_restore
            .as_ref()
            .map(serialize)
            .transpose()?,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn get_intent(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<Option<SafeModeIntent>, CollectionError> {
    let row = sqlx::query(
        "SELECT previous_enabled, target_enabled, target_snapshot_json, \
         rollback_snapshot_json, previous_restore_json, rollback_requested \
         FROM safe_mode_task_intents WHERE task_id = ?",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(map_intent(&row)?))
}

fn map_intent(row: &sqlx::sqlite::SqliteRow) -> Result<SafeModeIntent, CollectionError> {
    let previous_restore: Option<String> = row.try_get("previous_restore_json")?;
    Ok(SafeModeIntent {
        previous_enabled: row.try_get("previous_enabled")?,
        target_enabled: row.try_get("target_enabled")?,
        target: parse(row.try_get("target_snapshot_json")?)?,
        rollback: parse(row.try_get("rollback_snapshot_json")?)?,
        previous_restore: previous_restore.as_deref().map(parse).transpose()?,
        rollback_requested: row.try_get("rollback_requested")?,
    })
}

pub async fn request_rollback(pool: &SqlitePool, task_id: &str) -> Result<(), CollectionError> {
    let updated =
        sqlx::query("UPDATE safe_mode_task_intents SET rollback_requested = 1 WHERE task_id = ?")
            .bind(task_id)
            .execute(pool)
            .await?;
    if updated.rows_affected() != 1 {
        return Err(CollectionError::Validation(
            "Safe Mode intent is missing".to_string(),
        ));
    }
    Ok(())
}

fn parse(json: &str) -> Result<ProjectedCollectionState, CollectionError> {
    serde_json::from_str(json)
        .map_err(|error| CollectionError::Db(format!("Invalid Safe Mode snapshot: {error}")))
}

pub async fn refresh_snapshot_if_present_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    state: &ProjectedCollectionState,
) -> Result<(), CollectionError> {
    sqlx::query("UPDATE safe_mode_snapshots SET snapshot_json = ? WHERE game_id = ?")
        .bind(serialize(state)?)
        .bind(game_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

fn serialize(state: &ProjectedCollectionState) -> Result<String, CollectionError> {
    serde_json::to_string(state).map_err(|error| {
        CollectionError::Db(format!("Cannot serialize Safe Mode snapshot: {error}"))
    })
}
