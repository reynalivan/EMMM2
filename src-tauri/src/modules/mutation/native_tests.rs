use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use super::coordinator::{IntentTarget, MutationCoordinator};
use super::journal::{OperationJournal, OperationPlan, OperationStatus, PlannedStep};
use super::recovery::{RecoveryRoots, RecoveryRunner};
use crate::modules::library::api::mods::core_ops::{plan_toggle_rename, ToggleRenamePlan};
use crate::platform::fs::file_utils::filesystem_identity;
use crate::platform::fs::operation_lock::OperationLock;

const GAME_ID: &str = "native-switch-test";
const CHILD_ROOT_ENV: &str = "EMMM_NATIVE_SWITCH_TEST_ROOT";
const CHILD_BOUNDARY_ENV: &str = "EMMM_NATIVE_SWITCH_TEST_BOUNDARY";
const EXIT_CODE: i32 = 86;
const HISTORY_LIMIT: usize = 16;
const SAMPLE_COUNT: usize = 100;
const WARMUP_COUNT: usize = 8;
const REPEAT_COUNT: usize = 3;

fn durable_plan(root: &Path, rename: &ToggleRenamePlan) -> OperationPlan {
    OperationPlan::new(
        "workspace-switch",
        GAME_ID,
        vec![PlannedStep::rename(
            0,
            rename.old_path().to_path_buf(),
            rename.new_path().to_path_buf(),
        )
        .with_expected_identity(Some(rename.expected_identity().to_owned()))],
    )
    .with_source_epoch(filesystem_identity(root).expect("root identity"))
}

fn exit_at(selected: &str, boundary: &str) {
    if selected == boundary {
        // Deliberately bypass destructors: the parent must recover only persisted evidence.
        std::process::exit(EXIT_CODE);
    }
}

#[tokio::test]
#[ignore = "subprocess fixture; run through native_process_exit_recovers_each_disk_boundary"]
async fn native_exit_worker() {
    let Some(root) = std::env::var_os(CHILD_ROOT_ENV).map(PathBuf::from) else {
        return;
    };
    let boundary = std::env::var(CHILD_BOUNDARY_ENV).expect("child crash boundary");
    let mods = root.join("Mods");
    let rename = plan_toggle_rename(&mods.join("Blue"), false)
        .unwrap()
        .expect("disable plan");
    let journal = OperationJournal::open(root.join("journal.json"), HISTORY_LIMIT).unwrap();
    exit_at(&boundary, "before-plan");
    let id = journal
        .plan_operation(durable_plan(&mods, &rename))
        .unwrap();
    exit_at(&boundary, "planned");
    journal.mark_applying(&id).unwrap();
    exit_at(&boundary, "applying");
    rename.apply("mod folder").unwrap();
    exit_at(&boundary, "renamed");
    journal.mark_step_applied(&id, 0).unwrap();
    exit_at(&boundary, "settled");
    journal.mark_disk_committed(&id).unwrap();
    exit_at(&boundary, "disk-committed");
    journal.mark_db_committed(&id).unwrap();
    exit_at(&boundary, "db-committed");
    journal.complete(&id).unwrap();
    exit_at(&boundary, "completed");
    panic!("unknown crash boundary: {boundary}");
}

#[tokio::test]
async fn native_process_exit_recovers_each_disk_boundary() {
    for boundary in [
        "before-plan",
        "planned",
        "applying",
        "renamed",
        "settled",
        "disk-committed",
        "db-committed",
        "completed",
    ] {
        let fixture = tempdir().unwrap();
        let mods = fixture.path().join("Mods");
        let source = mods.join("Blue");
        let target = mods.join("DISABLED Blue");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("marker.ini"), b"preserve payload").unwrap();
        let identity = filesystem_identity(&source).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "modules::mutation::native_tests::native_exit_worker",
                "--ignored",
                "--nocapture",
            ])
            .env(CHILD_ROOT_ENV, fixture.path())
            .env(CHILD_BOUNDARY_ENV, boundary)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!("child timed out at {boundary}: {output:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(EXIT_CODE),
            "{boundary}: {output:?}"
        );

        let journal = Arc::new(
            OperationJournal::open(fixture.path().join("journal.json"), HISTORY_LIMIT).unwrap(),
        );
        let staging = fixture.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        let recovery = RecoveryRunner::new(
            journal.clone(),
            RecoveryRoots::new(HashMap::from([(GAME_ID.to_owned(), mods)]), staging),
        );
        recovery.run_recovery().await.unwrap();
        let committed = matches!(boundary, "disk-committed" | "db-committed" | "completed");
        let retained = if committed { &target } else { &source };
        assert!(retained.is_dir(), "retained disk state at {boundary}");
        assert!(!if committed { &source } else { &target }.exists());
        assert_eq!(
            filesystem_identity(retained).as_deref(),
            Some(identity.as_str())
        );
        assert_eq!(
            std::fs::read(retained.join("marker.ini")).unwrap(),
            b"preserve payload"
        );
        let entries = journal.entries();
        if boundary == "before-plan" {
            assert!(entries.is_empty());
        } else {
            let expected = match boundary {
                "disk-committed" => OperationStatus::DiskCommitted,
                "db-committed" | "completed" => OperationStatus::Completed,
                _ => OperationStatus::RolledBack,
            };
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].status, expected, "journal at {boundary}");
            assert_eq!(entries[0].disk_revision.is_some(), committed);
        }
        // Restarting recovery twice must neither flip acknowledged state nor lose payload.
        recovery.run_recovery().await.unwrap();
        assert!(retained.is_dir());
        assert_eq!(journal.entries(), entries);
    }
}

fn create_library(root: &Path, count: usize, nested: bool) -> PathBuf {
    for index in 0..count {
        let parent = if nested {
            root.join(format!("Object-{}", index % 10))
                .join(format!("Pack-{}", index / 10 % 10))
        } else {
            root.to_path_buf()
        };
        std::fs::create_dir_all(parent.join(format!("Mod-{index:05}"))).unwrap();
    }
    if nested {
        root.join("Object-0").join("Pack-0").join("Mod-00000")
    } else {
        root.join("Mod-00000")
    }
}

fn percentile(samples: &[Duration], percent: usize) -> f64 {
    samples[(samples.len() * percent).div_ceil(100).saturating_sub(1)].as_secs_f64() * 1000.0
}

#[derive(Debug)]
struct StorageAckSample {
    path: PathBuf,
    total: Duration,
    admission: Duration,
    lease_wait: Duration,
    durable_storage: Duration,
}

async fn measure_storage_ack(
    coordinator: &MutationCoordinator,
    mods: &Path,
    path: PathBuf,
    epoch: &str,
    intent_revision: u64,
    enable: bool,
) -> StorageAckSample {
    let started = Instant::now();
    let admission = coordinator.admit_intents_in_epoch(
        GAME_ID,
        Some(epoch),
        Some(intent_revision),
        [IntentTarget::ModPath(path.to_string_lossy().into_owned())],
    );
    let admission_elapsed = started.elapsed();
    let lease_started = Instant::now();
    let lease = coordinator.inner_lock().acquire().await.unwrap();
    let lease_wait = lease_started.elapsed();
    assert!(admission.is_current());
    admission.validate_resolved_path(&path, &path).unwrap();
    let rename = plan_toggle_rename(&path, enable)
        .unwrap()
        .expect("alternating switch plan");
    let durable_started = Instant::now();
    let operation = coordinator
        .begin_operation_under_lock(durable_plan(mods, &rename))
        .unwrap();
    rename.apply("mod folder").unwrap();
    assert_eq!(
        filesystem_identity(rename.new_path()).as_deref(),
        Some(rename.expected_identity())
    );
    assert!(!rename.old_path().exists());
    operation.mark_step_applied(0).unwrap();
    operation.mark_disk_committed().unwrap();
    let durable_storage = durable_started.elapsed();
    let total = started.elapsed();
    let path = rename.new_path().to_path_buf();
    // Deferred projection/finalization is outside storage acknowledgement latency.
    operation.mark_db_committed().unwrap();
    operation.commit().unwrap();
    drop(lease);
    StorageAckSample {
        path,
        total,
        admission: admission_elapsed,
        lease_wait,
        durable_storage,
    }
}

fn raw_micros(samples: &[Duration]) -> String {
    samples
        .iter()
        .map(|sample| sample.as_micros().to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[tokio::test]
#[ignore = "manual native storage benchmark; creates only isolated temporary mod folders"]
async fn native_storage_ack_benchmark() {
    eprintln!(
        "native-storage-ack limits=isolated-local-temp-filesystem,debug-build,no-ipc,no-ui,no-projection,no-runtime,no-cold-cache-control repeats={REPEAT_COUNT} warmups={WARMUP_COUNT} samples_per_repeat={SAMPLE_COUNT}"
    );
    for count in [100, 1_000, 10_000] {
        for nested in [false, true] {
            for repeat in 1..=REPEAT_COUNT {
                let fixture = tempdir().unwrap();
                let mods = fixture.path().join("Mods");
                let mut path = create_library(&mods, count, nested);
                let journal = Arc::new(
                    OperationJournal::open(fixture.path().join("journal.json"), HISTORY_LIMIT)
                        .unwrap(),
                );
                let coordinator =
                    MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
                let epoch = filesystem_identity(&mods).unwrap();
                let cold = measure_storage_ack(&coordinator, &mods, path, &epoch, 1, false).await;
                path = cold.path;
                for index in 0..WARMUP_COUNT {
                    let sample = measure_storage_ack(
                        &coordinator,
                        &mods,
                        path,
                        &epoch,
                        index as u64 + 2,
                        index % 2 == 0,
                    )
                    .await;
                    path = sample.path;
                }
                let mut totals = Vec::with_capacity(SAMPLE_COUNT);
                let mut admissions = Vec::with_capacity(SAMPLE_COUNT);
                let mut lease_waits = Vec::with_capacity(SAMPLE_COUNT);
                let mut durable_storage = Vec::with_capacity(SAMPLE_COUNT);
                for index in 0..SAMPLE_COUNT {
                    let sample = measure_storage_ack(
                        &coordinator,
                        &mods,
                        path,
                        &epoch,
                        (WARMUP_COUNT + index + 2) as u64,
                        index % 2 == 0,
                    )
                    .await;
                    path = sample.path;
                    totals.push(sample.total);
                    admissions.push(sample.admission);
                    lease_waits.push(sample.lease_wait);
                    durable_storage.push(sample.durable_storage);
                }
                let raw_totals = raw_micros(&totals);
                let raw_admissions = raw_micros(&admissions);
                let raw_lease_waits = raw_micros(&lease_waits);
                let raw_durable_storage = raw_micros(&durable_storage);
                totals.sort_unstable();
                lease_waits.sort_unstable();
                eprintln!(
                    "native-storage-ack count={count} layout={} repeat={repeat}/{REPEAT_COUNT} cold_total_us={} cold_admission_us={} cold_lease_wait_us={} cold_durable_storage_us={} warm_samples={} warm_p50_ms={:.3} warm_p95_ms={:.3} warm_p99_ms={:.3} warm_max_ms={:.3} warm_lease_wait_p95_ms={:.3} raw_total_us=[{}] raw_admission_us=[{}] raw_lease_wait_us=[{}] raw_durable_storage_us=[{}]",
                    if nested { "nested" } else { "flat" },
                    cold.total.as_micros(),
                    cold.admission.as_micros(),
                    cold.lease_wait.as_micros(),
                    cold.durable_storage.as_micros(),
                    totals.len(),
                    percentile(&totals, 50),
                    percentile(&totals, 95),
                    percentile(&totals, 99),
                    totals.last().expect("timed samples").as_secs_f64() * 1000.0,
                    percentile(&lease_waits, 95),
                    raw_totals,
                    raw_admissions,
                    raw_lease_waits,
                    raw_durable_storage,
                );
            }
        }
    }
}
