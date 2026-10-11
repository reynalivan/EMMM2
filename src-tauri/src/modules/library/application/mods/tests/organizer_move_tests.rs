use super::*;
use crate::modules::games::domain::models::{GameType, ItemStatus};
use crate::modules::settings::application::config::{AppSettings, ConfigService, GameConfig};
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::validate_path;
use crate::platform::fs::operation_lock::OperationLock;
use crate::test_utils::{
    insert_test_collection, insert_test_game, insert_test_mod, insert_test_object,
    TestCollectionFixture, TestGameFixture, TestModFixture, TestObjectFixture,
};
use std::path::PathBuf;
use tempfile::TempDir;

fn normalized_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn prepared_move(source: &Path, target: &Path) -> PreparedOrganizerMove {
    let destination = target.parent().unwrap();
    let anchor = destination.ancestors().find(|path| path.is_dir()).unwrap();
    let (target_namespace_proof, target_scope_path) =
        capture_move_destination_scope(source.parent().unwrap(), destination).unwrap();
    PreparedOrganizerMove {
        namespace_proof: crate::platform::fs::file_utils::FilesystemNamespaceProof::capture(
            source.parent().unwrap(),
            &[source.to_path_buf()],
        )
        .unwrap(),
        target_namespace_proof,
        target_scope_path,
        target_anchor: anchor.to_path_buf(),
        target_proof: FilesystemIdentityProof::capture(anchor).unwrap(),
        renames: vec![PreparedOrganizerRename {
            source_proof: FilesystemIdentityProof::capture(source).unwrap(),
            old_path: source.to_path_buf(),
            new_path: target.to_path_buf(),
            old_rel: "source".into(),
            new_rel: "target".into(),
            target_object_id: "object".into(),
        }],
        primary_results: vec!["target".into()],
        target_base_path: target.parent().unwrap().to_path_buf(),
        target_base_requires_creation: !destination.exists(),
    }
}

#[cfg(windows)]
#[tokio::test]
async fn organizer_missing_destination_does_not_adopt_a_raced_ancestor_junction() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Mods");
    let source = root.join("Source");
    let outside = fixture.path().join("Outside");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(source.join("mesh.buf"), b"owned source").unwrap();
    let prepared = prepared_move(&source, &root.join("Group/Target/Source"));
    create_junction(&root.join("Group"), &outside);

    assert!(execute_prepared_move(&WatcherState::new(), &prepared)
        .await
        .is_err());
    assert_eq!(
        std::fs::read(source.join("mesh.buf")).unwrap(),
        b"owned source"
    );
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
}

#[tokio::test]
async fn organizer_missing_destination_does_not_adopt_a_raced_directory() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Mods");
    let source = root.join("Source");
    let destination = root.join("Target");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("mesh.buf"), b"owned source").unwrap();
    let prepared = prepared_move(&source, &destination.join("Source"));
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(destination.join("external.txt"), b"external target").unwrap();

    assert!(execute_prepared_move(&WatcherState::new(), &prepared)
        .await
        .is_err());
    assert_eq!(
        std::fs::read(source.join("mesh.buf")).unwrap(),
        b"owned source"
    );
    assert_eq!(
        std::fs::read(destination.join("external.txt")).unwrap(),
        b"external target"
    );
    assert!(!destination.join("Source").exists());
}

#[tokio::test]
async fn organizer_creates_owned_nested_destinations_and_preserves_source_identity() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Mods");
    let source = root.join("Source");
    let target = root.join("Group/Target/Source");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("mesh.buf"), b"owned source").unwrap();
    let identity = FilesystemIdentityProof::capture(&source).unwrap();
    let prepared = prepared_move(&source, &target);

    execute_prepared_move(&WatcherState::new(), &prepared)
        .await
        .unwrap();
    identity.validate(&target).unwrap();
    assert_eq!(
        std::fs::read(target.join("mesh.buf")).unwrap(),
        b"owned source"
    );
    assert!(!source.exists());
}

#[tokio::test]
async fn organizer_only_enable_preserves_each_sibling_parent_when_moving_to_a_container() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Mods");
    let source = root.join("Source");
    let target = root.join("Target/Container/Source");
    let sibling = root.join("Target/Sibling");
    let disabled_sibling = root.join("Target/DISABLED Sibling");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::create_dir(&sibling).unwrap();
    std::fs::write(source.join("mesh.buf"), b"selected source").unwrap();
    std::fs::write(sibling.join("mesh.buf"), b"owned sibling").unwrap();
    let sibling_proof = FilesystemIdentityProof::capture(&sibling).unwrap();
    let mut prepared = prepared_move(&source, &target);
    prepared.namespace_proof = crate::platform::fs::file_utils::FilesystemNamespaceProof::capture(
        &root,
        &[source.clone(), sibling.clone()],
    )
    .unwrap();
    prepared.renames.push(PreparedOrganizerRename {
        source_proof: sibling_proof.clone(),
        old_path: sibling.clone(),
        new_path: disabled_sibling.clone(),
        old_rel: "Target/Sibling".into(),
        new_rel: "Target/DISABLED Sibling".into(),
        target_object_id: "object".into(),
    });

    execute_prepared_move(&WatcherState::new(), &prepared)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(target.join("mesh.buf")).unwrap(),
        b"selected source"
    );
    assert_eq!(
        std::fs::read(disabled_sibling.join("mesh.buf")).unwrap(),
        b"owned sibling"
    );
    sibling_proof.validate(&disabled_sibling).unwrap();
    assert!(!target.parent().unwrap().join("DISABLED Sibling").exists());
    assert!(!source.exists());
    assert!(!sibling.exists());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn organizer_retained_created_destination_rejects_reparenting_before_retry() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Mods");
    let source = root.join("Source");
    let destination = root.join("Group/Target");
    let outside = fixture.path().join("Outside");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(source.join("mesh.buf"), b"owned source").unwrap();
    let prepared = prepared_move(&source, &destination.join("Source"));
    let pins = std::sync::Arc::new(pin_organizer_destination(&prepared).unwrap());
    let identity = FilesystemIdentityProof::capture(&destination).unwrap();
    let moved = outside.join("Target");
    std::fs::rename(&destination, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &destination).unwrap();

    let result = rename_organizer_step(
        &prepared,
        &prepared.renames[0],
        &identity,
        Some(pins),
        false,
        &mut crate::platform::fs::retry::RetryBudget::default(),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        std::fs::read(source.join("mesh.buf")).unwrap(),
        b"owned source"
    );
    assert_eq!(std::fs::read_dir(&moved).unwrap().count(), 0);
}

#[test]
fn organizer_destination_rejects_a_retained_folder_under_a_replaced_ancestor() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Mods");
    let parent = root.join("Target");
    let destination = parent.join("Container");
    std::fs::create_dir_all(&destination).unwrap();
    let identity = FilesystemIdentityProof::capture(&destination).unwrap();
    let (proof, scope) = capture_move_destination_scope(&root, &destination).unwrap();
    let parked = fixture.path().join("ParkedTarget");
    std::fs::rename(&parent, &parked).unwrap();
    std::fs::create_dir(&parent).unwrap();
    std::fs::rename(parked.join("Container"), &destination).unwrap();

    identity.validate(&destination).unwrap();
    assert!(proof.validate_paths(&[scope]).is_err());
}

#[cfg(windows)]
fn create_junction(link: &Path, target: &Path) {
    use std::os::windows::process::CommandExt;
    let result = std::process::Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .creation_flags(0x08000000)
        .output()
        .expect("create owned junction");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[cfg(windows)]
#[tokio::test]
async fn organizer_preparation_rejects_an_outside_destination_junction() {
    let fixture = TempDir::new().unwrap();
    let mods_root = fixture.path().join("Mods");
    let object_root = mods_root.join("Target");
    let outside = fixture.path().join("Outside");
    std::fs::create_dir_all(&object_root).unwrap();
    std::fs::create_dir(&outside).unwrap();
    create_junction(&object_root.join("Linked"), &outside);
    let pool = crate::test_utils::init_test_db().await.pool;
    let root_text = mods_root.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "junction-game",
            name: "Junction",
            game_type: GameType::Gimi,
            path: &root_text,
            mods_path: Some(&root_text),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "junction-target",
            game_id: "junction-game",
            name: "Target",
            folder_path: "Target",
            object_type: "Other",
        },
    )
    .await
    .unwrap();

    let prepared = prepare_move_mods_to_object(
        &pool,
        MoveModsToObjectParams {
            game_id: "junction-game",
            folder_paths: &[],
            target_object_id: "junction-target",
            target_subpath: Some("Linked"),
            status: None,
        },
    )
    .await;

    assert!(matches!(prepared, Err(AppError::Security(_))));
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
}

#[cfg(windows)]
#[tokio::test]
async fn organizer_target_discovery_does_not_follow_junctions() {
    let fixture = TempDir::new().unwrap();
    let mods_root = fixture.path().join("Mods");
    let object_root = mods_root.join("Target");
    let outside = fixture.path().join("Outside");
    std::fs::create_dir_all(object_root.join("Container")).unwrap();
    std::fs::create_dir(&outside).unwrap();
    create_junction(&object_root.join("Linked"), &outside);
    let pool = crate::test_utils::init_test_db().await.pool;
    let root_text = mods_root.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "target-game",
            name: "Targets",
            game_type: GameType::Gimi,
            path: &root_text,
            mods_path: Some(&root_text),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "target-object",
            game_id: "target-game",
            name: "Target",
            folder_path: "Target",
            object_type: "Other",
        },
    )
    .await
    .unwrap();

    let targets = crate::modules::library::application::mods::organizer_targets::list_move_targets_for_object_service(
        &pool, "target-game", "target-object",
    ).await.unwrap();

    assert_eq!(targets.len(), 2);
    assert!(targets
        .iter()
        .any(|target| target.target_subpath.as_deref() == Some("Container")));
    assert!(!targets
        .iter()
        .any(|target| target.target_subpath.as_deref() == Some("Linked")));
}

#[tokio::test]
async fn organizer_move_does_not_overwrite_a_destination_created_after_preparation() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let target = root.path().join("target");
    std::fs::create_dir(&source).unwrap();
    let prepared = prepared_move(&source, &target);
    std::fs::create_dir(&target).unwrap();
    assert!(execute_prepared_move(&WatcherState::new(), &prepared)
        .await
        .is_err());
    assert!(source.is_dir());
    assert!(target.is_dir());
}

#[tokio::test]
async fn organizer_move_rejects_a_replaced_source_after_preparation() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let target = root.path().join("target");
    std::fs::create_dir(&source).unwrap();
    let prepared = prepared_move(&source, &target);
    std::fs::rename(&source, root.path().join("original")).unwrap();
    std::fs::create_dir(&source).unwrap();
    assert!(execute_prepared_move(&WatcherState::new(), &prepared)
        .await
        .is_err());
    assert!(source.is_dir());
    assert!(!target.exists());
}

#[tokio::test]
async fn organizer_rollback_rejects_a_replaced_destination() {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let target = root.path().join("target");
    std::fs::create_dir(&source).unwrap();
    let prepared = prepared_move(&source, &target);
    execute_prepared_move(&WatcherState::new(), &prepared)
        .await
        .unwrap();
    std::fs::rename(&target, root.path().join("original")).unwrap();
    std::fs::create_dir(&target).unwrap();
    assert!(rollback_prepared_move(&WatcherState::new(), &prepared)
        .await
        .is_err());
    assert!(target.is_dir());
    assert!(!source.exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn organizer_move_leaves_projection_and_collection_reference_for_terminal_reconcile() {
    let temp = TempDir::new().expect("temporary mods root");
    let mods_root = temp.path();
    let source_path = mods_root.join("Source").join("Old Mod");
    let target_object_path = mods_root.join("Target");
    std::fs::create_dir_all(&source_path).expect("source folder");
    std::fs::write(
        source_path.join("mod.ini"),
        "[TextureOverrideTest]\nhash = abc\n",
    )
    .expect("mod metadata");
    std::fs::create_dir_all(&target_object_path).expect("target object folder");

    let pool = crate::test_utils::init_test_db().await.pool;
    let mods_root_text = mods_root.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "game",
            name: "Game",
            game_type: GameType::Gimi,
            path: &mods_root_text,
            mods_path: Some(&mods_root_text),
        },
    )
    .await
    .expect("game");
    for (id, name, folder_path) in [
        ("source", "Source", "Source"),
        ("target", "Target", "Target"),
    ] {
        insert_test_object(
            &pool,
            &TestObjectFixture {
                id,
                game_id: "game",
                name,
                folder_path,
                object_type: "Other",
            },
        )
        .await
        .expect("object");
    }
    insert_test_mod(
        &pool,
        &TestModFixture {
            id: "mod",
            game_id: "game",
            object_id: Some("source"),
            actual_name: "Old Mod",
            folder_path: "Source/Old Mod",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Other"),
            mods_path: Some(&mods_root_text),
        },
    )
    .await
    .expect("mod");
    let source_identity = FilesystemIdentityProof::capture(&source_path).unwrap();
    sqlx::query("UPDATE mods SET filesystem_identity = ? WHERE id = 'mod'")
        .bind(source_identity.identity())
        .execute(&pool)
        .await
        .expect("indexed source identity");
    insert_test_collection(
        &pool,
        &TestCollectionFixture {
            id: "collection",
            name: "Saved",
            game_id: "game",
            is_safe: true,
        },
    )
    .await
    .expect("collection");
    sqlx::query(
        "INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_ref_key, object_id, node_type) \
         VALUES ('collection', 'mod', 'Source/Old Mod', 'source/old mod', 'source', 'source', 'FlatModRoot')",
    )
    .execute(&pool)
    .await
    .expect("collection member");

    let config = ConfigService::new_for_test_async(pool.clone()).await;
    config
        .save_settings(AppSettings {
            games: vec![GameConfig {
                id: "game".to_string(),
                name: "Game".to_string(),
                game_type: GameType::Gimi,
                instance_path: PathBuf::from(mods_root),
                mod_path: PathBuf::from(mods_root),
                ready_to_move_path: None,
                launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
                game_exe: Some(PathBuf::from(mods_root)),
                loader_exe: None,
                xxmi_launcher_exe: None,
                launch_args: None,
                warnings: Vec::new(),
            }],
            ..AppSettings::default()
        })
        .expect("test game configuration");
    let folder = validate_path(&config, "game", &source_path.to_string_lossy())
        .expect("validated source folder");
    let lock = OperationLock::new();
    let guard = lock.acquire().await.expect("operation lock");
    let outcome = move_mods_to_object_service(
        &pool,
        &guard,
        &WatcherState::new(),
        MoveModsToObjectParams {
            game_id: "game",
            folder_paths: &[folder],
            target_object_id: "target",
            target_subpath: None,
            status: None,
        },
    )
    .await
    .expect("filesystem move");
    assert!(
        outcome.result.failures.is_empty(),
        "move must reach the terminal reconcile boundary"
    );
    assert_eq!(
        outcome
            .result
            .success
            .iter()
            .map(|path| normalized_path(path))
            .collect::<Vec<_>>(),
        vec!["Target/Old Mod"]
    );
    assert_eq!(outcome.path_hints.len(), 1);
    assert_eq!(
        normalized_path(&outcome.path_hints[0].old_path),
        "Source/Old Mod"
    );
    assert_eq!(
        normalized_path(&outcome.path_hints[0].new_path),
        "Target/Old Mod"
    );
    assert_eq!(outcome.path_hints[0].target_object_id, "target");

    assert!(target_object_path.join("Old Mod").is_dir());
    source_identity
        .validate(&target_object_path.join("Old Mod"))
        .unwrap();
    let mod_row: (Option<String>, String) =
        sqlx::query_as("SELECT object_id, folder_path FROM mods WHERE id = 'mod'")
            .fetch_one(&pool)
            .await
            .expect("mod projection");
    assert_eq!(
        mod_row,
        (Some("source".to_string()), "Source/Old Mod".to_string())
    );
    let collection_path: String = sqlx::query_scalar(
        "SELECT mod_path FROM collection_mods WHERE collection_id = 'collection'",
    )
    .fetch_one(&pool)
    .await
    .expect("collection reference");
    assert_eq!(collection_path, "Source/Old Mod");

    let changed_paths = vec![
        source_path.to_string_lossy().to_string(),
        target_object_path
            .join("Old Mod")
            .to_string_lossy()
            .to_string(),
    ];
    let reconcile_hints = outcome
        .path_hints
        .iter()
        .map(
            |hint| crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcilePathHint {
                old_path: hint.old_path.clone(),
                new_path: hint.new_path.clone(),
                target_object_id: hint.target_object_id.clone(),
            },
        )
        .collect::<Vec<_>>();
    crate::modules::reconciliation::application::disk_reconcile::reconcile::reconcile_disk_projection(
        crate::modules::reconciliation::application::disk_reconcile::reconcile::ReconcileDiskProjectionRequest {
            pool: &pool,
            game_id: "game",
            mods_path: mods_root,
            safe_mode_keywords: &[],
            reason: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::InternalMutation,
            changed_paths: &changed_paths,
            force_full: false,
            watcher_events: None,
            path_hints: &reconcile_hints,
            trusted_mutation_scope: false,
            progress_reporter: None,
            precomputed_discovery: None,
        },
    )
    .await
    .expect("terminal reconcile");

    let healed_mod: (Option<String>, String) =
        sqlx::query_as("SELECT object_id, folder_path FROM mods WHERE game_id = 'game'")
            .fetch_one(&pool)
            .await
            .expect("healed mod projection");
    assert_eq!(
        (healed_mod.0, normalized_path(&healed_mod.1)),
        (Some("target".to_string()), "Target/Old Mod".to_string())
    );
    let healed_collection_path: String = sqlx::query_scalar(
        "SELECT mod_path FROM collection_mods WHERE collection_id = 'collection'",
    )
    .fetch_one(&pool)
    .await
    .expect("healed collection reference");
    assert_eq!(normalized_path(&healed_collection_path), "Target/Old Mod");
}
