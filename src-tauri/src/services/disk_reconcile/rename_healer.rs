use std::path::Path;

use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::models::ItemStatus;
use crate::services::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::services::disk_reconcile::helpers::{
    generate_stable_mod_id, is_disabled_runtime_name, load_runtime_mod_metadata,
    normalize_runtime_name,
};
use crate::services::disk_reconcile::path_updates::push_path_update;
use crate::services::disk_reconcile::types::{DiskReconcilePathKind, DiskReconcilePathUpdate};
use crate::services::disk_reconcile::watcher_batch::{collect_rename_hints, WatcherRenameHints};
use crate::services::scanner::watcher::ModWatchEvent;

async fn load_object_type(
    conn: &mut sqlx::SqliteConnection,
    object_id: &str,
) -> Result<String, String> {
    crate::repo::object_repo::get_object_type_by_id(conn, object_id)
        .await
        .map_err(|error| error.to_string())?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Disk Reconcile object type missing for object '{object_id}'"))
}

async fn load_existing_manual_safe(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    folder_path: &str,
    mods_path: &str,
) -> Result<Option<bool>, String> {
    crate::repo::mod_repo::get_manual_is_safe_by_key(
        conn,
        game_id,
        &crate::common::path_key::folder_path_key(folder_path, Some(mods_path)),
    )
    .await
    .map_err(|error| error.to_string())
}

struct ModRenameHintsRequest<'a> {
    game_id: &'a str,
    mods_path: &'a Path,
    mods_root: &'a str,
    safe_mode_keywords: &'a [String],
    hints: &'a WatcherRenameHints,
    path_updates: &'a mut Vec<DiskReconcilePathUpdate>,
    collection_reference_impact: &'a mut CollectionReferenceImpact,
    change_summary: &'a mut ChangeSummaryBuilder,
}

async fn apply_mod_rename_hints(
    conn: &mut sqlx::SqliteConnection,
    request: ModRenameHintsRequest<'_>,
) -> Result<(), String> {
    for (old_relative, new_relative) in &request.hints.mod_renames {
        let mod_exists = crate::repo::mod_repo::get_mod_id_and_status_by_path(
            &mut *conn,
            old_relative,
            request.game_id,
        )
        .await
        .map_err(|error| error.to_string())?;
        let Some((old_id, _object_id, _status)) = mod_exists else {
            continue;
        };

        let components = Path::new(new_relative).components().collect::<Vec<_>>();
        if components.len() != 2 {
            continue;
        }

        let object_folder = components[0].as_os_str().to_string_lossy().to_string();
        let mod_folder = components[1].as_os_str().to_string_lossy().to_string();
        let object_name = normalize_runtime_name(&object_folder);
        let mut new_objects_count = 0usize;
        let object_id = crate::repo::object_repo::ensure_object_exists(
            &mut *conn,
            crate::repo::object_repo::EnsureObjectInput {
                game_id: request.game_id,
                folder_path: &object_folder,
                obj_name: &object_name,
                obj_type: "Other",
                db_thumbnail: None,
                db_tags_json: "[]",
                db_metadata_json: "{}",
                db_hash_db_json: None,
                db_custom_skins_json: None,
            },
            &mut new_objects_count,
        )
        .await
        .map_err(|e| e.to_string())?;
        let object_type = load_object_type(&mut *conn, &object_id).await?;
        let existing_manual_safe =
            load_existing_manual_safe(&mut *conn, request.game_id, old_relative, request.mods_root)
                .await?;
        let metadata = load_runtime_mod_metadata(
            &request.mods_path.join(new_relative),
            &mod_folder,
            is_disabled_runtime_name(&object_folder),
            request.safe_mode_keywords,
            existing_manual_safe,
        );
        let new_id = generate_stable_mod_id(request.game_id, new_relative);

        crate::repo::mod_repo::update_mod_identity_tx(
            &mut *conn,
            &new_id,
            new_relative,
            &metadata.actual_name,
            metadata.status,
            metadata.is_safe,
            metadata.corridor_source,
            &old_id,
            Some(request.mods_root),
        )
        .await
        .map_err(|error| error.to_string())?;

        crate::repo::mod_repo::update_mod_object_id_and_type_tx(
            &mut *conn,
            &new_id,
            &object_id,
            &object_type,
        )
        .await
        .map_err(|error| error.to_string())?;

        let impact = crate::services::collection_service::handle_mod_moved_or_renamed_tx(
            &mut *conn,
            old_relative,
            new_relative,
            Some(&object_id),
        )
        .await
        .map_err(|error| format!("Failed to heal mod rename in collections: {error}"))?;
        request.collection_reference_impact.merge(impact);

        push_path_update(
            &mut *request.path_updates,
            DiskReconcilePathKind::Mod,
            old_relative,
            new_relative,
        );
        request
            .change_summary
            .record_mod_renamed(&metadata.actual_name);
    }

    Ok(())
}

async fn apply_object_rename_hints(
    conn: &mut sqlx::SqliteConnection,
    game_id: &str,
    mods_root: &str,
    hints: &WatcherRenameHints,
    path_updates: &mut Vec<DiskReconcilePathUpdate>,
    collection_reference_impact: &mut CollectionReferenceImpact,
    change_summary: &mut ChangeSummaryBuilder,
) -> Result<(), String> {
    for (old_folder, new_folder) in &hints.object_renames {
        let next_status = ItemStatus::from_is_disabled(is_disabled_runtime_name(new_folder));
        crate::repo::object_repo::update_object_runtime_state_by_path(
            &mut *conn,
            game_id,
            old_folder,
            new_folder,
            next_status,
        )
        .await
        .map_err(|error| format!("Failed to update object folder path: {error}"))?;

        for (old_sep, new_sep) in [
            (format!("{old_folder}\\"), format!("{new_folder}\\")),
            (format!("{old_folder}/"), format!("{new_folder}/")),
        ] {
            crate::repo::mod_repo::update_child_paths_tx(
                &mut *conn,
                game_id,
                &old_sep,
                &new_sep,
                Some(mods_root),
            )
            .await
            .map_err(|error| format!("Failed to update child paths: {error}"))?;
        }

        let impact = crate::services::collection_service::handle_object_renamed_tx(
            &mut *conn, old_folder, new_folder,
        )
        .await
        .map_err(|error| format!("Failed to heal object rename in collections: {error}"))?;
        collection_reference_impact.merge(impact);

        push_path_update(
            path_updates,
            DiskReconcilePathKind::Object,
            old_folder,
            new_folder,
        );
        change_summary.record_object_renamed(&normalize_runtime_name(new_folder));
    }

    Ok(())
}

pub(crate) struct WatcherRenameHintsApplyRequest<'a> {
    pub conn: &'a mut sqlx::SqliteConnection,
    pub game_id: &'a str,
    pub mods_path: &'a Path,
    pub safe_mode_keywords: &'a [String],
    pub watcher_events: &'a [ModWatchEvent],
    pub path_updates: &'a mut Vec<DiskReconcilePathUpdate>,
    pub collection_reference_impact: &'a mut CollectionReferenceImpact,
    pub change_summary: &'a mut ChangeSummaryBuilder,
}

pub(crate) async fn apply_watcher_rename_hints(
    request: WatcherRenameHintsApplyRequest<'_>,
) -> Result<(), String> {
    let hints = collect_rename_hints(request.mods_path, request.watcher_events);
    if hints.mod_renames.is_empty() && hints.object_renames.is_empty() {
        return Ok(());
    }

    let mods_root = request.mods_path.to_string_lossy().to_string();
    apply_mod_rename_hints(
        &mut *request.conn,
        ModRenameHintsRequest {
            game_id: request.game_id,
            mods_path: request.mods_path,
            mods_root: &mods_root,
            safe_mode_keywords: request.safe_mode_keywords,
            hints: &hints,
            path_updates: &mut *request.path_updates,
            collection_reference_impact: &mut *request.collection_reference_impact,
            change_summary: &mut *request.change_summary,
        },
    )
    .await?;
    apply_object_rename_hints(
        &mut *request.conn,
        request.game_id,
        &mods_root,
        &hints,
        &mut *request.path_updates,
        &mut *request.collection_reference_impact,
        &mut *request.change_summary,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::domain::models::{GameType, ItemStatus};
    use crate::test_utils::{
        init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
        TestModFixture, TestObjectFixture,
    };

    async fn seed_game_object_mod(
        pool: &sqlx::SqlitePool,
        mods_path: &Path,
        mod_folder_path: &str,
    ) -> String {
        let mods_path_string = mods_path.to_string_lossy().to_string();
        insert_test_game(
            pool,
            &TestGameFixture {
                id: "game-1",
                name: "Game",
                game_type: GameType::GIMI,
                path: mods_path.parent().unwrap().to_string_lossy().as_ref(),
                mods_path: Some(&mods_path_string),
            },
        )
        .await
        .expect("game seed");
        insert_test_object(
            pool,
            &TestObjectFixture {
                id: "obj-alice",
                game_id: "game-1",
                name: "Alice",
                folder_path: "Alice",
                object_type: "Character",
            },
        )
        .await
        .expect("object seed");
        insert_test_mod(
            pool,
            &TestModFixture {
                id: "mod-old",
                game_id: "game-1",
                object_id: Some("obj-alice"),
                actual_name: "Old Mod",
                folder_path: mod_folder_path,
                status: ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: Some(&mods_path_string),
            },
        )
        .await
        .expect("mod seed");
        mods_path_string
    }

    struct HealerRun {
        path_updates: Vec<DiskReconcilePathUpdate>,
    }

    async fn run_healer(
        pool: &sqlx::SqlitePool,
        mods_path: &Path,
        events: &[ModWatchEvent],
    ) -> HealerRun {
        let mut path_updates = Vec::new();
        let mut impact = CollectionReferenceImpact::default();
        let mut change_summary = ChangeSummaryBuilder::default();

        let mut tx = pool.begin().await.expect("tx begin");
        apply_watcher_rename_hints(WatcherRenameHintsApplyRequest {
            conn: &mut tx,
            game_id: "game-1",
            mods_path,
            safe_mode_keywords: &[],
            watcher_events: events,
            path_updates: &mut path_updates,
            collection_reference_impact: &mut impact,
            change_summary: &mut change_summary,
        })
        .await
        .expect("healer should succeed");
        tx.commit().await.expect("tx commit");

        HealerRun { path_updates }
    }

    #[tokio::test]
    async fn mod_rename_hint_rewrites_row_identity_and_path() {
        let ctx = init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        std::fs::create_dir_all(&mods_path).expect("mods root");
        seed_game_object_mod(&ctx.pool, &mods_path, "Alice/Old Mod").await;

        let events = vec![ModWatchEvent::Renamed {
            from: mods_path
                .join("Alice")
                .join("Old Mod")
                .to_string_lossy()
                .to_string(),
            to: mods_path
                .join("Alice")
                .join("New Mod")
                .to_string_lossy()
                .to_string(),
        }];
        let run = run_healer(&ctx.pool, &mods_path, &events).await;

        let expected_rel = PathBuf::from("Alice")
            .join("New Mod")
            .to_string_lossy()
            .to_string();
        let mod_row: (String, String, String, Option<String>, i64) = sqlx::query_as(
            "SELECT id, folder_path, actual_name, object_id, status FROM mods WHERE game_id = ?",
        )
        .bind("game-1")
        .fetch_one(&ctx.pool)
        .await
        .expect("mod row");
        assert_eq!(mod_row.0, generate_stable_mod_id("game-1", &expected_rel));
        assert_eq!(mod_row.1, expected_rel);
        assert_eq!(mod_row.2, "New Mod");
        assert_eq!(mod_row.3.as_deref(), Some("obj-alice"));
        assert_eq!(mod_row.4, ItemStatus::Enabled as i64);

        assert_eq!(run.path_updates.len(), 1);
        assert_eq!(run.path_updates[0].kind, DiskReconcilePathKind::Mod);
        assert_eq!(run.path_updates[0].to, expected_rel);
    }

    #[tokio::test]
    async fn object_rename_hint_updates_object_status_and_child_mod_paths() {
        let ctx = init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        std::fs::create_dir_all(&mods_path).expect("mods root");
        seed_game_object_mod(&ctx.pool, &mods_path, "Alice/Blue").await;

        let events = vec![ModWatchEvent::Renamed {
            from: mods_path.join("Alice").to_string_lossy().to_string(),
            to: mods_path
                .join("DISABLED Alice")
                .to_string_lossy()
                .to_string(),
        }];
        let run = run_healer(&ctx.pool, &mods_path, &events).await;

        let object_row: (String, i64) =
            sqlx::query_as("SELECT folder_path, status FROM objects WHERE game_id = ?")
                .bind("game-1")
                .fetch_one(&ctx.pool)
                .await
                .expect("object row");
        assert_eq!(object_row.0, "DISABLED Alice");
        assert_eq!(object_row.1, ItemStatus::Disabled as i64);

        let mod_folder_path: String =
            sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ?")
                .bind("game-1")
                .fetch_one(&ctx.pool)
                .await
                .expect("mod row");
        assert_eq!(mod_folder_path, "DISABLED Alice/Blue");

        assert_eq!(run.path_updates.len(), 1);
        assert_eq!(run.path_updates[0].kind, DiskReconcilePathKind::Object);
        assert_eq!(run.path_updates[0].from, "Alice");
        assert_eq!(run.path_updates[0].to, "DISABLED Alice");
    }

    #[tokio::test]
    async fn events_without_rename_hints_are_a_no_op() {
        let ctx = init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let mods_path = temp.path().join("Mods");
        std::fs::create_dir_all(&mods_path).expect("mods root");
        seed_game_object_mod(&ctx.pool, &mods_path, "Alice/Blue").await;

        let outside_root = temp.path().join("Elsewhere");
        let events = vec![
            ModWatchEvent::Created(
                mods_path
                    .join("Alice")
                    .join("Fresh")
                    .to_string_lossy()
                    .to_string(),
            ),
            // Rename outside the mods root must be ignored.
            ModWatchEvent::Renamed {
                from: outside_root.join("Alice").to_string_lossy().to_string(),
                to: outside_root.join("Bob").to_string_lossy().to_string(),
            },
        ];
        let run = run_healer(&ctx.pool, &mods_path, &events).await;

        assert!(run.path_updates.is_empty());
        let mod_folder_path: String =
            sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ?")
                .bind("game-1")
                .fetch_one(&ctx.pool)
                .await
                .expect("mod row");
        assert_eq!(mod_folder_path, "Alice/Blue");
    }
}
