use crate::modules::mutation::api::MutationCoordinator;
use crate::modules::reconciliation::api::disk_reconcile::onboarding_session::OnboardingSnapshotLease;

pub(crate) async fn onboarding_snapshot_requires_repair(
    lease: &OnboardingSnapshotLease,
    coordinator: &MutationCoordinator,
    game_id: &str,
) -> bool {
    lease.observed_changes_during_apply().await
        || !lease.journal_allows_game_snapshot(coordinator, game_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::reconciliation::api::disk_reconcile::{
        onboarding_session::{ConsumedOnboardingSnapshot, OnboardingIndexingSessionStore},
        reconcile::{reconcile_disk_projection, ReconcileDiskProjectionRequest},
        types::{DiskReconcileReason, DiskReconcileStatus},
    };
    use crate::modules::settings::api::config::GameConfig;
    use crate::shared::path_key::folder_path_key;
    use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Condvar, Mutex};

    struct SdkDeliveryGate {
        open: Mutex<bool>,
        ready: Condvar,
    }

    struct PausedSdkDelivery(Arc<SdkDeliveryGate>);

    impl PausedSdkDelivery {
        fn new(gate: Arc<SdkDeliveryGate>) -> Self {
            *gate.open.lock().expect("SDK delivery gate") = false;
            Self(gate)
        }
    }

    impl Drop for PausedSdkDelivery {
        fn drop(&mut self) {
            *self.0.open.lock().expect("SDK delivery gate") = true;
            self.0.ready.notify_all();
        }
    }

    fn write_mod(root: &Path, object: &str, name: &str) {
        let path = root.join(object).join(name);
        std::fs::create_dir_all(&path).expect("native mod directory");
        std::fs::write(path.join("mod.ini"), "[TextureOverride]\nhash = abc\n")
            .expect("native mod INI");
        std::fs::write(path.join("mesh.buf"), "mesh").expect("native mod asset");
    }

    #[tokio::test]
    async fn late_external_changes_after_snapshot_apply_require_full_repair_without_journal_writes()
    {
        const GAME: &str = "activation-snapshot";
        let db = init_test_db().await;
        let temp = tempfile::tempdir().expect("fixture directory");
        let root = temp.path().join("Mods");
        let game_type = "GIMI".parse().expect("fixture game type");
        write_mod(&root, "Alice", "Blue");
        let mod_path = root.to_string_lossy().into_owned();
        insert_test_game(
            &db.pool,
            &TestGameFixture {
                id: GAME,
                name: "Snapshot fixture",
                game_type,
                path: temp.path().to_string_lossy().as_ref(),
                mods_path: Some(&mod_path),
            },
        )
        .await
        .expect("game fixture");
        let coordinator = MutationCoordinator::unconfigured();
        coordinator
            .configure(Arc::default())
            .expect("owned journal");
        let journal_revision = coordinator
            .current_journal_revision()
            .expect("journal revision");
        let game = GameConfig {
            id: GAME.to_string(),
            name: "Snapshot fixture".to_string(),
            game_type,
            instance_path: temp.path().to_path_buf(),
            mod_path: root.clone(),
            ready_to_move_path: None,
            launch_mode: Default::default(),
            game_exe: None,
            loader_exe: None,
            xxmi_launcher_exe: None,
            launch_args: None,
            warnings: Vec::new(),
        };
        let store = OnboardingIndexingSessionStore::new();
        let session = store
            .begin_with_progress_at_revision(vec![game], Some(journal_revision), |_| {})
            .await
            .expect("raw-watched onboarding snapshot");
        let (claimed, _claim_guard) = store
            .claim_for_activation(GAME)
            .expect("owned activation claim");
        let ConsumedOnboardingSnapshot::Snapshot(lease) =
            claimed.resolve().await.expect("snapshot resolution")
        else {
            panic!("unchanged initial discovery must remain applicable");
        };
        let sdk_events = Arc::new(AtomicUsize::new(0));
        let counted_events = Arc::clone(&sdk_events);
        let delivery_gate = Arc::new(SdkDeliveryGate {
            open: Mutex::new(true),
            ready: Condvar::new(),
        });
        let callback_gate = Arc::clone(&delivery_gate);
        let watcher_state = super::super::WatcherState::new();
        let resource_session = watcher_state.begin_session(&root);
        let (_sdk_watcher, _sdk_receiver) =
            super::super::watch_mod_directory_with_runtime_config_and_observer(
                &root,
                None,
                Arc::clone(&watcher_state.suppressor),
                resource_session,
                Some(Arc::new(move |_, _, _| {
                    let mut open = callback_gate.open.lock().expect("SDK delivery gate");
                    while !*open {
                        open = callback_gate.ready.wait(open).expect("SDK delivery gate");
                    }
                    counted_events.fetch_add(1, Ordering::AcqRel);
                })),
            )
            .expect("debounced workspace watcher");
        // Declared after the watcher so unwinding releases callback delivery
        // before watcher shutdown waits for its native thread.
        let paused_delivery = PausedSdkDelivery::new(delivery_gate);
        let delivered_before_apply = sdk_events.load(Ordering::Acquire);
        let _guard = coordinator
            .inner_lock()
            .acquire()
            .await
            .expect("projection guard");
        let request = |discovery| ReconcileDiskProjectionRequest {
            pool: &db.pool,
            game_id: GAME,
            mods_path: &root,
            safe_mode_keywords: &[],
            reason: &DiskReconcileReason::GameSwitched,
            changed_paths: &[],
            force_full: true,
            watcher_events: None,
            path_hints: &[],
            trusted_mutation_scope: false,
            progress_reporter: None,
            precomputed_discovery: discovery,
        };
        let applied = reconcile_disk_projection(request(Some(lease.discovery())))
            .await
            .expect("precomputed projection application");
        assert_eq!(applied.status, DiskReconcileStatus::Applied);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM mods WHERE game_id = ?")
                .bind(GAME)
                .fetch_one(&db.pool)
                .await
                .expect("snapshot rows"),
            1
        );

        write_mod(&root, "Bob", "New");
        std::fs::write(
            root.join("Alice/Blue/mod.ini"),
            "[TextureOverride]\nhash = changed\n",
        )
        .expect("external edit after snapshot application");
        assert_eq!(
            coordinator
                .current_journal_revision()
                .expect("unchanged journal"),
            journal_revision
        );
        assert!(lease.journal_allows_game_snapshot(&coordinator, GAME));
        assert_eq!(
            sdk_events.load(Ordering::Acquire),
            delivered_before_apply,
            "workspace application delivery stays paused during the raw-lease check"
        );
        let needs_repair = onboarding_snapshot_requires_repair(&lease, &coordinator, GAME).await;
        if !needs_repair {
            assert!(
                lease.observed_changes_during_apply().await,
                "raw native watcher must observe the external write"
            );
        }
        if needs_repair {
            let repaired = reconcile_disk_projection(request(None))
                .await
                .expect("full fallback repair");
            assert_eq!(repaired.status, DiskReconcileStatus::Applied);
        }
        let mut rows =
            sqlx::query_scalar::<_, String>("SELECT folder_path FROM mods WHERE game_id = ?")
                .bind(GAME)
                .fetch_all(&db.pool)
                .await
                .expect("fresh projected rows")
                .into_iter()
                .map(|path| folder_path_key(&path, None))
                .collect::<Vec<_>>();
        rows.sort();
        assert_eq!(
            rows,
            vec!["alice/blue", "bob/new"],
            "external changes must be projected before accepting the snapshot"
        );
        assert!(needs_repair);
        assert_eq!(
            coordinator
                .current_journal_revision()
                .expect("journal after repair"),
            journal_revision
        );
        assert_eq!(sdk_events.load(Ordering::Acquire), delivered_before_apply);
        drop(paused_delivery);
        drop(lease);
        store.cancel(&session.session_id).expect("fixture cleanup");
    }
}
