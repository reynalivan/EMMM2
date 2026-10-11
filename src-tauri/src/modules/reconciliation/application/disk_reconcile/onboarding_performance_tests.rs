use super::*;
use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::OnboardingDiskDiscovery;
use std::path::PathBuf;
use std::sync::mpsc;

const REPEATS: usize = 5;
const ROOTS: usize = 16;
const TOTAL_MODS: usize = 1_024;
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

fn owned_fixture(skewed: bool, foreground: bool) -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("owned onboarding benchmark fixture");
    let counts = if foreground {
        vec![64]
    } else if skewed {
        std::iter::once(TOTAL_MODS - (ROOTS - 1) * 8)
            .chain(std::iter::repeat_n(8, ROOTS - 1))
            .collect()
    } else {
        vec![TOTAL_MODS / ROOTS; ROOTS]
    };
    for (root, count) in counts.into_iter().enumerate() {
        for index in 0..count {
            let path = fixture
                .path()
                .join(format!("Object{root:02}"))
                .join(format!("Mod{index:04}"));
            std::fs::create_dir_all(path.join("Assets")).expect("fixture directories");
            std::fs::write(
                path.join("mod.ini"),
                "[TextureOverrideFixture]\nhash = abc\n",
            )
            .expect("fixture INI");
        }
    }
    fixture
}

#[derive(Debug)]
struct PermitSpan {
    wait: Duration,
    hold: Duration,
}

struct MeasuredPermit {
    _permit: Option<ScanPermit>,
    acquired: Instant,
    wait: Duration,
    spans: Arc<Mutex<Vec<PermitSpan>>>,
}

impl Drop for MeasuredPermit {
    fn drop(&mut self) {
        self.spans
            .lock()
            .expect("permit observations")
            .push(PermitSpan {
                wait: self.wait,
                hold: self.acquired.elapsed(),
            });
    }
}

struct PriorityOwner {
    priority: Arc<ScanPriority>,
    game_id: &'static str,
}

impl Drop for PriorityOwner {
    fn drop(&mut self) {
        self.priority.finish(self.game_id);
    }
}

fn assert_discovery_parity(actual: &DiskScopedDiscovery, expected: &DiskScopedDiscovery) {
    assert_eq!(actual.scoped, expected.scoped);
    assert_eq!(actual.scan_counts, expected.scan_counts);
    assert_eq!(
        actual.census.top_level_roots,
        expected.census.top_level_roots
    );
    assert_eq!(actual.census.entries.len(), expected.census.entries.len());
    for (actual, expected) in actual.census.entries.iter().zip(&expected.census.entries) {
        assert_eq!(actual.folder_path, expected.folder_path);
        assert_eq!(actual.folder_path_key, expected.folder_path_key);
        assert_eq!(actual.raw_name, expected.raw_name);
        assert_eq!(actual.absolute_path, expected.absolute_path);
    }
    assert_eq!(
        actual.projection.objects.len(),
        expected.projection.objects.len()
    );
    for (actual, expected) in actual
        .projection
        .objects
        .iter()
        .zip(&expected.projection.objects)
    {
        assert_eq!(actual.folder_path, expected.folder_path);
        assert_eq!(actual.folder_path_key, expected.folder_path_key);
        assert_eq!(actual.name, expected.name);
        assert_eq!(actual.is_disabled, expected.is_disabled);
        assert_eq!(actual.absolute_path, expected.absolute_path);
        assert_eq!(actual.filesystem_identity, expected.filesystem_identity);
    }
    assert_eq!(actual.projection.mods.len(), expected.projection.mods.len());
    for (actual, expected) in actual.projection.mods.iter().zip(&expected.projection.mods) {
        assert_eq!(actual.folder_path, expected.folder_path);
        assert_eq!(actual.folder_path_key, expected.folder_path_key);
        assert_eq!(
            actual.object_folder_path_key,
            expected.object_folder_path_key
        );
        assert_eq!(actual.raw_name, expected.raw_name);
        assert_eq!(actual.absolute_path, expected.absolute_path);
        assert_eq!(actual.filesystem_identity, expected.filesystem_identity);
    }
}

fn assert_progress_complete(
    events: &[OnboardingDiscoveryProgress],
    discovery: &DiskScopedDiscovery,
) {
    assert!(!events.is_empty());
    let roots = discovery.census.top_level_roots;
    let classified = discovery.scan_counts.classified_directories;
    assert!(events.iter().all(|event| {
        event.total_roots == roots
            && event.completed_roots <= roots
            && event.folders_classified <= classified
    }));
    // Raw parallel callbacks are unordered; the session reporter serializes them.
    assert!(events.iter().any(|event| {
        event.phase == OnboardingDiscoveryPhase::Classifying
            && event.is_terminal
            && event.completed_roots == roots
            && event.folders_classified == classified
    }));
}

fn span_micros(spans: &Mutex<Vec<PermitSpan>>) -> (Vec<u128>, Vec<u128>) {
    spans
        .lock()
        .expect("permit observations")
        .iter()
        .map(|span| (span.wait.as_micros(), span.hold.as_micros()))
        .unzip()
}

#[test]
#[ignore = "manual owned-fixture onboarding root/batch skew benchmark; no database or native app"]
fn benchmark_onboarding_root_batch_spans() {
    for skewed in [false, true] {
        let fixture = owned_fixture(skewed, false);
        let reference = collect_onboarding_disk_discovery_with_parallelism(
            fixture.path(),
            None,
            None,
            None,
            NonZeroUsize::MIN,
        )
        .expect("serial reference")
        .discovery;
        assert_eq!(reference.projection.mods.len(), TOTAL_MODS);
        let root_order: Vec<PathBuf> = reference
            .projection
            .objects
            .iter()
            .map(|root| root.absolute_path.clone())
            .collect();
        let root_mod_counts: Vec<usize> = reference
            .projection
            .objects
            .iter()
            .map(|root| {
                reference
                    .projection
                    .mods
                    .iter()
                    .filter(|entry| entry.object_folder_path_key == root.folder_path_key)
                    .count()
            })
            .collect();
        eprintln!(
            "onboarding_root_layout {}",
            serde_json::json!({
                "skewed": skewed,
                "root_order": root_order.iter().map(|root| root.file_name().unwrap().to_string_lossy()).collect::<Vec<_>>(),
                "root_mod_counts": root_mod_counts,
            })
        );
        for repeat in 0..REPEATS {
            for width in if repeat % 2 == 0 { [1, 8] } else { [8, 1] } {
                let events = Mutex::new(Vec::new());
                let progress = |event| events.lock().expect("progress observations").push(event);
                let spans = Arc::new(Mutex::new(Vec::new()));
                let gate = || {
                    Ok(Box::new(MeasuredPermit {
                        _permit: None,
                        acquired: Instant::now(),
                        wait: Duration::ZERO,
                        spans: Arc::clone(&spans),
                    }) as Box<dyn Send>)
                };
                let started = Instant::now();
                let actual = collect_onboarding_disk_discovery_with_parallelism(
                    fixture.path(),
                    Some(&progress),
                    Some(&gate),
                    None,
                    NonZeroUsize::new(width).unwrap(),
                )
                .expect("measured discovery")
                .discovery;
                let wall = started.elapsed();
                assert_discovery_parity(&actual, &reference);
                assert_progress_complete(&events.into_inner().unwrap(), &actual);
                let (_, hold_us) = span_micros(&spans);
                assert_eq!(hold_us.len(), ROOTS.div_ceil(width));
                eprintln!(
                    "onboarding_root_batch_spans {}",
                    serde_json::json!({
                        "skewed": skewed, "repeat": repeat, "width": width,
                        "wall_us": wall.as_micros(), "batch_hold_us": hold_us,
                        "census_work_ms": actual.timings.census_ms,
                        "classification_work_ms": actual.timings.classification_ms,
                        "mods": actual.projection.mods.len(),
                    })
                );
            }
        }
    }
}

fn background_scan(
    root: &Path,
    priority: Arc<ScanPriority>,
    cancelled: &AtomicBool,
    signal: mpsc::SyncSender<(Instant, bool)>,
    spans: Arc<Mutex<Vec<PermitSpan>>>,
    cancel_on_progress: bool,
) -> Result<OnboardingDiskDiscovery, DiskProjectionError> {
    let _owner = PriorityOwner {
        priority: Arc::clone(&priority),
        game_id: "background",
    };
    let signalled = AtomicBool::new(false);
    let progress = |event: OnboardingDiscoveryProgress| {
        if event.folders_classified > 0 && !signalled.swap(true, Ordering::AcqRel) {
            let observed_at = Instant::now();
            let busy = priority.state.lock().unwrap().busy;
            // Trigger from completed real work so fast scans cannot finish
            // before an externally scheduled cancellation thread gets CPU.
            if cancel_on_progress {
                cancelled.store(true, Ordering::Release);
                priority.cancel();
            }
            let _ = signal.try_send((observed_at, busy));
        }
    };
    let gate = || {
        let waiting = Instant::now();
        let permit = priority.acquire("background")?;
        Ok(Box::new(MeasuredPermit {
            _permit: Some(permit),
            acquired: Instant::now(),
            wait: waiting.elapsed(),
            spans: Arc::clone(&spans),
        }) as Box<dyn Send>)
    };
    collect_onboarding_disk_discovery_with_parallelism(
        root,
        Some(&progress),
        Some(&gate),
        Some(cancelled),
        ONBOARDING_ROOT_PARALLELISM,
    )
}

#[test]
#[ignore = "manual owned-fixture onboarding foreground/cancellation gate benchmark; no database or native app"]
fn benchmark_onboarding_foreground_wait_and_cancel() {
    let foreground = owned_fixture(false, true);
    let foreground_reference = collect_onboarding_disk_discovery_with_parallelism(
        foreground.path(),
        None,
        None,
        None,
        ONBOARDING_ROOT_PARALLELISM,
    )
    .unwrap()
    .discovery;
    for skewed in [false, true] {
        let background = owned_fixture(skewed, false);
        let background_reference = collect_onboarding_disk_discovery_with_parallelism(
            background.path(),
            None,
            None,
            None,
            ONBOARDING_ROOT_PARALLELISM,
        )
        .unwrap()
        .discovery;
        for repeat in 0..REPEATS {
            for cancel in if repeat % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let priority =
                    ScanPriority::new(["background".to_string(), "foreground".to_string()]);
                let cancelled = AtomicBool::new(false);
                let background_spans = Arc::new(Mutex::new(Vec::new()));
                let foreground_spans = Arc::new(Mutex::new(Vec::new()));
                std::thread::scope(|scope| {
                    let (signal_tx, signal_rx) = mpsc::sync_channel(1);
                    let (finished_tx, finished_rx) = mpsc::sync_channel(1);
                    let priority_for_worker = Arc::clone(&priority);
                    let spans_for_worker = Arc::clone(&background_spans);
                    let root = background.path();
                    let cancellation = &cancelled;
                    let worker = scope.spawn(move || {
                        let result = background_scan(
                            root,
                            priority_for_worker,
                            cancellation,
                            signal_tx,
                            spans_for_worker,
                            cancel,
                        );
                        finished_tx
                            .send((result, Instant::now()))
                            .expect("benchmark receiver");
                    });
                    let (observed_at, busy_at_observation) = signal_rx
                        .recv_timeout(PROBE_TIMEOUT)
                        .expect("background classified real folders");
                    let requested = if cancel { observed_at } else { Instant::now() };
                    let busy_at_request = if cancel {
                        busy_at_observation
                    } else {
                        priority.state.lock().unwrap().busy
                    };
                    let mut foreground_wall = Duration::ZERO;
                    if !cancel {
                        let _owner = PriorityOwner {
                            priority: Arc::clone(&priority),
                            game_id: "foreground",
                        };
                        priority.prefer("foreground");
                        let events = Mutex::new(Vec::new());
                        let progress = |event| events.lock().unwrap().push(event);
                        let gate = || {
                            let waiting = Instant::now();
                            let permit = priority.acquire("foreground")?;
                            Ok(Box::new(MeasuredPermit {
                                _permit: Some(permit),
                                acquired: Instant::now(),
                                wait: waiting.elapsed(),
                                spans: Arc::clone(&foreground_spans),
                            }) as Box<dyn Send>)
                        };
                        let actual = collect_onboarding_disk_discovery_with_parallelism(
                            foreground.path(),
                            Some(&progress),
                            Some(&gate),
                            None,
                            ONBOARDING_ROOT_PARALLELISM,
                        )
                        .expect("foreground scan")
                        .discovery;
                        foreground_wall = requested.elapsed();
                        assert_discovery_parity(&actual, &foreground_reference);
                        assert_progress_complete(&events.into_inner().unwrap(), &actual);
                    }
                    let (outcome, settled_at) = finished_rx
                        .recv_timeout(PROBE_TIMEOUT)
                        .expect("background settles within bounded probe timeout");
                    let completion_wall = settled_at.saturating_duration_since(requested);
                    worker.join().expect("background worker");
                    if cancel {
                        assert!(busy_at_request);
                        assert!(
                            matches!(outcome, Err(DiskProjectionError::Failed(message)) if message.contains("cancelled"))
                        );
                    } else {
                        assert_discovery_parity(
                            &outcome.expect("background scan").discovery,
                            &background_reference,
                        );
                    }
                    assert!(!priority.state.lock().unwrap().busy);
                    let (background_wait_us, background_hold_us) = span_micros(&background_spans);
                    let (foreground_wait_us, foreground_hold_us) = span_micros(&foreground_spans);
                    eprintln!(
                        "onboarding_foreground_gate {}",
                        serde_json::json!({
                            "skewed": skewed, "repeat": repeat, "cancel": cancel,
                        "busy_at_request": busy_at_request,
                        "background_settled_before_request": settled_at < requested,
                            "foreground_wall_us": foreground_wall.as_micros(),
                            "request_to_background_settled_us": completion_wall.as_micros(),
                            "foreground_wait_us": foreground_wait_us, "foreground_hold_us": foreground_hold_us,
                            "background_wait_us": background_wait_us, "background_hold_us": background_hold_us,
                        })
                    );
                });
            }
        }
    }
}
