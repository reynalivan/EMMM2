use super::*;
use crate::modules::collections::domain::collection::{
    CreateCollectionInput, CreateCollectionMode,
};

async fn save_live(
    f: &Fixture,
    name: &str,
) -> crate::modules::collections::domain::collection::CollectionSummary {
    super::super::super::create_collection(
        &f.db.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: name.to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn saving_a_new_baseline_in_safe_mode_does_not_restore_the_previous_unsafe_selection() {
    let f = fixture().await;
    let on = prepare_safe_mode_transition(&f.db.pool, "game-1", false)
        .await
        .unwrap();
    execute(&f, &on).await.unwrap();
    let saved = save_live(&f, "New safe baseline").await;
    let off = prepare_safe_mode_transition(&f.db.pool, "game-1", true)
        .await
        .unwrap();
    execute(&f, &off).await.unwrap();
    assert!(f.root.path().join("AINOZ/DISABLED Private").is_dir());
    assert_eq!(
        storage::runtime::get(&f.db.pool, "game-1")
            .await
            .unwrap()
            .unwrap()
            .active_collection_id,
        Some(saved.id)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn replacing_the_active_baseline_in_safe_mode_updates_the_restore_selection() {
    let f = fixture().await;
    let saved = save_live(&f, "Baseline").await;
    let on = prepare_safe_mode_transition(&f.db.pool, "game-1", false)
        .await
        .unwrap();
    execute(&f, &on).await.unwrap();
    super::super::super::replace_collection_with_current_state(&f.db.pool, "game-1", &saved.id)
        .await
        .unwrap();
    let off = prepare_safe_mode_transition(&f.db.pool, "game-1", true)
        .await
        .unwrap();
    execute(&f, &off).await.unwrap();
    assert!(f.root.path().join("AINOZ/DISABLED Private").is_dir());
    assert!(!f.root.path().join("AINOZ/Private").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_safe_member_reclassified_unsafe_is_excluded_from_safe_apply_preview_and_execution()
{
    let f = fixture().await;
    let saved = save_live(&f, "Baseline").await;
    sqlx::query("UPDATE mods SET is_safe = 0 WHERE id = 'extra'")
        .execute(&f.db.pool)
        .await
        .unwrap();
    let preview = super::super::super::preview_apply(
        &f.db.pool,
        "game-1",
        &saved.id,
        Some(&f.root.path().to_string_lossy()),
        true,
    )
    .await
    .unwrap();
    assert!(preview
        .effective_target_projected_state
        .active_roots
        .is_empty());
    let mut settings = f.config.get_settings();
    settings
        .safety
        .set_runtime_safe_mode("game-1".to_string(), true);
    super::super::super::apply_collection(super::super::super::ApplyCollectionRequest {
        pool: &f.db.pool,
        game_id: "game-1",
        collection_id: &saved.id,
        capture_last_changes: true,
        mods_path: f.root.path().to_path_buf(),
        suppressor: Arc::new(WatcherSuppressor::new(false)),
        ignore_missing: false,
        settings,
    })
    .await
    .unwrap();
    assert!(f.root.path().join("AINOZ/DISABLED Extra").is_dir());
    assert!(f.root.path().join("AINOZ/DISABLED Private").is_dir());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_queued_safe_mode_transition_revalidates_classification_before_enabling_the_filter() {
    let f = fixture().await;
    let on = prepare_safe_mode_transition(&f.db.pool, "game-1", false)
        .await
        .unwrap();
    sqlx::query("UPDATE mods SET is_safe = 0 WHERE id = 'extra'")
        .execute(&f.db.pool)
        .await
        .unwrap();
    execute(&f, &on).await.unwrap();
    assert!(f.root.path().join("AINOZ/DISABLED Extra").is_dir());
    assert!(f
        .config
        .get_settings()
        .safety
        .runtime_safe_mode_for("game-1"));
    let off = prepare_safe_mode_transition(&f.db.pool, "game-1", true)
        .await
        .unwrap();
    execute(&f, &off).await.unwrap();
    assert!(f.root.path().join("AINOZ/Extra").is_dir());
}
