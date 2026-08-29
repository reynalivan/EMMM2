use super::*;

#[tokio::test]
async fn rollback_expectations_distinguish_clean_modified_and_unsaved_state() {
    let (pool, _config, _watcher) = setup().await;
    for (id, name, is_unsaved) in [
        ("clean", "Clean", false),
        ("modified-baseline", "Modified baseline", false),
        ("modified-draft", "Modified Last changes", true),
    ] {
        create_empty_collection(&pool, id, name, is_unsaved).await;
    }
    let mut connection = pool.acquire().await.expect("acquire runtime connection");
    crate::repo::collection::runtime::set_active_tx(&mut connection, "g1", Some("clean"))
        .await
        .expect("set unrelated current baseline");
    drop(connection);

    let mut actual = Vec::new();
    for (task_id, rollback_collection_id, rollback_active_collection_id) in [
        ("rollback-clean", "clean", Some("clean")),
        (
            "rollback-modified",
            "modified-draft",
            Some("modified-baseline"),
        ),
        ("rollback-unsaved", "modified-draft", None),
    ] {
        crate::repo::task::create_task_with_rollback_intent(
            &pool,
            task_id,
            "g1",
            "apply_collection",
            Some("failed-target"),
            Some(rollback_collection_id),
            rollback_active_collection_id,
        )
        .await
        .expect("create rollback task");
        let task = crate::repo::task::get_task_by_id(&pool, task_id)
            .await
            .expect("load rollback task")
            .expect("rollback task exists");
        let rollback = resolve_rollback_target(&pool, &task)
            .await
            .expect("resolve stored rollback");
        actual.push((rollback.collection_id, rollback.active_baseline_id));
        crate::repo::task::update_status(&pool, task_id, TaskStatus::Completed)
            .await
            .expect("settle characterization task");
    }
    let expected = vec![
        ("clean".to_string(), Some("clean".to_string())),
        (
            "modified-draft".to_string(),
            Some("modified-baseline".to_string()),
        ),
        ("modified-draft".to_string(), None),
    ];
    assert_eq!(actual, expected);
}
