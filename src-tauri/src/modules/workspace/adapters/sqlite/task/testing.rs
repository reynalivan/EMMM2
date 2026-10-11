use super::*;

#[tokio::test]
async fn task_log_retention_preserves_old_open_tasks_and_new_settled_tasks() {
    let context = crate::test_utils::init_test_db().await;
    let pool = &context.pool;
    for (id, status, old) in [
        ("old-pending", TaskStatus::Pending, true),
        ("old-running", TaskStatus::Running, true),
        ("old-completed", TaskStatus::Completed, true),
        ("old-failed", TaskStatus::Failed, true),
        ("new-completed", TaskStatus::Completed, false),
    ] {
        sqlx::query(
            "INSERT INTO games (id, name, game_type, path, mods_path) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(id)
        .bind(crate::modules::games::domain::models::GameType::Gimi as i64)
        .bind(format!("C:/Game/{id}"))
        .bind(format!("C:/Game/{id}/Mods"))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO tasks (id, game_id, task_type, status, created_at) VALUES (?, ?, ?, ?, CASE WHEN ? THEN datetime('now', '-20 days') ELSE CURRENT_TIMESTAMP END)")
            .bind(id).bind(id).bind(TASK_TYPE_APPLY_COLLECTION).bind(status.as_str()).bind(old)
            .execute(pool).await.unwrap();
    }
    assert_eq!(purge_old_tasks(pool).await.unwrap(), 2);
    let retained: Vec<String> = sqlx::query_scalar("SELECT id FROM tasks ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap();
    assert_eq!(retained, ["new-completed", "old-pending", "old-running"]);
    assert_eq!(purge_old_tasks(pool).await.unwrap(), 0);
}
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
