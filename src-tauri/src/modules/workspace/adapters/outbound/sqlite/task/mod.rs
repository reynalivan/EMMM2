use crate::shared::errors::AppError;
use crate::modules::workspace::domain::task::{PipelineTask, TaskStatus, TASK_TYPE_APPLY_COLLECTION};
use sqlx::{Row, SqliteConnection, SqlitePool};

/// Columns every `PipelineTask` read selects, in the order `row_to_task` expects.
const TASK_COLUMNS: &str = "id, game_id, task_type, status, target_id, rollback_collection_id, rollback_active_collection_id, final_active_collection_id, created_at, updated_at";

fn row_to_task(r: &sqlx::sqlite::SqliteRow) -> PipelineTask {
    PipelineTask {
        id: r.get("id"),
        game_id: r.get("game_id"),
        task_type: r.get("task_type"),
        status: TaskStatus::from_db_value(r.get::<&str, _>("status")),
        target_id: r.try_get("target_id").ok().flatten(),
        rollback_collection_id: r.try_get("rollback_collection_id").ok().flatten(),
        rollback_active_collection_id: r.try_get("rollback_active_collection_id").ok().flatten(),
        final_active_collection_id: r.try_get("final_active_collection_id").ok().flatten(),
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
    create_task_with_rollback_intent(pool, id, game_id, task_type, target_id, None, None).await
}

pub async fn create_task_with_rollback_intent(
    pool: &SqlitePool,
    task_id: &str,
    game_id: &str,
    task_type: &str,
    target_id: Option<&str>,
    rollback_collection_id: Option<&str>,
    rollback_active_collection_id: Option<&str>,
) -> Result<String, AppError> {
    let final_active_collection_id = if task_type == TASK_TYPE_APPLY_COLLECTION {
        target_id
    } else {
        None
    };
    create_task_with_full_intent(
        pool,
        task_id,
        game_id,
        task_type,
        target_id,
        rollback_collection_id,
        rollback_active_collection_id,
        final_active_collection_id,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn create_task_with_full_intent(
    pool: &SqlitePool,
    task_id: &str,
    game_id: &str,
    task_type: &str,
    target_id: Option<&str>,
    rollback_collection_id: Option<&str>,
    rollback_active_collection_id: Option<&str>,
    final_active_collection_id: Option<&str>,
) -> Result<String, AppError> {
    sqlx::query(
        r#"
        INSERT INTO tasks (
            id, game_id, task_type, status, target_id,
            rollback_collection_id, rollback_active_collection_id,
            final_active_collection_id
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(task_id)
    .bind(game_id)
    .bind(task_type)
    .bind(TaskStatus::Pending.as_str())
    .bind(target_id)
    .bind(rollback_collection_id)
    .bind(rollback_active_collection_id)
    .bind(final_active_collection_id)
    .execute(pool)
    .await
    .map_err(|error| {
        let message = error.to_string();
        if message.contains("UNIQUE constraint failed: tasks.game_id") {
            AppError::Validation(format!(
                "Game '{game_id}' already has an open collection apply task"
            ))
        } else {
            AppError::Db(message)
        }
    })?;

    Ok(task_id.to_string())
}

/// Create a normal collection apply already claimed by this process. The
/// pending row and its `RUNNING` claim commit together, so lock-free recovery
/// actions can never observe an actionable task owned by a live apply.
pub async fn create_claimed_task(
    pool: &SqlitePool,
    task_id: &str,
    game_id: &str,
    task_type: &str,
    target_id: Option<&str>,
) -> Result<String, AppError> {
    let final_active_collection_id = if task_type == TASK_TYPE_APPLY_COLLECTION {
        target_id
    } else {
        None
    };
    create_claimed_task_with_final_active(
        pool,
        task_id,
        game_id,
        task_type,
        target_id,
        final_active_collection_id,
    )
    .await
}

pub async fn create_claimed_task_with_final_active(
    pool: &SqlitePool,
    task_id: &str,
    game_id: &str,
    task_type: &str,
    target_id: Option<&str>,
    final_active_collection_id: Option<&str>,
) -> Result<String, AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Db(e.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO tasks (
            id, game_id, task_type, status, target_id, final_active_collection_id
        )
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(task_id)
    .bind(game_id)
    .bind(task_type)
    .bind(TaskStatus::Pending.as_str())
    .bind(target_id)
    .bind(final_active_collection_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| map_create_error(error, game_id))?;
    let claimed =
        compare_and_set_status_tx(&mut tx, task_id, TaskStatus::Pending, TaskStatus::Running)
            .await
            .map_err(|e| AppError::Db(e.to_string()))?;
    if !claimed {
        return Err(AppError::Validation(format!(
            "Task '{task_id}' could not be claimed"
        )));
    }
    tx.commit().await.map_err(|e| AppError::Db(e.to_string()))?;
    Ok(task_id.to_string())
}

/// Mark a task as completed or failed.
pub async fn update_status(
    pool: &SqlitePool,
    id: &str,
    status: TaskStatus,
) -> Result<(), AppError> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| AppError::Db(e.to_string()))?;
    update_status_tx(&mut conn, id, status)
        .await
        .map_err(|e| AppError::Db(e.to_string()))
}

pub async fn update_status_tx(
    conn: &mut SqliteConnection,
    id: &str,
    status: TaskStatus,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE tasks 
        SET status = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(status.as_str())
    .bind(id)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn update_rollback_intent(
    pool: &SqlitePool,
    id: &str,
    rollback_collection_id: Option<&str>,
    rollback_active_collection_id: Option<&str>,
    expected_status: TaskStatus,
) -> Result<(), AppError> {
    let result = sqlx::query(
        r#"
        UPDATE tasks
        SET rollback_collection_id = ?, rollback_active_collection_id = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ? AND status = ?
        "#,
    )
    .bind(rollback_collection_id)
    .bind(rollback_active_collection_id)
    .bind(id)
    .bind(expected_status.as_str())
    .execute(pool)
    .await
    .map_err(|e| AppError::Db(e.to_string()))?;

    if result.rows_affected() != 1 {
        return Err(AppError::Validation(format!(
            "Task '{id}' is no longer pending"
        )));
    }
    Ok(())
}

fn map_create_error(error: sqlx::Error, game_id: &str) -> AppError {
    let message = error.to_string();
    if message.contains("UNIQUE constraint failed: tasks.game_id") {
        AppError::Validation(format!(
            "Game '{game_id}' already has an open collection apply task"
        ))
    } else {
        AppError::Db(message)
    }
}

pub async fn compare_and_set_status(
    pool: &SqlitePool,
    id: &str,
    expected: TaskStatus,
    status: TaskStatus,
) -> Result<bool, AppError> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| AppError::Db(e.to_string()))?;
    compare_and_set_status_tx(&mut conn, id, expected, status)
        .await
        .map_err(|e| AppError::Db(e.to_string()))
}

pub async fn compare_and_set_status_tx(
    conn: &mut SqliteConnection,
    id: &str,
    expected: TaskStatus,
    status: TaskStatus,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE tasks
        SET status = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ? AND status = ?
        "#,
    )
    .bind(status.as_str())
    .bind(id)
    .bind(expected.as_str())
    .execute(&mut *conn)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn open_task_references_rollback_collection_tx(
    conn: &mut SqliteConnection,
    collection_id: &str,
) -> Result<bool, sqlx::Error> {
    let referenced: i64 = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM tasks
            WHERE rollback_collection_id = ? AND status IN (?, ?)
        )
        "#,
    )
    .bind(collection_id)
    .bind(TaskStatus::Pending.as_str())
    .bind(TaskStatus::Running.as_str())
    .fetch_one(&mut *conn)
    .await?;
    Ok(referenced != 0)
}

/// Get all open tasks across all games (useful for crash recovery on boot).
pub async fn get_all_pending_tasks_global(
    pool: &SqlitePool,
) -> Result<Vec<PipelineTask>, AppError> {
    let rows = sqlx::query(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks WHERE status IN (?, ?) ORDER BY created_at ASC"
    ))
    .bind(TaskStatus::Pending.as_str())
    .bind(TaskStatus::Running.as_str())
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Db(e.to_string()))?;

    Ok(rows.iter().map(row_to_task).collect())
}

/// A fresh single-instance app process has no live owner for a durable
/// `RUNNING` recovery claim. Release those interrupted apply claims once during
/// backend boot; repeatable frontend startup checks remain read-only.
pub async fn reclaim_interrupted_apply_tasks(pool: &SqlitePool) -> Result<u64, AppError> {
    sqlx::query(
        r#"
        UPDATE tasks
        SET status = ?, updated_at = CURRENT_TIMESTAMP
        WHERE task_type = ? AND status = ?
        "#,
    )
    .bind(TaskStatus::Pending.as_str())
    .bind(TASK_TYPE_APPLY_COLLECTION)
    .bind(TaskStatus::Running.as_str())
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
    .map_err(|e| AppError::Db(e.to_string()))
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
/// Open rows are the crash-recovery queue that `app_startup_check` reads,
/// so age alone must not delete them — an interrupted apply older than the retention
/// window would vanish before the user was ever offered recovery.
pub async fn purge_old_tasks(pool: &SqlitePool) -> Result<u64, AppError> {
    sqlx::query(
        "DELETE FROM tasks WHERE status NOT IN (?, ?) AND created_at < datetime('now', '-7 days')",
    )
    .bind(TaskStatus::Pending.as_str())
    .bind(TaskStatus::Running.as_str())
    .execute(pool)
    .await
    .map(|result| result.rows_affected())
    .map_err(|e| AppError::Db(e.to_string()))
}
