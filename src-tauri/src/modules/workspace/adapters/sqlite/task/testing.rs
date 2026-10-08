use super::*;
/// Create a new pending task in the database and return its ID.
#[cfg(test)]
pub async fn create_task(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    task_type: &str,
    target_id: Option<&str>,
) -> Result<String, AppError> {
    create_task_with_rollback_intent(pool, id, game_id, task_type, target_id, None, None).await
}

#[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
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

/// Mark a task as completed or failed.
#[cfg(test)]
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

#[cfg(test)]
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
