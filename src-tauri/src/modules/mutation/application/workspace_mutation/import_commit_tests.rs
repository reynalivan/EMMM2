use super::{apply_item_classification, mark_unproven_recovery_target, DestinationAncestryProof};
use crate::modules::ingestion::application::import_batch::types::{
    ConfidenceTier, ImportContentKind, ImportDecision, ImportFlow, ImportItem, ImportItemStatus,
    ImportMatchStatus, ImportPackageShape, ImportSourceKind, ReviewGate, StableCategory,
    TargetMode,
};

fn import_item(decision: ImportDecision, destination_object_id: Option<&str>) -> ImportItem {
    ImportItem {
        id: "item-1".to_string(),
        batch_id: "batch-1".to_string(),
        source_kind: ImportSourceKind::Folder,
        source_path: "source".to_string(),
        staging_path: None,
        planned_name: "Imported Mod".to_string(),
        status: ImportItemStatus::FinalizingMetadata,
        match_category: Some(StableCategory::Weapon),
        sub_category: Some("Sword".to_string()),
        classification_metadata: serde_json::json!({"rarity": 5}),
        source_metadata: serde_json::json!({}),
        category_suggestions: Vec::new(),
        canonical_suggestions: Vec::new(),
        destination_suggestions: Vec::new(),
        selected_entry_key: (decision == ImportDecision::CreateCanonical)
            .then(|| "new-canonical-key".to_string()),
        selected_alias_name: (decision == ImportDecision::CreateCanonical)
            .then(|| "New Canonical Name".to_string()),
        destination_object_id: destination_object_id.map(str::to_string),
        destination_path: None,
        confidence_percentage: 95,
        confidence_tier: ConfidenceTier::High,
        identity_match_status: ImportMatchStatus::AutoMatched,
        evidence: Vec::new(),
        decision,
        fingerprint: None,
        archive_sha256: None,
        payload_manifest: None,
        duplicate_of_item_id: None,
        target_comparison: None,
        analysis_revision: 1,
        analysis_ack_revision: Some(1),
        review_gate: ReviewGate::default(),
        diagnostics: Vec::new(),
        content_kind: ImportContentKind::Skin,
        package_shape: ImportPackageShape::Single,
        result: None,
        error: None,
    }
}

async fn existing_object_fixture() -> sqlx::SqlitePool {
    let pool = crate::test_utils::init_test_db().await.pool;
    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "game-1",
            name: "Game",
            game_type: crate::modules::games::domain::models::GameType::Gimi,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();
    crate::test_utils::insert_test_object(
        &pool,
        &crate::test_utils::TestObjectFixture {
            id: "object-1",
            game_id: "game-1",
            name: "Existing Object",
            folder_path: "Existing Object",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "mod-1",
            game_id: "game-1",
            object_id: Some("object-1"),
            actual_name: "Existing Mod",
            folder_path: "Existing Object/Existing Mod",
            status: crate::modules::games::domain::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE objects SET sub_category = 'Avatar', metadata = '{\"preserved\":true}', matched_entry_key = 'existing-key', matched_alias_name = 'Existing Alias', matched_source = 'existing-source' WHERE id = 'object-1'",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

#[tokio::test]
async fn reallocate_preserves_existing_object_classification_and_child_mods() {
    let pool = existing_object_fixture().await;
    let item = import_item(ImportDecision::Reallocate, Some("object-1"));

    let result = apply_item_classification(&pool, "game-1", &item, "object-1".to_string())
        .await
        .unwrap();

    assert_eq!(result.object_id, "object-1");
    assert_eq!(result.child_mods_updated, 0);
    assert!(!result.aliases_changed);
    let object: (String, Option<String>, String, Option<String>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT object_type, sub_category, metadata, matched_entry_key, matched_alias_name, matched_source FROM objects WHERE id = 'object-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(object.0, "Character");
    assert_eq!(object.1.as_deref(), Some("Avatar"));
    assert_eq!(object.2, "{\"preserved\":true}");
    assert_eq!(object.3.as_deref(), Some("existing-key"));
    assert_eq!(object.4.as_deref(), Some("Existing Alias"));
    assert_eq!(object.5.as_deref(), Some("existing-source"));
    let child_type: String = sqlx::query_scalar("SELECT object_type FROM mods WHERE id = 'mod-1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(child_type, "Character");
}

#[tokio::test]
async fn create_canonical_retry_with_bound_object_still_applies_classification() {
    let pool = existing_object_fixture().await;
    let item = import_item(ImportDecision::CreateCanonical, Some("object-1"));

    let result = apply_item_classification(&pool, "game-1", &item, "object-1".to_string())
        .await
        .unwrap();

    assert_eq!(result.child_mods_updated, 1);
    let object: (String, Option<String>) =
        sqlx::query_as("SELECT object_type, matched_entry_key FROM objects WHERE id = 'object-1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(object.0, "Weapon");
    assert_eq!(object.1.as_deref(), Some("new-canonical-key"));
}

#[tokio::test]
async fn existing_object_classification_guard_rejects_wrong_game_and_missing_object() {
    let pool = existing_object_fixture().await;
    let item = import_item(ImportDecision::Reallocate, Some("object-1"));

    let wrong_game = apply_item_classification(&pool, "game-2", &item, "object-1".to_string())
        .await
        .unwrap_err();
    assert!(wrong_game
        .to_string()
        .contains("belongs to a different game"));

    let missing_item = import_item(ImportDecision::Reallocate, Some("missing-object"));
    let missing =
        apply_item_classification(&pool, "game-1", &missing_item, "missing-object".to_string())
            .await
            .unwrap_err();
    assert!(missing
        .to_string()
        .contains("Destination object 'missing-object'"));
}

#[tokio::test]
async fn interrupted_recovery_preserves_unproven_target_and_requires_repair() {
    let pool = crate::test_utils::init_test_db().await.pool;
    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "game-1",
            name: "Game",
            game_type: crate::modules::games::domain::models::GameType::Gimi,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();
    crate::modules::ingestion::adapters::sqlite::import_batch::create_batch(
        &pool,
        &crate::modules::ingestion::adapters::sqlite::import_batch::CreateImportBatchRecord {
            id: "batch-recovery".to_string(),
            game_id: "game-1".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[
            crate::modules::ingestion::adapters::sqlite::import_batch::NewImportItemRecord {
                id: "item-recovery".to_string(),
                source_kind: ImportSourceKind::Folder,
                source_path: "missing-source".to_string(),
                staging_path: Some("missing-staging".to_string()),
                planned_name: "Imported Mod".to_string(),
            },
        ],
    )
    .await
    .unwrap();
    sqlx::query("UPDATE import_jobs SET status = 'committing' WHERE id = 'item-recovery'")
        .execute(&pool)
        .await
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("unexpected-target");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("foreign.txt"), "must survive").unwrap();

    mark_unproven_recovery_target(&pool, "item-recovery", &target)
        .await
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(target.join("foreign.txt")).unwrap(),
        "must survive"
    );
    let state: (String, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT status, result, error_msg, destination_path FROM import_jobs WHERE id = 'item-recovery'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state.0, "failed");
    assert_eq!(state.1.as_deref(), Some("recovery_requires_repair"));
    assert!(state
        .2
        .as_deref()
        .is_some_and(|error| error.contains("left unchanged")));
    assert_eq!(state.3.as_deref(), Some(target.to_string_lossy().as_ref()));
}

#[test]
fn destination_proof_rejects_replaced_existing_ancestor() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let object = root.join("Existing Object");
    let target = object.join("Imported Mod");
    std::fs::create_dir_all(&object).unwrap();
    let proof = DestinationAncestryProof::capture(&root, &target).unwrap();

    std::fs::rename(&object, root.join("Original Object")).unwrap();
    std::fs::create_dir(&object).unwrap();

    assert!(proof.validate(&target).is_err());
}

#[test]
fn destination_proof_allows_not_yet_created_subpath() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let object = root.join("Existing Object");
    let target = object.join("Variants/New/Imported Mod");
    std::fs::create_dir_all(&object).unwrap();
    let proof = DestinationAncestryProof::capture(&root, &target).unwrap();

    proof.validate(&target).unwrap();
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    proof.validate(&target).unwrap();
}

fn planned_import_fixture() -> (tempfile::TempDir, super::PlannedMove) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let source = temp.path().join("source/Selected Mod");
    let target = root.join("Imported Mod");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("asset.bin"), b"selected payload").unwrap();
    let source_proof =
        crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&source).unwrap();
    let plan = super::PlannedMove {
        source_proof,
        source_root_proof: None,
        destination_proof: DestinationAncestryProof::capture(&root, &target).unwrap(),
        item: import_item(ImportDecision::Reallocate, Some("object-1")),
        source: source.clone(),
        object_dir: root.clone(),
        target: target.clone(),
        creates_object: false,
    };
    (temp, plan)
}

#[cfg(windows)]
#[test]
fn import_source_cannot_be_replaced_after_final_validation() {
    let (temp, plan) = planned_import_fixture();
    let parked = temp.path().join("Parked Mod");
    let parked_parent = temp.path().join("Parked Source Parent");
    let parent = crate::platform::fs::rename::PinnedDirectory::open(&plan.object_dir).unwrap();

    super::publish_import_source_once(
        &plan,
        &parent,
        || {},
        || {
            std::fs::rename(&plan.source, &parked)
                .expect_err("selected import source must remain pinned through publication");
            std::fs::rename(plan.source.parent().unwrap(), &parked_parent)
                .expect_err("selected import source parent must remain pinned through publication");
        },
    )
    .unwrap();

    assert!(!plan.source.exists());
    assert!(!parked.exists());
    assert!(!parked_parent.exists());
    assert_eq!(
        crate::platform::fs::file_utils::filesystem_identity(&plan.target).as_deref(),
        Some(plan.source_proof.identity())
    );
    assert_eq!(
        std::fs::read(plan.target.join("asset.bin")).unwrap(),
        b"selected payload"
    );
}

#[test]
fn import_rejects_source_replaced_between_validation_and_binding() {
    let (temp, plan) = planned_import_fixture();
    let parked = temp.path().join("Parked Mod");
    let parent = crate::platform::fs::rename::PinnedDirectory::open(&plan.object_dir).unwrap();

    let error = super::publish_import_source_once(
        &plan,
        &parent,
        || {
            std::fs::rename(&plan.source, &parked).unwrap();
            std::fs::create_dir(&plan.source).unwrap();
            std::fs::write(plan.source.join("asset.bin"), b"replacement payload").unwrap();
        },
        || panic!("a replacement source must be rejected before publication"),
    )
    .unwrap_err();

    assert!(error.to_string().contains("changed"));
    assert!(!plan.target.exists());
    assert_eq!(
        std::fs::read(plan.source.join("asset.bin")).unwrap(),
        b"replacement payload"
    );
    assert_eq!(
        std::fs::read(parked.join("asset.bin")).unwrap(),
        b"selected payload"
    );
}

#[tokio::test]
async fn import_rejects_source_replaced_after_planning() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let (temp, plan) = planned_import_fixture();
    let source = plan.source.clone();
    let target = plan.target.clone();
    let parked = temp.path().join("Parked Mod");
    std::fs::rename(&source, &parked).unwrap();
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("asset.bin"), b"replacement payload").unwrap();
    let mut journal = Vec::new();

    let error = super::execute_moves(
        &pool,
        &[plan],
        &mut journal,
        &mut Vec::new(),
        &mut std::collections::BTreeSet::new(),
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("changed"));
    assert!(journal.is_empty());
    assert!(!target.exists());
    assert_eq!(
        std::fs::read(source.join("asset.bin")).unwrap(),
        b"replacement payload"
    );
    assert_eq!(
        std::fs::read(parked.join("asset.bin")).unwrap(),
        b"selected payload"
    );
}

#[tokio::test]
async fn import_move_records_owned_source_and_preserves_occupied_destination() {
    let pool = crate::test_utils::init_test_db().await.pool;
    for occupied in [false, true] {
        let (_temp, plan) = planned_import_fixture();
        if occupied {
            std::fs::create_dir(&plan.target).unwrap();
            std::fs::write(plan.target.join("asset.bin"), b"existing payload").unwrap();
        }
        let mut journal = Vec::new();
        let result = super::execute_moves(
            &pool,
            std::slice::from_ref(&plan),
            &mut journal,
            &mut Vec::new(),
            &mut std::collections::BTreeSet::new(),
        )
        .await;

        if occupied {
            assert!(result.unwrap_err().to_string().contains("target_changed"));
            assert!(journal.is_empty());
            assert_eq!(
                std::fs::read(plan.source.join("asset.bin")).unwrap(),
                b"selected payload"
            );
            assert_eq!(
                std::fs::read(plan.target.join("asset.bin")).unwrap(),
                b"existing payload"
            );
        } else {
            result.unwrap();
            assert!(!plan.source.exists());
            assert_eq!(journal.len(), 1);
            assert_eq!(
                journal[0].source_identity.as_deref(),
                Some(plan.source_proof.identity())
            );
            journal[0]
                .target_proof
                .as_ref()
                .unwrap()
                .validate(&plan.target)
                .unwrap();
            assert_eq!(
                std::fs::read(plan.target.join("asset.bin")).unwrap(),
                b"selected payload"
            );
        }
    }
}

#[cfg(windows)]
#[test]
fn destination_proof_rejects_rebound_windows_junction() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let object = root.join("Existing Object");
    let target = object.join("Imported Mod");
    std::fs::create_dir_all(&object).unwrap();
    let proof = DestinationAncestryProof::capture(&root, &target).unwrap();
    let original = root.join("Original Object");
    std::fs::rename(&object, &original).unwrap();
    let output = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&object)
        .arg(&original)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "junction fixture creation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(proof.validate(&target).is_err());
}

#[cfg(windows)]
#[test]
fn destination_guard_blocks_replacement_after_final_validation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let object = root.join("Existing Object");
    let target = object.join("Imported Mod");
    std::fs::create_dir_all(&object).unwrap();
    let proof = DestinationAncestryProof::capture(&root, &target).unwrap();
    let mut pins = super::DestinationDirectoryPins::default();
    let mut created = Vec::new();
    proof
        .prepare_parent(&target, &mut pins, &mut created)
        .unwrap();
    proof.validate(&target).unwrap();

    assert!(std::fs::rename(&object, root.join("Replaced Object")).is_err());
    drop(pins);
    std::fs::rename(&object, root.join("Replaced Object")).unwrap();
}

#[cfg(windows)]
#[test]
fn destination_guard_pins_each_new_subpath_component() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let object = root.join("New Object");
    let variants = object.join("Variants");
    let target = variants.join("New/Imported Mod");
    std::fs::create_dir_all(&root).unwrap();
    let proof = DestinationAncestryProof::capture(&root, &target).unwrap();
    let mut pins = super::DestinationDirectoryPins::default();
    let mut created = Vec::new();
    proof
        .prepare_parent(&target, &mut pins, &mut created)
        .unwrap();
    proof.validate(&target).unwrap();

    assert!(std::fs::rename(&variants, root.join("Replaced Variants")).is_err());
    let object = crate::shared::path_key::physical_namespace_path(&object).unwrap();
    let variants = crate::shared::path_key::physical_namespace_path(&variants).unwrap();
    assert_eq!(
        created,
        vec![object, variants.clone(), variants.join("New")]
    );
}

#[cfg(windows)]
#[test]
fn pinned_parent_publishes_into_new_destination_tree() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let target = root.join("New Object/Variants/Imported Mod");
    let source = temp.path().join("staged-mod");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("mod.ini"), "[TextureOverride]").unwrap();
    let proof = DestinationAncestryProof::capture(&root, &target).unwrap();
    let mut pins = super::DestinationDirectoryPins::default();
    let mut created = Vec::new();
    proof
        .prepare_parent(&target, &mut pins, &mut created)
        .unwrap();
    let parent = pins.get_parent(&target).unwrap();

    crate::platform::fs::file_utils::rename_cross_drive_fallback_tracked_into(
        &source, &target, parent,
    )
    .unwrap();

    assert!(!source.exists());
    assert_eq!(
        std::fs::read_to_string(target.join("mod.ini")).unwrap(),
        "[TextureOverride]"
    );
}

#[cfg(windows)]
#[test]
fn pinned_parent_publishes_one_character_components() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("R");
    let target = root.join("A/B/C");
    let source = temp.path().join("S");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&source).unwrap();
    let proof = DestinationAncestryProof::capture(&root, &target).unwrap();
    let mut pins = super::DestinationDirectoryPins::default();
    let mut created = Vec::new();
    proof
        .prepare_parent(&target, &mut pins, &mut created)
        .unwrap();
    let parent = pins.get_parent(&target).unwrap();

    crate::platform::fs::file_utils::rename_cross_drive_fallback_tracked_into(
        &source, &target, parent,
    )
    .unwrap();

    assert!(!source.exists());
    assert!(target.is_dir());
}

#[cfg(target_os = "linux")]
#[test]
fn pinned_parent_restores_source_if_namespace_changes_after_validation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let object = root.join("Existing Object");
    let original_object = root.join("Original Object");
    let target = object.join("Imported Mod");
    let source = temp.path().join("staged-mod");
    std::fs::create_dir_all(&object).unwrap();
    std::fs::create_dir(&source).unwrap();
    let proof = DestinationAncestryProof::capture(&root, &target).unwrap();
    let mut pins = super::DestinationDirectoryPins::default();
    let mut created = Vec::new();
    proof
        .prepare_parent(&target, &mut pins, &mut created)
        .unwrap();
    proof.validate(&target).unwrap();
    std::fs::rename(&object, &original_object).unwrap();
    std::fs::create_dir(&object).unwrap();
    let parent = pins.get_parent(&target).unwrap();

    assert!(
        crate::platform::fs::file_utils::rename_cross_drive_fallback_tracked_into(
            &source, &target, parent,
        )
        .is_err()
    );
    assert!(source.exists());
    assert!(!target.exists());
    assert!(!original_object.join("Imported Mod").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn pinned_parent_retains_owned_target_after_second_parent_rename() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let object = root.join("Existing Object");
    let first_moved_object = root.join("First Moved Object");
    let second_moved_object = root.join("Second Moved Object");
    let target = object.join("Imported Mod");
    let source = temp.path().join("staged-mod");
    std::fs::create_dir_all(&object).unwrap();
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("mod.ini"), "owned").unwrap();
    let proof = DestinationAncestryProof::capture(&root, &target).unwrap();
    let mut pins = super::DestinationDirectoryPins::default();
    let mut created = Vec::new();
    proof
        .prepare_parent(&target, &mut pins, &mut created)
        .unwrap();
    proof.validate(&target).unwrap();
    std::fs::rename(&object, &first_moved_object).unwrap();
    std::fs::create_dir(&object).unwrap();
    let parent = pins.get_parent(&target).unwrap();

    let error =
        crate::platform::fs::file_utils::rename_cross_drive_fallback_tracked_into_with_test_hook(
            &source,
            &target,
            parent,
            || {
                std::fs::rename(&first_moved_object, &second_moved_object).unwrap();
                std::fs::create_dir(&source).unwrap();
            },
        )
        .unwrap_err();

    let retained_target = second_moved_object.join("Imported Mod");
    assert!(error.target_is_owned());
    assert_eq!(error.target_path(), Some(retained_target.as_path()));
    error
        .target_proof()
        .unwrap()
        .validate(&retained_target)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(retained_target.join("mod.ini")).unwrap(),
        "owned"
    );
    assert!(source.is_dir());
    assert!(!target.exists());
    assert!(!first_moved_object.exists());
}
