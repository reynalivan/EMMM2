use crate::domain::errors::AppError;
use crate::domain::task::{PipelineTask, TaskStatus};
use sqlx::{Row, SqlitePool};

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
        VALUES (?, ?, ?, 'PENDING', ?)
        "#,
    )
    .bind(id)
    .bind(game_id)
    .bind(task_type)
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

/// Get all PENDING tasks (useful for crash recovery on boot).
pub async fn get_pending_tasks(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<PipelineTask>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, game_id, task_type, status, target_id, created_at, updated_at
        FROM tasks
        WHERE game_id = ? AND status = 'PENDING'
        ORDER BY created_at ASC
        "#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Db(e.to_string()))?;

    let tasks = rows
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| PipelineTask {
            id: r.get("id"),
            game_id: r.get("game_id"),
            task_type: r.get("task_type"),
            status: TaskStatus::from_db_value(r.get::<&str, _>("status")),
            target_id: r.try_get("target_id").ok().flatten(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect();

    Ok(tasks)
}

/// Get all PENDING tasks across all games (useful for crash recovery on boot).
pub async fn get_all_pending_tasks_global(
    pool: &SqlitePool,
) -> Result<Vec<PipelineTask>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, game_id, task_type, status, target_id, created_at, updated_at
        FROM tasks
        WHERE status = 'PENDING'
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Db(e.to_string()))?;

    let tasks = rows
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| PipelineTask {
            id: r.get("id"),
            game_id: r.get("game_id"),
            task_type: r.get("task_type"),
            status: TaskStatus::from_db_value(r.get::<&str, _>("status")),
            target_id: r.try_get("target_id").ok().flatten(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect();

    Ok(tasks)
}

/// Get a specific task by its ID.
pub async fn get_task_by_id(pool: &SqlitePool, id: &str) -> Result<Option<PipelineTask>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT id, game_id, task_type, status, target_id, created_at, updated_at
        FROM tasks
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Db(e.to_string()))?;

    if let Some(r) = row {
        Ok(Some(PipelineTask {
            id: r.get("id"),
            game_id: r.get("game_id"),
            task_type: r.get("task_type"),
            status: TaskStatus::from_db_value(r.get::<&str, _>("status")),
            target_id: r.try_get("target_id").ok().flatten(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    } else {
        Ok(None)
    }
}
