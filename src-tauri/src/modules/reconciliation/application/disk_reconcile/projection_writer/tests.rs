use std::path::{Path, PathBuf};

use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use crate::modules::games::domain::models::{GameType, ItemStatus};
use crate::modules::reconciliation::application::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcilePathKind, DiskReconcilePathUpdate,
};
use crate::modules::system::adapters::sqlite::utils::stable_ids::generate_stable_id;
use crate::shared::safety_constants::SAFETY_SOURCE_UNKNOWN;
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
    TestModFixture, TestObjectFixture,
};

use super::{reconcile_projection_in_tx, ProjectionWriteRequest};

#[tokio::test]
async fn metadata_batch_preserves_order_and_serial_parsing_policy() {
    use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::DiskModEntry;
    use crate::modules::reconciliation::application::disk_reconcile::helpers::{
        derive_runtime_mod_metadata, load_runtime_mod_metadata,
    };

    let temp = tempfile::tempdir().expect("metadata batch fixture");
    let entries: Vec<_> = (0..64)
        .map(|position| {
            let raw_name = format!("DISABLED Mod {position}");
            let absolute_path = temp.path().join(&raw_name);
            std::fs::create_dir(&absolute_path).expect("mod directory");
            match position % 4 {
                0 => std::fs::write(
                    absolute_path.join("info.json"),
                    format!(r#"{{"actual_name":"  Disk {position}  ","is_safe":false}}"#),
                )
                .expect("valid metadata"),
                1 => std::fs::write(absolute_path.join("info.json"), "invalid")
                    .expect("invalid metadata"),
                2 => std::fs::create_dir(absolute_path.join("info.json"))
                    .expect("unreadable metadata"),
                _ => {}
            }
            DiskModEntry {
                folder_path: raw_name.clone(),
                folder_path_key: raw_name.to_ascii_lowercase(),
                object_folder_path_key: "object".to_string(),
                raw_name,
                absolute_path,
                filesystem_identity: None,
            }
        })
        .collect();
    let observations = super::mods::observe_metadata_batch(&entries)
        .await
        .expect("bounded metadata batch");
    assert_eq!(observations.len(), entries.len());
    for (entry, observation) in entries.iter().zip(&observations) {
        for current_manual in [None, Some(true), Some(false)] {
            let serial = load_runtime_mod_metadata(
                &entry.absolute_path,
                &entry.raw_name,
                &[],
                current_manual,
            );
            let batched =
                derive_runtime_mod_metadata(observation, &entry.raw_name, &[], current_manual);
            assert_eq!(batched.actual_name, serial.actual_name);
            assert_eq!(batched.is_safe, serial.is_safe);
            assert_eq!(batched.safety_source, serial.safety_source);
            assert_eq!(batched.status, serial.status);
        }
    }
}

struct WriterRun {
    progress: Vec<(crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePhase, u64, Option<u64>)>,
    objects_changed: bool,
    folders_changed: bool,
    db_object_rows_loaded: usize,
    db_mod_rows_loaded: usize,
    path_updates: Vec<DiskReconcilePathUpdate>,
    collection_reference_impact: CollectionReferenceImpact,
    change_summary: crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileChangeSummary,
}

async fn run_writer(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    changed_roots: &[String],
    force_full: bool,
) -> WriterRun {
    let scoped = !force_full && !changed_roots.is_empty();
    let projection = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::collect_scoped_disk_discovery_with_progress(
        mods_path,
        changed_roots,
        scoped,
        None,
    )
    .expect("disk projection should be collected")
    .projection;
    let mut path_updates = Vec::new();
    let mut impact = CollectionReferenceImpact::default();
    let mut change_summary = ChangeSummaryBuilder::default();

    let events = std::sync::Mutex::new(Vec::new());
    let on_progress =
        |phase, completed, total| events.lock().unwrap().push((phase, completed, total));
    let mut tx = pool.begin().await.expect("tx should begin");
    let write_outcome = reconcile_projection_in_tx(
        &mut tx,
        ProjectionWriteRequest {
            game_id,
            mods_path,
            safe_mode_keywords: &[],
            projection: &projection,
            changed_roots,
            scoped,
            force_full,
            path_updates: &mut path_updates,
            collection_reference_impact: &mut impact,
            change_summary: &mut change_summary,
            protected_object_keys: &std::collections::HashSet::new(),
            protected_mod_keys: &std::collections::HashSet::new(),
            on_progress: Some(&on_progress),
        },
    )
    .await
    .expect("projection write should succeed");
    tx.commit().await.expect("tx should commit");

    WriterRun {
        progress: events.into_inner().unwrap(),
        objects_changed: write_outcome.objects_changed,
        folders_changed: write_outcome.folders_changed,
        db_object_rows_loaded: write_outcome.db_object_rows_loaded,
        db_mod_rows_loaded: write_outcome.db_mod_rows_loaded,
        path_updates,
        collection_reference_impact: impact,
        change_summary: change_summary.build(),
    }
}

#[tokio::test]
async fn mmr_replacement_does_not_inherit_manual_safety_but_owned_rename_does() {
    let db = init_test_db().await;
    let root = tempfile::tempdir().unwrap();
    let mods_path = root.path().join("Mods");
    let original = create_terminal_mod(&mods_path, "Alice", "Blue");
    seed_game(&db.pool, "game-1", &mods_path).await;
    run_writer(&db.pool, "game-1", &mods_path, &[], true).await;
    sqlx::query("UPDATE mods SET is_safe=1, safety_source='manual' WHERE game_id='game-1'")
        .execute(&db.pool)
        .await
        .unwrap();
    std::fs::write(
        original.join("info.json"),
        r#"{"actual_name":"Blue","is_safe":false}"#,
    )
    .unwrap();
    let renamed = original.with_file_name("Renamed");
    std::fs::rename(&original, &renamed).unwrap();
    run_writer(&db.pool, "game-1", &mods_path, &[], true).await;
    let (safe, previous_identity): (bool, String) =
        sqlx::query_as("SELECT is_safe,filesystem_identity FROM mods WHERE game_id='game-1'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert!(
        safe,
        "same physical mod retains the explicit choice across rename"
    );
    std::fs::rename(&renamed, root.path().join("retained-original")).unwrap();
    let replacement = create_terminal_mod(&mods_path, "Alice", "Renamed");
    std::fs::write(
        replacement.join("info.json"),
        r#"{"actual_name":"Unsafe replacement","is_safe":false}"#,
    )
    .unwrap();
    run_writer(&db.pool, "game-1", &mods_path, &[], true).await;
    let (name, safe, identity): (String, bool, String) = sqlx::query_as(
        "SELECT actual_name,is_safe,filesystem_identity FROM mods WHERE game_id='game-1'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_ne!(identity, previous_identity);
    assert_eq!(name, "Unsafe replacement");
    assert!(
        !safe,
        "replacement cannot inherit the old physical mod's manual safety"
    );
}

fn create_terminal_mod(mods_path: &Path, object: &str, mod_folder: &str) -> PathBuf {
    let terminal = mods_path.join(object).join(mod_folder);
    std::fs::create_dir_all(&terminal).expect("mod folder should be created");
    std::fs::write(
        terminal.join("mod.ini"),
        "[TextureOverrideAlice]\nhash = abc\n",
    )
    .expect("ini should be written");
    std::fs::write(terminal.join("mesh.buf"), "mesh").expect("asset should be written");
    terminal
}

async fn seed_game(pool: &sqlx::SqlitePool, game_id: &str, mods_path: &Path) -> String {
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::Gimi,
            path: mods_path.parent().unwrap().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    mods_path_string
}

#[tokio::test]
async fn first_index_inserts_physical_identity_without_a_second_write() {
    let db = init_test_db().await;
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("Mods");
    for index in 0..130 {
        let name = format!("DISABLED 衣装{index:03}");
        let folder = create_terminal_mod(&root, "Alice", &name);
        std::fs::write(
            folder.join("info.json"),
            format!(r#"{{"actual_name":"Metadata {index}","is_safe":true}}"#),
        )
        .unwrap();
    }
    seed_game(&db.pool, "game-1", &root).await;
    sqlx::query("CREATE TRIGGER reject_identity_rewrite BEFORE UPDATE OF filesystem_identity ON mods BEGIN SELECT RAISE(ABORT, 'new mod must include its physical identity on insert'); END")
        .execute(&db.pool).await.unwrap();
    let outcome = run_writer(&db.pool, "game-1", &root, &[], true).await;
    assert!(outcome.folders_changed);
    use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePhase;
    assert_eq!(
        outcome.progress,
        vec![
            (DiskReconcilePhase::Projecting, 0, Some(130)),
            (DiskReconcilePhase::Projecting, 64, Some(130)),
            (DiskReconcilePhase::Projecting, 128, Some(130)),
            (DiskReconcilePhase::Projecting, 130, Some(130)),
            (DiskReconcilePhase::Finalizing, 0, None),
        ]
    );
    let rows: Vec<(String, i64, bool, String, Option<String>)> = sqlx::query_as(
        "SELECT actual_name,status,is_safe,safety_source,filesystem_identity FROM mods ORDER BY actual_name"
    ).fetch_all(&db.pool).await.unwrap();
    assert_eq!(rows.len(), 130);
    assert!(rows.iter().all(|(name, status, safe, source, identity)| {
        name.starts_with("Metadata ")
            && *status == ItemStatus::Disabled as i64
            && *safe
            && source == "manual"
            && identity.is_some()
    }));
    let unchanged = run_writer(&db.pool, "game-1", &root, &[], false).await;
    assert!(!unchanged.folders_changed);
}

#[tokio::test]
#[ignore = "manual synthetic fresh-index / unchanged-sync timing, no real library"]
async fn benchmark_fresh_projection_and_unchanged_sync() {
    use std::collections::HashSet;
    use std::time::Instant;
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("Mods");
    const MODS: usize = 2_048;
    for index in 0..MODS {
        let folder = create_terminal_mod(
            &root,
            &format!("Object{:02}", index / 128),
            &format!("Mod{index:04}"),
        );
        if index % 3 == 0 {
            std::fs::write(
                folder.join("info.json"),
                format!(r#"{{"actual_name":"Custom {index}","is_safe":true}}"#),
            )
            .unwrap();
        }
    }
    let projection = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::collect_scoped_disk_discovery_with_progress(&root, &[], false, None).unwrap().projection;
    assert_eq!(projection.mods.len(), MODS);
    for repeat in 0..6 {
        let db = init_test_db().await;
        seed_game(&db.pool, "game-1", &root).await;
        for warm in [false, true] {
            let mut updates = Vec::new();
            let mut impact = CollectionReferenceImpact::default();
            let mut summary = ChangeSummaryBuilder::default();
            let start = Instant::now();
            let mut tx = db.pool.begin().await.unwrap();
            let outcome = reconcile_projection_in_tx(
                &mut tx,
                ProjectionWriteRequest {
                    game_id: "game-1",
                    mods_path: &root,
                    safe_mode_keywords: &[],
                    projection: &projection,
                    changed_roots: &[],
                    scoped: false,
                    force_full: false,
                    path_updates: &mut updates,
                    collection_reference_impact: &mut impact,
                    change_summary: &mut summary,
                    protected_object_keys: &HashSet::new(),
                    protected_mod_keys: &HashSet::new(),
                    on_progress: None,
                },
            )
            .await
            .unwrap();
            let read_start = Instant::now();
            crate::modules::workspace::adapters::sqlite::runtime_projection::rebuild_game_projection_tx(&mut tx, "game-1").await.unwrap();
            let read_us = read_start.elapsed().as_micros();
            tx.commit().await.unwrap();
            let total_us = start.elapsed().as_micros();
            let (count, identified, manual): (i64, i64, i64) = sqlx::query_as(
                "SELECT COUNT(*),COUNT(filesystem_identity),SUM(safety_source='manual') FROM mods",
            )
            .fetch_one(&db.pool)
            .await
            .unwrap();
            assert_eq!(count, MODS as i64);
            assert_eq!(identified, count);
            assert_eq!(manual, 683);
            if warm {
                assert!(!outcome.folders_changed && !outcome.objects_changed);
            }
            eprintln!("fresh_projection repeat={repeat} warm={warm} mods={MODS} initial_index_us={} identity_us={} objects_us={} mods_us={} metadata_within_mods_us={} finalization_us={} read_model_us={read_us} total_us={total_us}",outcome.timings.initial_index.as_micros(),outcome.timings.identity.as_micros(),outcome.timings.objects.as_micros(),outcome.timings.mods.as_micros(),outcome.timings.metadata_within_mods.as_micros(),outcome.timings.finalization.as_micros());
        }
    }
}

#[tokio::test]
async fn failed_later_insert_batch_rolls_back_and_never_reports_finishing() {
    let db = init_test_db().await;
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("Mods");
    for index in 0..130 {
        create_terminal_mod(&root, "Alice", &format!("Mod{index:03}"));
    }
    seed_game(&db.pool, "game-1", &root).await;
    sqlx::query("CREATE TRIGGER reject_later_mod BEFORE INSERT ON mods WHEN NEW.actual_name='Mod064' BEGIN SELECT RAISE(ABORT,'owned fixture failure'); END").execute(&db.pool).await.unwrap();
    let projection = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::collect_scoped_disk_discovery_with_progress(&root,&[],false,None).unwrap().projection;
    let mut updates = Vec::new();
    let mut impact = CollectionReferenceImpact::default();
    let mut summary = ChangeSummaryBuilder::default();
    let events = std::sync::Mutex::new(Vec::new());
    let report = |phase, completed, total| events.lock().unwrap().push((phase, completed, total));
    let mut tx = db.pool.begin().await.unwrap();
    let result = reconcile_projection_in_tx(
        &mut tx,
        ProjectionWriteRequest {
            game_id: "game-1",
            mods_path: &root,
            safe_mode_keywords: &[],
            projection: &projection,
            changed_roots: &[],
            scoped: false,
            force_full: false,
            path_updates: &mut updates,
            collection_reference_impact: &mut impact,
            change_summary: &mut summary,
            protected_object_keys: &std::collections::HashSet::new(),
            protected_mod_keys: &std::collections::HashSet::new(),
            on_progress: Some(&report),
        },
    )
    .await;
    assert!(result.is_err());
    tx.rollback().await.unwrap();
    use crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcilePhase;
    assert_eq!(
        events.into_inner().unwrap(),
        vec![
            (DiskReconcilePhase::Projecting, 0, Some(130)),
            (DiskReconcilePhase::Projecting, 64, Some(130))
        ]
    );
    let counts: (i64, i64) =
        sqlx::query_as("SELECT (SELECT COUNT(*) FROM mods),(SELECT COUNT(*) FROM objects)")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(counts, (0, 0));
}

#[tokio::test]
#[ignore = "manual paired bound INSERT comparison, owned database only"]
async fn benchmark_paired_mod_inserts() {
    #[derive(Debug, PartialEq, sqlx::FromRow)]
    struct InsertedModParity {
        id: String,
        folder_path_key: String,
        actual_name: String,
        status: i64,
        is_safe: bool,
        safety_source: String,
        filesystem_identity: Option<String>,
    }
    use crate::modules::library::adapters::sqlite::mods::{
        insert_reconcile_mods_tx, set_filesystem_identity_tx, ReconcileModInsert,
    };
    async fn insert_reference_mod_tx(
        conn: &mut sqlx::SqliteConnection,
        game_id: &str,
        mods_path: &str,
        row: &ReconcileModInsert<'_>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO mods (id, game_id, object_id, actual_name, folder_path, folder_path_key, status, object_type, is_favorite, is_safe, safety_source, size_bytes) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&row.id)
        .bind(game_id)
        .bind(row.object_id)
        .bind(&row.actual_name)
        .bind(row.folder_path)
        .bind(crate::shared::path_key::folder_path_key(row.folder_path, Some(mods_path)))
        .bind(row.status as i64)
        .bind(row.object_type)
        .bind(false)
        .bind(row.is_safe)
        .bind(row.safety_source)
        .bind(0_i64)
        .execute(conn)
        .await?;
        Ok(())
    }
    use std::time::Instant;
    let db = init_test_db().await;
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("Mods");
    create_terminal_mod(&root, "Alice", "Seed");
    seed_game(&db.pool, "game-1", &root).await;
    run_writer(&db.pool, "game-1", &root, &[], false).await;
    let object: String = sqlx::query_scalar("SELECT id FROM objects LIMIT 1")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let paths = (0..2048)
        .map(|i| format!("Alice/Mod{i:04}"))
        .collect::<Vec<_>>();
    let identities = (0..2048)
        .map(|i| format!("owned-fixture-{i}"))
        .collect::<Vec<_>>();
    let rows = paths
        .iter()
        .enumerate()
        .map(|(i, p)| ReconcileModInsert {
            id: generate_stable_id("game-1", p),
            object_id: &object,
            actual_name: format!("Name {i}"),
            folder_path: p,
            folder_path_key: crate::shared::path_key::folder_path_key(
                p,
                Some(root.to_string_lossy().as_ref()),
            ),
            status: ItemStatus::Disabled,
            object_type: "Other",
            is_safe: true,
            safety_source: "manual",
            filesystem_identity: Some(&identities[i]),
        })
        .collect::<Vec<_>>();
    let mut reference = None;
    for pair in 0..5 {
        for batched in if pair % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            sqlx::query("DELETE FROM mods WHERE game_id='game-1'")
                .execute(&db.pool)
                .await
                .unwrap();
            let started = Instant::now();
            let mut tx = db.pool.begin().await.unwrap();
            if batched {
                insert_reconcile_mods_tx(&mut tx, "game-1", &rows)
                    .await
                    .unwrap();
            } else {
                for row in &rows {
                    insert_reference_mod_tx(
                        &mut tx,
                        "game-1",
                        root.to_string_lossy().as_ref(),
                        row,
                    )
                    .await
                    .unwrap();
                    set_filesystem_identity_tx(&mut tx, &row.id, row.filesystem_identity)
                        .await
                        .unwrap();
                }
            }
            tx.commit().await.unwrap();
            let elapsed_us = started.elapsed().as_micros();
            let persisted:Vec<InsertedModParity>=sqlx::query_as("SELECT id,folder_path_key,actual_name,status,is_safe,safety_source,filesystem_identity FROM mods ORDER BY id").fetch_all(&db.pool).await.unwrap();
            assert_eq!(persisted.len(), rows.len());
            if let Some(expected) = &reference {
                assert_eq!(&persisted, expected);
            } else {
                reference = Some(persisted);
            }
            eprintln!(
                "paired_inserts pair={pair} batched={batched} mods=2048 elapsed_us={elapsed_us}"
            );
        }
    }
}

#[tokio::test]
#[ignore = "manual serial/bounded metadata profiling on generated fixtures only"]
async fn benchmark_bounded_metadata_observations() {
    use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::DiskModEntry;
    use crate::modules::reconciliation::application::disk_reconcile::helpers::{
        derive_runtime_mod_metadata, load_runtime_mod_metadata,
    };
    use std::time::Instant;

    const MODS: usize = 2_048;
    const ROUNDS: usize = 6;
    let fixture = tempfile::tempdir().expect("owned metadata profiler fixture");
    for one_root in [false, true] {
        let root = fixture
            .path()
            .join(if one_root { "one-root" } else { "many-roots" });
        let entries: Vec<_> = (0..MODS)
            .map(|position| {
                let folder_path = format!(
                    "Object{:02}/Mod{position:04}",
                    if one_root { 0 } else { position / 128 },
                );
                let absolute_path = root.join(&folder_path);
                std::fs::create_dir_all(&absolute_path).expect("metadata directory");
                if position % 3 == 0 {
                    let json = serde_json::json!({
                        "actual_name": format!("Custom {position}"),
                        "is_safe": position % 2 == 0,
                        "description": "Owned synthetic description. ".repeat(128),
                        "tags": ["outfit", "variant", "generated-fixture"],
                        "metadata": { "notes": "Synthetic metadata payload. ".repeat(32) },
                    });
                    std::fs::write(absolute_path.join("info.json"), json.to_string())
                        .expect("realistic metadata payload");
                }
                DiskModEntry {
                    folder_path_key: folder_path.to_ascii_lowercase(),
                    object_folder_path_key: format!(
                        "object{:02}",
                        if one_root { 0 } else { position / 128 }
                    ),
                    folder_path,
                    raw_name: format!("Mod{position:04}"),
                    absolute_path,
                    filesystem_identity: None,
                }
            })
            .collect();
        let mut reference = None;
        for round in 0..ROUNDS {
            for batched in if round % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let started = Instant::now();
                let metadata_pass = async {
                    let mut fields = Vec::with_capacity(MODS);
                    let mut peak_observation_bytes = 0;
                    if batched {
                        for batch in entries.chunks(crate::modules::library::adapters::sqlite::mods::RECONCILE_INSERT_BATCH_SIZE) {
                        let observations = super::mods::observe_metadata_batch(batch)
                            .await
                            .expect("bounded metadata observation");
                        peak_observation_bytes = peak_observation_bytes.max(
                            observations.capacity() * std::mem::size_of_val(&observations[0])
                                + observations
                                    .iter()
                                    .map(|observation| {
                                        observation.actual_name.as_ref().map_or(0, String::capacity)
                                    })
                                    .sum::<usize>(),
                        );
                        for (entry, observation) in batch.iter().zip(&observations) {
                            let metadata = derive_runtime_mod_metadata(
                                observation,
                                &entry.raw_name,
                                &[],
                                None,
                            );
                            fields.push((
                                metadata.actual_name,
                                metadata.is_safe,
                                metadata.safety_source,
                                metadata.status,
                            ));
                        }
                    }
                    } else {
                        for entry in &entries {
                            let metadata = load_runtime_mod_metadata(
                                &entry.absolute_path,
                                &entry.raw_name,
                                &[],
                                None,
                            );
                            fields.push((
                                metadata.actual_name,
                                metadata.is_safe,
                                metadata.safety_source,
                                metadata.status,
                            ));
                        }
                    }
                    (fields, peak_observation_bytes)
                };
                let foreground_probe = async {
                    tokio::task::yield_now().await;
                    started.elapsed().as_micros()
                };
                let ((fields, peak_observation_bytes), foreground_probe_us) =
                    tokio::join!(biased; metadata_pass, foreground_probe);
                let elapsed_us = started.elapsed().as_micros();
                if let Some(expected) = &reference {
                    assert_eq!(&fields, expected);
                } else {
                    reference = Some(fields);
                }
                eprintln!("metadata_profile one_root={one_root} round={round} batched={batched} mods={MODS} info_json=683 elapsed_us={elapsed_us} foreground_probe_us={foreground_probe_us} peak_retained_observation_bytes={peak_observation_bytes} os_cache=uncontrolled");
            }
        }
    }
}

async fn seed_collection_with_members(pool: &sqlx::SqlitePool, mod_paths: &[&str]) {
    crate::modules::collections::adapters::sqlite::create(
        pool,
        "collection-swap",
        "game-1",
        "Swap preset",
        true,
        false,
    )
    .await
    .expect("collection seed");

    for mod_path in mod_paths {
        let (mod_id, object_id, object_folder): (String, String, String) = sqlx::query_as(
            r#"SELECT m.id, o.id, o.folder_path
               FROM mods m
               JOIN objects o ON o.id = m.object_id
               WHERE m.game_id = 'game-1' AND m.actual_name = ?"#,
        )
        .bind(mod_path.rsplit('/').next().expect("member name"))
        .fetch_one(pool)
        .await
        .expect("live member seed row");
        let object_ref_key = crate::shared::path_key::folder_path_key(&object_folder, None);
        sqlx::query(
            r#"INSERT OR IGNORE INTO collection_objects
               (collection_id, object_ref_key, object_id, object_display_name, is_enabled)
               VALUES ('collection-swap', ?, ?, ?, 1)"#,
        )
        .bind(&object_ref_key)
        .bind(&object_id)
        .bind(&object_folder)
        .execute(pool)
        .await
        .expect("collection object seed");
        sqlx::query(
            r#"INSERT INTO collection_mods
               (collection_id, mod_id, mod_path, mod_path_key, object_ref_key, object_id, node_type)
               VALUES ('collection-swap', ?, ?, ?, ?, ?, 'FlatModRoot')"#,
        )
        .bind(&mod_id)
        .bind(mod_path)
        .bind(crate::shared::path_key::folder_path_key(mod_path, None))
        .bind(&object_ref_key)
        .bind(&object_id)
        .execute(pool)
        .await
        .expect("collection mod seed");
    }
}

#[tokio::test]
async fn inserts_new_objects_and_mods_discovered_on_disk() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "Blue Dress");
    seed_game(&ctx.pool, "game-1", &mods_path).await;

    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    assert!(run.objects_changed);
    assert!(run.folders_changed);
    assert!(run.path_updates.is_empty());

    let object_row: (String, String, i64) =
        sqlx::query_as("SELECT folder_path, object_type, status FROM objects WHERE game_id = ?")
            .bind("game-1")
            .fetch_one(&ctx.pool)
            .await
            .expect("object row");
    assert_eq!(object_row.0, "Alice");
    assert_eq!(object_row.1, "Other");
    assert_eq!(object_row.2, ItemStatus::Enabled as i64);

    let expected_rel = PathBuf::from("Alice")
        .join("Blue Dress")
        .to_string_lossy()
        .to_string();
    let mod_row: (String, String, i64, String, i64) = sqlx::query_as(
        "SELECT id, folder_path, status, safety_source, size_bytes FROM mods WHERE game_id = ?",
    )
    .bind("game-1")
    .fetch_one(&ctx.pool)
    .await
    .expect("mod row");
    assert_eq!(mod_row.0, generate_stable_id("game-1", &expected_rel));
    assert_eq!(mod_row.1, expected_rel);
    assert_eq!(mod_row.2, ItemStatus::Enabled as i64);
    assert_eq!(mod_row.3, SAFETY_SOURCE_UNKNOWN);
    assert_eq!(
        mod_row.4, 0,
        "routine indexing leaves storage sizing to the optimizer"
    );

    assert_eq!(run.change_summary.object_changes.added, 1);
    assert_eq!(run.change_summary.mod_changes.added, 1);
    assert!(run.change_summary.has_user_visible_changes);
}

#[tokio::test]
async fn full_and_scoped_reconcile_preserve_existing_storage_size() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let terminal = create_terminal_mod(&mods_path, "Alice", "Blue Dress");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    sqlx::query("UPDATE mods SET size_bytes = 777 WHERE game_id = 'game-1'")
        .execute(&ctx.pool)
        .await
        .expect("legacy size");

    std::fs::write(terminal.join("mesh.buf"), [3_u8; 64]).expect("updated mesh");
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &["Alice".into()], false).await;

    let refreshed_size: i64 =
        sqlx::query_scalar("SELECT size_bytes FROM mods WHERE game_id = 'game-1'")
            .fetch_one(&ctx.pool)
            .await
            .expect("refreshed size");
    assert_eq!(refreshed_size, 777);
}

#[tokio::test]
async fn full_reconcile_restores_object_name_after_staging_a_stale_path_key() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "Blue Dress");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    sqlx::query("UPDATE objects SET folder_path_key = 'legacy-alice-key' WHERE game_id = ?")
        .bind("game-1")
        .execute(&ctx.pool)
        .await
        .expect("stale object path key should be seeded");

    let repaired = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    assert_eq!(
        repaired.db_object_rows_loaded, 2,
        "reuse the post-object-stage index"
    );
    assert_eq!(
        repaired.db_mod_rows_loaded, 2,
        "unchanged mod identities need no third load"
    );

    let row: (String, String, String, String) = sqlx::query_as(
        "SELECT name, name_key, folder_path, folder_path_key FROM objects WHERE game_id = ?",
    )
    .bind("game-1")
    .fetch_one(&ctx.pool)
    .await
    .expect("reconciled object row");
    assert_eq!(row.0, "Alice");
    assert_eq!(row.1, crate::shared::path_key::canonical_name_key("Alice"));
    assert_eq!(row.2, "Alice");
    assert_eq!(
        row.3,
        crate::shared::path_key::folder_path_key("Alice", None)
    );
}

#[tokio::test]
async fn unchanged_full_reconcile_after_backfill_has_one_index_pass_and_no_identity_key_writes() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "Blue Dress");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    crate::modules::system::adapters::sqlite::settings::delete_setting(
        &ctx.pool,
        "unicode_key_version",
    )
    .await
    .expect("legacy backfill marker");
    crate::modules::system::adapters::sqlite::utils::unicode_keys::ensure_unicode_keys(&ctx.pool)
        .await
        .expect("startup backfill");
    sqlx::query("CREATE TABLE observed_identity_writes (kind TEXT NOT NULL)")
        .execute(&ctx.pool)
        .await
        .expect("write counter");
    sqlx::query("CREATE TRIGGER count_object_identity_writes AFTER UPDATE OF name_key, folder_path_key ON objects WHEN NEW.name_key IS NOT OLD.name_key OR NEW.folder_path_key IS NOT OLD.folder_path_key BEGIN INSERT INTO observed_identity_writes VALUES ('object'); END")
        .execute(&ctx.pool).await.expect("object key-change counter");
    sqlx::query("CREATE TRIGGER count_mod_identity_writes AFTER UPDATE OF id, folder_path_key ON mods WHEN NEW.id IS NOT OLD.id OR NEW.folder_path_key IS NOT OLD.folder_path_key BEGIN INSERT INTO observed_identity_writes VALUES ('mod'); END")
        .execute(&ctx.pool).await.expect("mod key-change counter");

    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    let identity_writes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM observed_identity_writes")
        .fetch_one(&ctx.pool)
        .await
        .expect("identity write count");
    assert_eq!(
        identity_writes, 0,
        "unchanged keys must never enter the staging namespace"
    );
    assert_eq!(run.db_object_rows_loaded, 1);
    assert_eq!(run.db_mod_rows_loaded, 1);
    assert!(!run.objects_changed);
    assert!(!run.folders_changed);
    assert!(run.path_updates.is_empty());
}

#[tokio::test]
async fn full_reconcile_repairs_an_object_name_leaked_by_an_earlier_stage() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "Blue Dress");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let leaked_stage = ".emmm-reconcile-object-stage-0-leaked";
    sqlx::query("UPDATE objects SET name = ?, name_key = ? WHERE game_id = ?")
        .bind(leaked_stage)
        .bind(crate::shared::path_key::canonical_name_key(leaked_stage))
        .bind("game-1")
        .execute(&ctx.pool)
        .await
        .expect("leaked staging name should be seeded");

    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let row: (String, String) =
        sqlx::query_as("SELECT name, name_key FROM objects WHERE game_id = ?")
            .bind("game-1")
            .fetch_one(&ctx.pool)
            .await
            .expect("reconciled object row");
    assert_eq!(row.0, "Alice");
    assert_eq!(row.1, crate::shared::path_key::canonical_name_key("Alice"));
}

#[tokio::test]
async fn full_reconcile_repairs_a_stale_mod_category_from_its_owner() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "Blue Dress");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    sqlx::query("UPDATE mods SET object_type = 'Outfit' WHERE game_id = 'game-1'")
        .execute(&ctx.pool)
        .await
        .expect("stale category should be stored");

    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let category: String =
        sqlx::query_scalar("SELECT object_type FROM mods WHERE game_id = 'game-1'")
            .fetch_one(&ctx.pool)
            .await
            .expect("category should load");
    assert_eq!(category, "Other");
}

#[tokio::test]
async fn moving_a_mod_to_another_object_adopts_the_target_category() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let source = create_terminal_mod(&mods_path, "Alice", "Blue Dress");
    std::fs::create_dir_all(mods_path.join("Bob")).expect("target object folder");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    sqlx::query("UPDATE objects SET object_type = 'Weapon' WHERE folder_path = 'Bob'")
        .execute(&ctx.pool)
        .await
        .expect("target category");
    sqlx::query("UPDATE mods SET object_type = 'Outfit' WHERE game_id = 'game-1'")
        .execute(&ctx.pool)
        .await
        .expect("source custom category");

    let destination = mods_path.join("Bob").join("Blue Dress");
    std::fs::rename(source, destination).expect("mod should move on disk");
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let row: (String, String) = sqlx::query_as(
        "SELECT o.folder_path, m.object_type FROM mods m JOIN objects o ON o.id = m.object_id WHERE m.game_id = 'game-1'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("moved mod row");
    assert_eq!(row, ("Bob".to_string(), "Weapon".to_string()));
}

#[tokio::test]
async fn heals_disabled_prefix_rename_by_runtime_key_and_records_path_update() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "DISABLED Blue Dress");
    let mods_path_string = seed_game(&ctx.pool, "game-1", &mods_path).await;

    insert_test_object(
        &ctx.pool,
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
        &ctx.pool,
        &TestModFixture {
            id: "mod-blue",
            game_id: "game-1",
            object_id: Some("obj-alice"),
            actual_name: "Blue Dress",
            folder_path: "Alice/Blue Dress",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("mod seed");

    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    assert!(run.folders_changed);
    let expected_rel = PathBuf::from("Alice")
        .join("DISABLED Blue Dress")
        .to_string_lossy()
        .to_string();
    let mod_row: (String, String, i64) =
        sqlx::query_as("SELECT id, folder_path, status FROM mods WHERE game_id = ?")
            .bind("game-1")
            .fetch_one(&ctx.pool)
            .await
            .expect("mod row");
    assert_eq!(mod_row.0, generate_stable_id("game-1", &expected_rel));
    assert_eq!(mod_row.1, expected_rel);
    assert_eq!(mod_row.2, ItemStatus::Disabled as i64);

    assert_eq!(run.path_updates.len(), 1);
    assert_eq!(run.path_updates[0].kind, DiskReconcilePathKind::Mod);
    assert_eq!(run.path_updates[0].from, "Alice/Blue Dress");
    assert_eq!(run.path_updates[0].to, expected_rel);
    assert_eq!(run.change_summary.mod_changes.renamed, 1);
}

#[tokio::test]
async fn force_full_deletes_db_rows_missing_from_disk() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    let mods_path_string = seed_game(&ctx.pool, "game-1", &mods_path).await;

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "obj-bob",
            game_id: "game-1",
            name: "Bob",
            folder_path: "Bob",
            object_type: "Character",
        },
    )
    .await
    .expect("object seed");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-red",
            game_id: "game-1",
            object_id: Some("obj-bob"),
            actual_name: "Red",
            folder_path: "Bob/Red",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("mod seed");

    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    assert!(run.objects_changed);
    assert!(run.folders_changed);
    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("game-1")
        .fetch_one(&ctx.pool)
        .await
        .expect("object count");
    let mod_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = ?")
        .bind("game-1")
        .fetch_one(&ctx.pool)
        .await
        .expect("mod count");
    assert_eq!(object_count, 0);
    assert_eq!(mod_count, 0);

    assert_eq!(run.change_summary.object_changes.removed, 1);
    assert_eq!(run.change_summary.mod_changes.removed, 1);
}

#[tokio::test]
async fn parent_delete_preserves_collection_members_and_reports_every_child_missing() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    let mods_path_string = seed_game(&ctx.pool, "game-1", &mods_path).await;

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "obj-bob",
            game_id: "game-1",
            name: "Bob",
            folder_path: "Bob",
            object_type: "Character",
        },
    )
    .await
    .expect("object seed");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-red",
            game_id: "game-1",
            object_id: Some("obj-bob"),
            actual_name: "Red",
            folder_path: "Bob/Red",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("mod seed");
    crate::modules::collections::adapters::sqlite::create(
        &ctx.pool,
        "collection-1",
        "game-1",
        "Preset",
        true,
        false,
    )
    .await
    .expect("collection seed");
    sqlx::query(
        "INSERT INTO collection_objects (collection_id, object_ref_key, object_id, object_display_name, is_enabled) VALUES (?, ?, ?, ?, 1)",
    )
    .bind("collection-1")
    .bind("bob")
    .bind("obj-bob")
    .bind("Bob")
    .execute(&ctx.pool)
    .await
    .expect("collection object seed");
    sqlx::query(
        "INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_ref_key, object_id, node_type) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind("collection-1")
    .bind("mod-red")
    .bind("Bob/Red")
    .bind(crate::shared::path_key::folder_path_key("Bob/Red", None))
    .bind("bob")
    .bind("obj-bob")
    .bind("FlatModRoot")
    .execute(&ctx.pool)
    .await
    .expect("collection mod seed");

    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    assert_eq!(
        run.collection_reference_impact.missing_paths,
        vec!["Bob/Red"]
    );
    assert_eq!(run.collection_reference_impact.affected_collection_count, 1);
    let member_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM collection_mods WHERE collection_id = 'collection-1'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("member count");
    assert_eq!(
        member_count, 1,
        "collection membership must survive parent deletion"
    );
    let snapshot_json: String =
        sqlx::query_scalar("SELECT snapshot_json FROM collections WHERE id = 'collection-1'")
            .fetch_one(&ctx.pool)
            .await
            .expect("collection snapshot");
    let snapshot = crate::modules::workspace::application::projected_state::parse_snapshot_json(
        &snapshot_json,
    )
    .unwrap_or_else(|| panic!("invalid collection snapshot: {snapshot_json}"));
    assert_eq!(snapshot.summary.missing_root_count, 1);
}

#[tokio::test]
async fn recreated_parent_and_child_rebind_durable_collection_references() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    let mods_path_string = seed_game(&ctx.pool, "game-1", &mods_path).await;

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "obj-bob-old",
            game_id: "game-1",
            name: "Bob",
            folder_path: "Bob",
            object_type: "Character",
        },
    )
    .await
    .expect("object seed");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-red-old",
            game_id: "game-1",
            object_id: Some("obj-bob-old"),
            actual_name: "Red",
            folder_path: "Bob/Red",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("mod seed");
    crate::modules::collections::adapters::sqlite::create(
        &ctx.pool,
        "collection-1",
        "game-1",
        "Preset",
        true,
        false,
    )
    .await
    .expect("collection seed");
    sqlx::query(
        "INSERT INTO collection_objects (collection_id, object_ref_key, object_id, object_display_name, is_enabled) VALUES ('collection-1', 'bob', 'obj-bob-old', 'Bob', 1)",
    )
    .execute(&ctx.pool)
    .await
    .expect("collection object seed");
    sqlx::query(
        "INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_ref_key, object_id, node_type) VALUES ('collection-1', 'mod-red-old', 'Bob/Red', ?, 'bob', 'obj-bob-old', 'FlatModRoot')",
    )
    .bind(crate::shared::path_key::folder_path_key("Bob/Red", None))
    .execute(&ctx.pool)
    .await
    .expect("collection mod seed");

    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    create_terminal_mod(&mods_path, "Bob", "Red");
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let object_binding: Option<String> = sqlx::query_scalar(
        "SELECT object_id FROM collection_objects WHERE collection_id = 'collection-1'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("collection object binding");
    let mod_binding: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT mod_id, object_id FROM collection_mods WHERE collection_id = 'collection-1'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("collection mod binding");
    let live_object_id: String =
        sqlx::query_scalar("SELECT id FROM objects WHERE game_id = 'game-1'")
            .fetch_one(&ctx.pool)
            .await
            .expect("live object");
    let live_mod_id: String = sqlx::query_scalar("SELECT id FROM mods WHERE game_id = 'game-1'")
        .fetch_one(&ctx.pool)
        .await
        .expect("live mod");

    assert_eq!(object_binding.as_deref(), Some(live_object_id.as_str()));
    assert_eq!(mod_binding.0.as_deref(), Some(live_mod_id.as_str()));
    assert_eq!(mod_binding.1.as_deref(), Some(live_object_id.as_str()));
}

#[tokio::test]
async fn full_reconcile_heals_offline_nested_terminal_rename_by_filesystem_identity() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let old_terminal = create_terminal_mod(&mods_path, "Alice/Variants", "Old");
    seed_game(&ctx.pool, "game-1", &mods_path).await;

    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    let (old_mod_id, object_id): (String, String) =
        sqlx::query_as("SELECT id, object_id FROM mods WHERE game_id = 'game-1'")
            .fetch_one(&ctx.pool)
            .await
            .expect("initial mod row");
    crate::modules::collections::adapters::sqlite::create(
        &ctx.pool,
        "collection-1",
        "game-1",
        "Preset",
        true,
        false,
    )
    .await
    .expect("collection seed");
    sqlx::query(
        "INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_ref_key, object_id, node_type) VALUES ('collection-1', ?, 'Alice/Variants/Old', ?, 'alice', ?, 'FlatModRoot')",
    )
    .bind(&old_mod_id)
    .bind(crate::shared::path_key::folder_path_key(
        "Alice/Variants/Old",
        None,
    ))
    .bind(&object_id)
    .execute(&ctx.pool)
    .await
    .expect("collection member seed");

    let new_terminal = mods_path.join("Alice").join("Variants").join("New");
    std::fs::rename(&old_terminal, &new_terminal).expect("offline rename");
    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let live_paths: Vec<String> =
        sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = 'game-1'")
            .fetch_all(&ctx.pool)
            .await
            .expect("live mod rows");
    let collection_path: String = sqlx::query_scalar(
        "SELECT mod_path FROM collection_mods WHERE collection_id = 'collection-1'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("collection path");
    let expected_new_path = PathBuf::from("Alice")
        .join("Variants")
        .join("New")
        .to_string_lossy()
        .to_string();
    assert_eq!(live_paths, vec![expected_new_path.clone()]);
    assert_eq!(
        crate::shared::path_key::folder_path_key(&collection_path, None),
        crate::shared::path_key::folder_path_key(&expected_new_path, None)
    );
    assert!(run.path_updates.iter().any(|update| {
        crate::shared::path_key::folder_path_key(&update.from, None).ends_with("variants/old")
            && crate::shared::path_key::folder_path_key(&update.to, None).ends_with("variants/new")
    }));
    assert!(run.collection_reference_impact.missing_paths.is_empty());
}

#[tokio::test]
async fn full_reconcile_heals_offline_parent_rename_and_all_collection_children() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "Blue");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    let (object_id, mod_id): (String, String) = sqlx::query_as(
        "SELECT o.id, m.id FROM objects o JOIN mods m ON m.object_id = o.id WHERE o.game_id = 'game-1'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("runtime rows");
    crate::modules::collections::adapters::sqlite::create(
        &ctx.pool,
        "collection-1",
        "game-1",
        "Preset",
        true,
        false,
    )
    .await
    .expect("collection");
    sqlx::query("INSERT INTO collection_objects (collection_id, object_ref_key, object_id, object_display_name, is_enabled) VALUES ('collection-1', 'alice', ?, 'Alice', 1)")
        .bind(&object_id)
        .execute(&ctx.pool)
        .await
        .expect("collection object");
    sqlx::query("INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_ref_key, object_id, node_type) VALUES ('collection-1', ?, 'Alice/Blue', ?, 'alice', ?, 'FlatModRoot')")
        .bind(&mod_id)
        .bind(crate::shared::path_key::folder_path_key("Alice/Blue", None))
        .bind(&object_id)
        .execute(&ctx.pool)
        .await
        .expect("collection mod");

    std::fs::rename(mods_path.join("Alice"), mods_path.join("Alicia"))
        .expect("offline parent rename");
    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let live_object: (String, String) =
        sqlx::query_as("SELECT id, folder_path FROM objects WHERE game_id = 'game-1'")
            .fetch_one(&ctx.pool)
            .await
            .expect("live object");
    let collection_path: String = sqlx::query_scalar(
        "SELECT mod_path FROM collection_mods WHERE collection_id = 'collection-1'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("collection path");
    let durable_object_key: String = sqlx::query_scalar(
        "SELECT object_ref_key FROM collection_objects WHERE collection_id = 'collection-1'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("durable object key");
    assert_eq!(live_object.0, object_id);
    assert_eq!(live_object.1, "Alicia");
    assert_eq!(
        crate::shared::path_key::folder_path_key(&collection_path, None),
        "alicia/blue"
    );
    assert_eq!(durable_object_key, "alicia");
    assert!(run.collection_reference_impact.missing_paths.is_empty());
}

#[tokio::test]
async fn offline_rename_into_deleted_destination_keeps_source_row_metadata() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let source = create_terminal_mod(&mods_path, "Alice", "Source");
    let destination = create_terminal_mod(&mods_path, "Alice", "Destination");
    std::fs::write(source.join("custom.png"), "source preview").expect("source preview");
    std::fs::write(destination.join("old.png"), "destination preview")
        .expect("destination preview");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    sqlx::query("UPDATE mods SET is_favorite = 1 WHERE actual_name = 'Source'")
        .execute(&ctx.pool)
        .await
        .expect("source metadata");
    crate::modules::collections::adapters::sqlite::create(
        &ctx.pool,
        "collection-1",
        "game-1",
        "Replacement",
        true,
        false,
    )
    .await
    .expect("collection");
    let members: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, folder_path, object_id FROM mods WHERE game_id = 'game-1' ORDER BY folder_path",
    )
    .fetch_all(&ctx.pool)
    .await
    .expect("members");
    for (mod_id, mod_path, object_id) in members {
        let is_source =
            crate::shared::path_key::folder_path_key(&mod_path, None).ends_with("alice/source");
        let preview_path = PathBuf::from(&mod_path)
            .join(if is_source { "custom.png" } else { "old.png" })
            .to_string_lossy()
            .to_string();
        let node_type = if is_source {
            "SourceNode"
        } else {
            "DestinationNode"
        };
        let warnings_json = if is_source {
            r#"["source-warning"]"#
        } else {
            "[]"
        };
        sqlx::query(
            "INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_ref_key, object_id, preview_path, node_type, warnings_json) VALUES ('collection-1', ?, ?, ?, 'alice', ?, ?, ?, ?)",
        )
        .bind(mod_id)
        .bind(&mod_path)
        .bind(crate::shared::path_key::folder_path_key(&mod_path, None))
        .bind(object_id)
        .bind(preview_path)
        .bind(node_type)
        .bind(warnings_json)
        .execute(&ctx.pool)
        .await
        .expect("collection member");
    }
    sqlx::query(
        "UPDATE collection_mods SET mod_path_key = NULL WHERE collection_id = 'collection-1' AND mod_path LIKE '%Destination'",
    )
    .execute(&ctx.pool)
    .await
    .expect("legacy destination key");

    std::fs::remove_dir_all(&destination).expect("old destination removed");
    std::fs::rename(&source, &destination).expect("source moved into destination");
    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT folder_path, is_favorite FROM mods WHERE game_id = 'game-1'")
            .fetch_all(&ctx.pool)
            .await
            .expect("live rows");
    assert_eq!(rows.len(), 1);
    assert!(
        crate::shared::path_key::folder_path_key(&rows[0].0, None).ends_with("alice/destination")
    );
    assert_eq!(rows[0].1, 1);
    assert_eq!(run.change_summary.mod_changes.removed, 1);
    assert!(run.collection_reference_impact.missing_paths.is_empty());
    type CollectionMemberRow = (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    );
    let collection_members: Vec<CollectionMemberRow> = sqlx::query_as(
        "SELECT mod_path, mod_id, preview_path, node_type, warnings_json FROM collection_mods WHERE collection_id = 'collection-1'",
    )
    .fetch_all(&ctx.pool)
    .await
    .expect("merged collection member");
    assert_eq!(collection_members.len(), 1);
    assert!(
        crate::shared::path_key::folder_path_key(&collection_members[0].0, None)
            .ends_with("alice/destination")
    );
    assert!(collection_members[0].1.is_some());
    let preview_path = collection_members[0]
        .2
        .as_deref()
        .expect("source preview path");
    assert!(crate::shared::path_key::folder_path_key(preview_path, None)
        .ends_with("alice/destination/custom.png"));
    assert!(mods_path.join(preview_path).is_file());
    assert_eq!(collection_members[0].3.as_deref(), Some("SourceNode"));
    assert_eq!(collection_members[0].4, r#"["source-warning"]"#);
}

#[tokio::test]
async fn offline_folder_swap_follows_both_filesystem_identities_without_id_collision() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let first = create_terminal_mod(&mods_path, "Alice", "First");
    let second = create_terminal_mod(&mods_path, "Alice", "Second");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    seed_collection_with_members(&ctx.pool, &["Alice/First", "Alice/Second"]).await;
    sqlx::query("UPDATE mods SET is_favorite = 1 WHERE actual_name = 'First'")
        .execute(&ctx.pool)
        .await
        .expect("first metadata");

    let stage = mods_path.join("Alice").join("Swap Stage");
    std::fs::rename(&first, &stage).expect("first staged");
    std::fs::rename(&second, &first).expect("second moved to first");
    std::fs::rename(&stage, &second).expect("first moved to second");
    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT folder_path, is_favorite FROM mods WHERE game_id = 'game-1' ORDER BY folder_path",
    )
    .fetch_all(&ctx.pool)
    .await
    .expect("live rows");
    assert_eq!(rows.len(), 2);
    assert!(crate::shared::path_key::folder_path_key(&rows[0].0, None).ends_with("alice/first"));
    assert_eq!(rows[0].1, 0);
    assert!(crate::shared::path_key::folder_path_key(&rows[1].0, None).ends_with("alice/second"));
    assert_eq!(rows[1].1, 1);
    assert_eq!(run.change_summary.mod_changes.removed, 0);
    let collection_paths: Vec<String> = sqlx::query_scalar(
        "SELECT mod_path FROM collection_mods WHERE collection_id = 'collection-swap' ORDER BY mod_path",
    )
    .fetch_all(&ctx.pool)
    .await
    .expect("collection paths");
    assert_eq!(
        collection_paths,
        vec!["Alice/First".to_string(), "Alice/Second".to_string()]
    );
}

#[tokio::test]
async fn offline_parent_swap_preserves_object_and_child_ownership() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "Blue");
    create_terminal_mod(&mods_path, "Bob", "Red");
    seed_game(&ctx.pool, "game-1", &mods_path).await;
    run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;
    seed_collection_with_members(&ctx.pool, &["Alice/Blue", "Bob/Red"]).await;
    sqlx::query("UPDATE objects SET object_type = 'AliceType' WHERE folder_path = 'Alice'")
        .execute(&ctx.pool)
        .await
        .expect("alice metadata");
    sqlx::query("UPDATE objects SET object_type = 'BobType' WHERE folder_path = 'Bob'")
        .execute(&ctx.pool)
        .await
        .expect("bob metadata");
    sqlx::query("UPDATE mods SET is_favorite = 1 WHERE actual_name = 'Blue'")
        .execute(&ctx.pool)
        .await
        .expect("blue metadata");

    let alice = mods_path.join("Alice");
    let bob = mods_path.join("Bob");
    let stage = mods_path.join("Parent Swap Stage");
    std::fs::rename(&alice, &stage).expect("alice staged");
    std::fs::rename(&bob, &alice).expect("bob moved to alice");
    std::fs::rename(&stage, &bob).expect("alice moved to bob");
    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    let objects: Vec<(String, String)> = sqlx::query_as(
        "SELECT folder_path, object_type FROM objects WHERE game_id = 'game-1' ORDER BY folder_path",
    )
    .fetch_all(&ctx.pool)
    .await
    .expect("objects");
    assert_eq!(
        objects,
        vec![
            ("Alice".to_string(), "BobType".to_string()),
            ("Bob".to_string(), "AliceType".to_string()),
        ]
    );
    let mods: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT m.folder_path, m.is_favorite, o.folder_path FROM mods m JOIN objects o ON o.id = m.object_id WHERE m.game_id = 'game-1' ORDER BY m.folder_path",
    )
    .fetch_all(&ctx.pool)
    .await
    .expect("mods");
    assert!(mods[0].0.ends_with("Alice\\Red") || mods[0].0.ends_with("Alice/Red"));
    assert_eq!((mods[0].1, mods[0].2.as_str()), (0, "Alice"));
    assert!(mods[1].0.ends_with("Bob\\Blue") || mods[1].0.ends_with("Bob/Blue"));
    assert_eq!((mods[1].1, mods[1].2.as_str()), (1, "Bob"));
    assert_eq!(run.change_summary.mod_changes.removed, 0);
    let collection_objects: Vec<String> = sqlx::query_scalar(
        "SELECT object_ref_key FROM collection_objects WHERE collection_id = 'collection-swap' ORDER BY object_ref_key",
    )
    .fetch_all(&ctx.pool)
    .await
    .expect("collection objects");
    assert_eq!(
        collection_objects,
        vec!["alice".to_string(), "bob".to_string()]
    );
    let collection_paths: Vec<String> = sqlx::query_scalar(
        "SELECT mod_path FROM collection_mods WHERE collection_id = 'collection-swap' ORDER BY mod_path",
    )
    .fetch_all(&ctx.pool)
    .await
    .expect("collection paths");
    assert_eq!(
        collection_paths,
        vec!["Alice/Red".to_string(), "Bob/Blue".to_string()]
    );
}

#[tokio::test]
async fn scoped_run_leaves_out_of_scope_rows_untouched() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    let mods_path_string = seed_game(&ctx.pool, "game-1", &mods_path).await;

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "obj-bob",
            game_id: "game-1",
            name: "Bob",
            folder_path: "Bob",
            object_type: "Character",
        },
    )
    .await
    .expect("object seed");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-red",
            game_id: "game-1",
            object_id: Some("obj-bob"),
            actual_name: "Red",
            folder_path: "Bob/Red",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("mod seed");

    // Scope only covers "Alice"; "Bob" is stale on disk but must survive.
    let run = run_writer(
        &ctx.pool,
        "game-1",
        &mods_path,
        &["Alice".to_string()],
        false,
    )
    .await;

    assert!(!run.objects_changed);
    assert!(!run.folders_changed);
    assert_eq!(run.db_object_rows_loaded, 0);
    assert_eq!(run.db_mod_rows_loaded, 0);
    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("game-1")
        .fetch_one(&ctx.pool)
        .await
        .expect("object count");
    let mod_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = ?")
        .bind("game-1")
        .fetch_one(&ctx.pool)
        .await
        .expect("mod count");
    assert_eq!(object_count, 1);
    assert_eq!(mod_count, 1);
    assert!(!run.change_summary.has_user_visible_changes);
}

#[tokio::test]
async fn scoped_run_loads_only_rows_below_affected_roots() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "Blue");
    let mods_path_string = seed_game(&ctx.pool, "game-1", &mods_path).await;

    for (object_id, object_name, mod_id, mod_name) in [
        ("obj-alice", "Alice", "mod-blue", "Blue"),
        ("obj-bob", "Bob", "mod-red", "Red"),
    ] {
        insert_test_object(
            &ctx.pool,
            &TestObjectFixture {
                id: object_id,
                game_id: "game-1",
                name: object_name,
                folder_path: object_name,
                object_type: "Character",
            },
        )
        .await
        .expect("object seed");
        let folder_path = format!("{object_name}/{mod_name}");
        insert_test_mod(
            &ctx.pool,
            &TestModFixture {
                id: mod_id,
                game_id: "game-1",
                object_id: Some(object_id),
                actual_name: mod_name,
                folder_path: &folder_path,
                status: ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: Some(&mods_path_string),
            },
        )
        .await
        .expect("mod seed");
    }

    let run = run_writer(
        &ctx.pool,
        "game-1",
        &mods_path,
        &["Alice".to_string()],
        false,
    )
    .await;

    assert_eq!(run.db_object_rows_loaded, 1);
    assert_eq!(run.db_mod_rows_loaded, 1);
    let bob_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mods WHERE game_id = 'game-1' AND folder_path = 'Bob/Red'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Bob row count");
    assert_eq!(bob_count, 1);
}
