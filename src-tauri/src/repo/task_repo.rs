use crate::domain::errors::AppError;
use crate::domain::task::{PipelineTask, TaskStatus};
use sqlx::{Row, SqlitePool};

/// Columns every `PipelineTask` read selects, in the order `row_to_task` expects.
const TASK_COLUMNS: &str = "id, game_id, task_type, status, target_id, created_at, updated_at";

fn row_to_task(r: &sqlx::sqlite::SqliteRow) -> PipelineTask {
    PipelineTask {
        id: r.get("id"),
        game_id: r.get("game_id"),
        task_type: r.get("task_type"),
        status: TaskStatus::from_db_value(r.get::<&str, _>("status")),
        target_id: r.try_get("target_id").ok().flatten(),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

/// Create a new pending task in the database and return its ID.
pub async fn create_task(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    task_type: &str,
    target_id: Option<&str>,
) -> Result<String, AppError> {
    sqlx::query(
        r#"
        INSERT INTO tasks (id, game_id, task_type, status, target_id)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(game_id)
    .bind(task_type)
    .bind(TaskStatus::Pending.as_str())
    .bind(target_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Db(e.to_string()))?;

    Ok(id.to_string())
}

/// Mark a task as completed or failed.
pub async fn update_status(
    pool: &SqlitePool,
    id: &str,
    status: TaskStatus,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE tasks 
        SET status = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(status.as_str())
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Db(e.to_string()))?;

    Ok(())
}

/// Get all PENDING tasks across all games (useful for crash recovery on boot).
pub async fn get_all_pending_tasks_global(
    pool: &SqlitePool,
) -> Result<Vec<PipelineTask>, AppError> {
    let rows = sqlx::query(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks WHERE status = ? ORDER BY created_at ASC"
    ))
    .bind(TaskStatus::Pending.as_str())
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Db(e.to_string()))?;

    Ok(rows.iter().map(row_to_task).collect())
}

/// Get a specific task by its ID.
pub async fn get_task_by_id(pool: &SqlitePool, id: &str) -> Result<Option<PipelineTask>, AppError> {
    let row = sqlx::query(&format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Db(e.to_string()))?;

    Ok(row.as_ref().map(row_to_task))
}

/// Drop old *settled* task rows. Returns the number of purged rows.
///
/// `PENDING` rows are the crash-recovery queue that `app_startup_check` reads,
/// so age alone must not delete them — a pending apply older than the retention
/// window would vanish before the user was ever offered recovery.
pub async fn purge_old_tasks(pool: &SqlitePool) -> Result<u64, AppError> {
    sqlx::query("DELETE FROM tasks WHERE status != ? AND created_at < datetime('now', '-7 days')")
        .bind(TaskStatus::Pending.as_str())
        .execute(pool)
        .await
        .map(|result| result.rows_affected())
        .map_err(|e| AppError::Db(e.to_string()))
}
