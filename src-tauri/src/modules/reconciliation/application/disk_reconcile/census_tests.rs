use super::*;

#[test]
fn onboarding_reports_activity_inside_one_root() {
    let temp = tempfile::tempdir().expect("fixture");
    for index in 0..12 {
        let path = temp.path().join("Object").join(format!("Container{index}"));
        std::fs::create_dir_all(path).expect("directory");
    }
    let reports = std::sync::Mutex::new(Vec::new());
    collect_onboarding_disk_discovery_with_progress(
        temp.path(),
        Some(&|progress| {
            reports.lock().expect("reports").push(progress);
        }),
    )
    .expect("discovery");
    assert!(reports.lock().expect("reports").iter().any(|progress| {
        progress.phase == OnboardingDiscoveryPhase::Classifying
            && progress.completed_roots == 0
            && progress.folders_classified > 0
            && progress.current_root.as_deref() == Some("Object")
    }));
}

#[test]
fn onboarding_cancels_inside_a_single_root_before_it_finishes() {
    let temp = tempfile::tempdir().expect("fixture");
    for index in 0..50 {
        std::fs::create_dir_all(temp.path().join("Object").join(format!("Container{index}")))
            .expect("directory");
    }
    let cancelled = AtomicBool::new(false);
    let reports = Mutex::new(Vec::new());
    let result = collect_onboarding_disk_discovery_with_progress_and_gate(
        temp.path(),
        Some(&|progress| {
            if progress.folders_classified > 0 && progress.completed_roots == 0 {
                cancelled.store(true, Ordering::Release);
            }
            reports.lock().expect("reports").push(progress);
        }),
        None,
        Some(&cancelled),
    );
    assert!(result.is_err());
    assert!(!reports
        .lock()
        .expect("reports")
        .iter()
        .any(|event| event.is_terminal));
}
use std::num::NonZeroUsize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::time::Instant;

fn mixed_onboarding_fixture(total_mods: usize) -> tempfile::TempDir {
    mixed_onboarding_fixture_with_roots(total_mods, 1)
}

fn mixed_onboarding_fixture_with_roots(total_mods: usize, total_roots: usize) -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("synthetic onboarding fixture");
    for index in 0..total_mods {
        let object = if total_roots == 1 {
            "Object".to_string()
        } else {
            format!("Object{:02}", index % total_roots)
        };
        let path = fixture.path().join(object).join(format!("Mod{index:05}"));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("mod.ini"), "[TextureOverrideTest]\nhash = abc\n").unwrap();
        match index % 3 {
            0 => {}
            1 => {
                for name in ["VariantA", "VariantB"] {
                    let variant = path.join(name);
                    std::fs::create_dir_all(&variant).unwrap();
                    std::fs::write(
                        variant.join("mod.ini"),
                        "[TextureOverrideVariant]\nhash = abc\n",
                    )
                    .unwrap();
                }
            }
            _ => {
                let assets = path.join("Assets").join("Textures");
                std::fs::create_dir_all(&assets).unwrap();
                std::fs::write(path.join("mesh.buf"), b"asset").unwrap();
                std::fs::write(assets.join("invalid.ini"), [0xff, 0xfe, 0xfd]).unwrap();
            }
        }
    }
    fixture
}

fn assert_onboarding_parity(expected: &DiskScopedDiscovery, actual: &DiskScopedDiscovery) {
    assert_parity(&expected.census, &actual.census);
    assert_eq!(expected.scan_counts, actual.scan_counts);
    assert_eq!(expected.scoped, actual.scoped);
    let object_fields = |projection: &DiskProjection| {
        projection
            .objects
            .iter()
            .map(|entry| {
                (
                    entry.folder_path.clone(),
                    entry.folder_path_key.clone(),
                    entry.name.clone(),
                    entry.is_disabled,
                    entry.absolute_path.clone(),
                    entry.filesystem_identity.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    let mod_fields = |projection: &DiskProjection| {
        projection
            .mods
            .iter()
            .map(|entry| {
                (
                    entry.folder_path.clone(),
                    entry.folder_path_key.clone(),
                    entry.object_folder_path_key.clone(),
                    entry.raw_name.clone(),
                    entry.absolute_path.clone(),
                    entry.filesystem_identity.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        object_fields(&expected.projection),
        object_fields(&actual.projection)
    );
    assert_eq!(
        mod_fields(&expected.projection),
        mod_fields(&actual.projection)
    );
}

#[test]
fn onboarding_child_parallelism_preserves_terminal_boundaries_and_all_fields() {
    let fixture = mixed_onboarding_fixture(24);
    let expected = collect_onboarding_disk_discovery_with_parallelism(
        fixture.path(),
        None,
        None,
        None,
        NonZeroUsize::MIN,
    )
    .unwrap()
    .discovery;
    assert_eq!(expected.projection.mods.len(), 24);
    for width in [2, 4, 8] {
        let actual = collect_onboarding_disk_discovery_with_parallelism(
            fixture.path(),
            None,
            None,
            None,
            NonZeroUsize::new(width).unwrap(),
        )
        .unwrap()
        .discovery;
        assert_onboarding_parity(&expected, &actual);
    }
}

#[test]
fn onboarding_child_parallelism_preserves_mixed_roots_and_partial_root_batch() {
    for total_roots in [3, 9] {
        let fixture = mixed_onboarding_fixture_with_roots(27, total_roots);
        let expected = collect_onboarding_disk_discovery_with_parallelism(
            fixture.path(),
            None,
            None,
            None,
            NonZeroUsize::MIN,
        )
        .unwrap()
        .discovery;
        assert_eq!(expected.census.top_level_roots, total_roots);
        assert_eq!(expected.projection.mods.len(), 27);
        for width in [2, 4, 8] {
            let actual = collect_onboarding_disk_discovery_with_parallelism(
                fixture.path(),
                None,
                None,
                None,
                NonZeroUsize::new(width).unwrap(),
            )
            .unwrap()
            .discovery;
            assert_onboarding_parity(&expected, &actual);
        }
    }
}

#[test]
fn onboarding_child_failure_never_emits_terminal_success() {
    for width in [1, 2, 4, 8] {
        let fixture = mixed_onboarding_fixture(24);
        std::fs::write(
            fixture.path().join("Object/Mod00012/mod.ini"),
            [0xff, 0xfe, 0xfd],
        )
        .unwrap();
        let events = Mutex::new(Vec::new());
        let result = collect_onboarding_disk_discovery_with_parallelism(
            fixture.path(),
            Some(&|event| events.lock().unwrap().push(event)),
            None,
            None,
            NonZeroUsize::new(width).unwrap(),
        );
        assert!(matches!(result, Err(DiskProjectionError::Failed(_))));
        assert!(events
            .into_inner()
            .unwrap()
            .iter()
            .all(|event| !event.is_terminal));
    }
}

#[test]
fn onboarding_does_not_count_a_folder_before_classification_succeeds() {
    for width in [1, 2, 4, 8] {
        let fixture = tempfile::tempdir().unwrap();
        let folder = fixture.path().join("Object/BadMod");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("mod.ini"), [0xff, 0xfe, 0xfd]).unwrap();
        let events = Mutex::new(Vec::new());
        let result = collect_onboarding_disk_discovery_with_parallelism(
            fixture.path(),
            Some(&|event| events.lock().unwrap().push(event)),
            None,
            None,
            NonZeroUsize::new(width).unwrap(),
        );
        assert!(matches!(result, Err(DiskProjectionError::Failed(_))));
        assert!(events
            .into_inner()
            .unwrap()
            .iter()
            .all(|event| event.folders_classified == 0));
    }
}

#[test]
fn onboarding_removed_root_never_returns_partial_discovery() {
    for width in [1, 2, 4, 8] {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("Object");
        std::fs::create_dir(&root).unwrap();
        let removed = AtomicBool::new(false);
        let events = Mutex::new(Vec::new());
        let progress = |event: OnboardingDiscoveryProgress| {
            if event.phase == OnboardingDiscoveryPhase::Classifying
                && !removed.swap(true, Ordering::Relaxed)
            {
                assert_eq!(root.parent(), Some(fixture.path()));
                std::fs::remove_dir(&root).unwrap();
            }
            events.lock().unwrap().push(event);
        };
        let result = collect_onboarding_disk_discovery_with_parallelism(
            fixture.path(),
            Some(&progress),
            None,
            None,
            NonZeroUsize::new(width).unwrap(),
        );
        assert!(matches!(
            result,
            Err(DiskProjectionError::SourceUnavailable(_))
        ));
        assert!(events
            .into_inner()
            .unwrap()
            .iter()
            .all(|event| !event.is_terminal));
    }
}

#[test]
#[ignore = "manual synthetic single-root classification benchmark"]
fn benchmark_onboarding_single_root_classification() {
    let fixture = mixed_onboarding_fixture(2_000);
    let expected = collect_onboarding_disk_discovery_with_parallelism(
        fixture.path(),
        None,
        None,
        None,
        NonZeroUsize::MIN,
    )
    .unwrap()
    .discovery;
    let mut samples = BTreeMap::<usize, Vec<Duration>>::new();
    for round in 0..5 {
        let order = if round % 2 == 0 {
            [1, 2, 4, 8]
        } else {
            [8, 4, 2, 1]
        };
        for width in order {
            let started = Instant::now();
            let actual = collect_onboarding_disk_discovery_with_parallelism(
                fixture.path(),
                None,
                None,
                None,
                NonZeroUsize::new(width).unwrap(),
            )
            .unwrap()
            .discovery;
            let elapsed = started.elapsed();
            assert_onboarding_parity(&expected, &actual);
            samples.entry(width).or_default().push(elapsed);
            eprintln!("single_root_trial round={} width={} mods=2000 elapsed_ms={:.3} census_work_ms={} classification_work_ms={} parity=exact", round + 1, width, elapsed.as_secs_f64() * 1_000.0, actual.timings.census_ms, actual.timings.classification_ms);
        }
    }
    for (width, mut values) in samples {
        values.sort();
        eprintln!(
            "single_root_summary width={} samples={} min_ms={:.3} median_ms={:.3} max_ms={:.3}",
            width,
            values.len(),
            values[0].as_secs_f64() * 1_000.0,
            values[values.len() / 2].as_secs_f64() * 1_000.0,
            values[values.len() - 1].as_secs_f64() * 1_000.0
        );
    }
}

const CENSUS_TEST_WIDTHS: [usize; 3] = [1, 2, 4];
type CensusFields<'a> = (&'a str, &'a str, &'a str, &'a Path);

fn fields(census: &DiskIdentityCensus) -> Vec<CensusFields<'_>> {
    census
        .entries
        .iter()
        .map(|entry| {
            (
                entry.folder_path.as_str(),
                entry.folder_path_key.as_str(),
                entry.raw_name.as_str(),
                entry.absolute_path.as_path(),
            )
        })
        .collect()
}

fn serial_reference(mods_path: &Path) -> DiskProjectionResult<DiskIdentityCensus> {
    let roots = list_runtime_dirs(mods_path)?;
    let mut entries = Vec::new();
    for root in &roots {
        let mut pending = vec![root.clone()];
        while let Some(path) = pending.pop() {
            let folder_path = relative_path_string(mods_path, &path)?;
            entries.push(DiskIdentityCensusEntry {
                folder_path_key: crate::shared::path_key::folder_path_key(&folder_path, None),
                folder_path,
                raw_name: runtime_dir_name(&path),
                absolute_path: path.clone(),
            });
            pending.extend(list_runtime_dirs(&path)?);
        }
    }
    entries.sort_by(|left, right| {
        left.folder_path
            .to_ascii_lowercase()
            .cmp(&right.folder_path.to_ascii_lowercase())
    });
    Ok(DiskIdentityCensus {
        entries,
        top_level_roots: roots.len(),
    })
}

fn assert_parity(expected: &DiskIdentityCensus, actual: &DiskIdentityCensus) {
    assert_eq!(expected.top_level_roots, actual.top_level_roots);
    assert_eq!(fields(expected), fields(actual));
}

type CensusSorter = fn(&mut [DiskIdentityCensusEntry]);

fn comparator_sort(entries: &mut [DiskIdentityCensusEntry]) {
    entries.sort_by(|left, right| {
        left.folder_path
            .to_ascii_lowercase()
            .cmp(&right.folder_path.to_ascii_lowercase())
    });
}

fn cached_key_sort(entries: &mut [DiskIdentityCensusEntry]) {
    entries.sort_by_cached_key(|entry| entry.folder_path.to_ascii_lowercase());
}

fn capture_unsorted_census(mods_path: &Path) -> DiskProjectionResult<DiskIdentityCensus> {
    let roots = list_runtime_dirs(mods_path)?;
    let mut entries = Vec::new();
    for batch in roots.chunks(CENSUS_ROOT_PARALLELISM.get()) {
        let results: Vec<_> = batch
            .par_iter()
            .map(|root| collect_census_root(mods_path, root))
            .collect();
        for result in results {
            entries.extend(result?);
        }
    }
    Ok(DiskIdentityCensus {
        entries,
        top_level_roots: roots.len(),
    })
}

#[test]
fn cached_census_sort_preserves_case_equal_ties_and_all_fields() {
    let paths = [
        "Zulu/Last",
        "Alice/Skin",
        "ALICE/SKIN",
        "alice/skin",
        "alice/SKIn",
        "Älpha/Skin",
        "älpha/Skin",
        "DISABLED Alice/Skin",
    ];
    let captured = DiskIdentityCensus {
        entries: paths
            .iter()
            .enumerate()
            .map(|(index, path)| DiskIdentityCensusEntry {
                folder_path: (*path).to_string(),
                folder_path_key: format!("source-key-{index}"),
                raw_name: format!("source-order-{index}"),
                absolute_path: PathBuf::from(path),
            })
            .collect(),
        top_level_roots: paths.len(),
    };
    let mut original = captured.clone();
    comparator_sort(&mut original.entries);
    let mut cached = captured.clone();
    cached_key_sort(&mut cached.entries);
    assert_parity(&original, &cached);
    let mut production = captured;
    sort_census_entries(&mut production.entries);
    assert_parity(&original, &production);
    let tied: Vec<_> = production
        .entries
        .iter()
        .filter(|entry| entry.folder_path.eq_ignore_ascii_case("alice/skin"))
        .map(|entry| entry.raw_name.as_str())
        .collect();
    assert_eq!(
        tied,
        [
            "source-order-1",
            "source-order-2",
            "source-order-3",
            "source-order-4"
        ]
    );
}

#[test]
#[ignore = "manual read-only captured-key and full directory census sort comparison"]
fn benchmark_readonly_census_sort_keys() {
    let root = std::env::var_os("EMMM_AUDIT_MODS_ROOT")
        .map(PathBuf::from)
        .expect("Set EMMM_AUDIT_MODS_ROOT to the authorized read-only Mods directory");
    assert!(root.is_absolute() && root.is_dir());
    let captured = capture_unsorted_census(&root).unwrap();
    let mut expected = captured.clone();
    comparator_sort(&mut expected.entries);
    assert_parity(&expected, &collect_disk_identity_census(&root).unwrap());
    eprintln!(
        "census_sort_root={} roots={} directories={} cloned_outside_sort_timer=true fields_checked=4",
        root.display(),
        captured.top_level_roots,
        captured.entries.len()
    );
    let strategies: [(&str, CensusSorter); 2] = [
        ("comparator", comparator_sort),
        ("cached_keys", cached_key_sort),
    ];
    let mut timings = BTreeMap::<(&str, &str), Vec<Duration>>::new();
    for scope in ["captured_sort_only", "full_census"] {
        for round in 0..7 {
            let order = if round % 2 == 0 { [0, 1] } else { [1, 0] };
            for index in order {
                let (label, sort) = strategies[index];
                let (actual, elapsed) = if scope == "captured_sort_only" {
                    let mut sample = captured.clone();
                    let started = Instant::now();
                    sort(&mut sample.entries);
                    (sample, started.elapsed())
                } else {
                    let started = Instant::now();
                    let mut sample = capture_unsorted_census(&root).unwrap();
                    sort(&mut sample.entries);
                    (sample, started.elapsed())
                };
                assert_parity(&expected, &actual);
                timings.entry((scope, label)).or_default().push(elapsed);
                eprintln!(
                    "census_sort_trial scope={} round={} mode={} elapsed_ms={:.3} parity=exact",
                    scope,
                    round + 1,
                    label,
                    elapsed.as_secs_f64() * 1_000.0
                );
            }
        }
    }
    for ((scope, label), mut samples) in timings {
        samples.sort();
        eprintln!(
            "census_sort_summary scope={} mode={} samples={} min_ms={:.3} median_ms={:.3} max_ms={:.3}",
            scope,
            label,
            samples.len(),
            samples[0].as_secs_f64() * 1_000.0,
            samples[samples.len() / 2].as_secs_f64() * 1_000.0,
            samples[samples.len() - 1].as_secs_f64() * 1_000.0
        );
    }
}

#[test]
fn bounded_census_preserves_all_fields_hidden_disabled_and_ordered_progress() {
    let fixture = tempfile::tempdir().unwrap();
    for relative in [
        "Alpha/Normal/Assets",
        "Alpha/DISABLED_pack/Skin",
        "Alpha/.hidden/NeverEnumerate",
        "DISABLED Beta/Enabled Skin",
        "γamma/UNICODE_衣装/Textures",
        ".root-hidden/NeverEnumerate",
    ] {
        std::fs::create_dir_all(fixture.path().join(relative)).unwrap();
    }
    std::fs::write(fixture.path().join("Alpha/Normal/asset.dds"), b"asset").unwrap();
    std::fs::write(fixture.path().join("Alpha/Normal/mod.ini"), b"invalid INI").unwrap();
    let expected = serial_reference(fixture.path()).unwrap();
    assert_parity(
        &expected,
        &collect_disk_identity_census(fixture.path()).unwrap(),
    );
    assert_eq!(expected.top_level_roots, 3);
    assert!(expected
        .entries
        .iter()
        .any(|entry| entry.raw_name == "DISABLED_pack"));
    assert!(expected
        .entries
        .iter()
        .all(|entry| !entry.raw_name.starts_with('.')));
    let mut first_progress = None;
    for width in CENSUS_TEST_WIDTHS {
        let events = Mutex::new(Vec::new());
        let progress = |event| events.lock().unwrap().push(event);
        let actual = collect_disk_identity_census_with_parallelism(
            fixture.path(),
            Some(&progress),
            NonZeroUsize::new(width).unwrap(),
        )
        .unwrap();
        assert_parity(&expected, &actual);
        let events = events.into_inner().unwrap();
        assert_eq!(events.len(), expected.top_level_roots + 1);
        for (completed, event) in events.iter().enumerate() {
            assert_eq!(event.completed_roots, completed);
            assert_eq!(event.total_roots, expected.top_level_roots);
            assert_eq!(event.folders_classified, 0);
            assert_eq!(event.phase, OnboardingDiscoveryPhase::Metadata);
            assert_eq!(event.is_terminal, completed == expected.top_level_roots);
        }
        match &first_progress {
            Some(first) => assert_eq!(first, &events),
            None => first_progress = Some(events),
        }
    }
}

#[test]
fn bounded_census_empty_root_has_one_terminal_event() {
    let fixture = tempfile::tempdir().unwrap();
    for width in CENSUS_TEST_WIDTHS {
        let events = Mutex::new(Vec::new());
        let progress = |event| events.lock().unwrap().push(event);
        let census = collect_disk_identity_census_with_parallelism(
            fixture.path(),
            Some(&progress),
            NonZeroUsize::new(width).unwrap(),
        )
        .unwrap();
        assert!(census.entries.is_empty());
        let events = events.into_inner().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events.iter().filter(|event| event.is_terminal).count(), 1);
        assert!(events.iter().all(|event| event.completed_roots == 0));
    }
}

#[test]
fn bounded_census_failure_preserves_serial_progress_prefix() {
    for width in CENSUS_TEST_WIDTHS {
        let fixture = tempfile::tempdir().unwrap();
        for name in ["First", "Second", "Third"] {
            std::fs::create_dir(fixture.path().join(name)).unwrap();
        }
        let roots = list_runtime_dirs(fixture.path()).unwrap();
        let removed = AtomicBool::new(false);
        let events = Mutex::new(Vec::new());
        let progress = |event: OnboardingDiscoveryProgress| {
            if event.completed_roots == 0 && !removed.swap(true, Ordering::Relaxed) {
                assert_eq!(roots[1].parent(), Some(fixture.path()));
                std::fs::remove_dir(&roots[1]).unwrap();
            }
            events.lock().unwrap().push(event);
        };
        let result = collect_disk_identity_census_with_parallelism(
            fixture.path(),
            Some(&progress),
            NonZeroUsize::new(width).unwrap(),
        );
        assert!(matches!(
            result,
            Err(DiskProjectionError::SourceUnavailable(_))
        ));
        let events = events.into_inner().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].completed_roots, 0);
        assert_eq!(events[1].completed_roots, 1);
        assert_eq!(
            events[1].current_root.as_deref(),
            Some(runtime_dir_name(&roots[0]).as_str())
        );
        assert!(events.iter().all(|event| !event.is_terminal));
    }
}

#[test]
fn bounded_census_unavailable_source_emits_no_progress() {
    let fixture = tempfile::tempdir().unwrap();
    let missing = fixture.path().join("missing");
    let events = Mutex::new(Vec::new());
    let progress = |event| events.lock().unwrap().push(event);
    assert!(matches!(
        collect_disk_identity_census_with_parallelism(&missing, Some(&progress), NonZeroUsize::MIN),
        Err(DiskProjectionError::SourceUnavailable(_))
    ));
    assert!(events.into_inner().unwrap().is_empty());
}

#[cfg(windows)]
#[test]
fn bounded_census_preserves_native_directory_link_policy() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("Mods");
    let target = fixture.path().join("OwnedTarget/Assets");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&target).unwrap();
    match std::os::windows::fs::symlink_dir(target.parent().unwrap(), root.join("Link")) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("Directory link policy fixture unavailable: {error}");
            return;
        }
        Err(error) => panic!("Directory link fixture failed: {error}"),
    }
    std::fs::create_dir(root.join("Ordinary")).unwrap();
    let expected = serial_reference(&root).unwrap();
    for width in CENSUS_TEST_WIDTHS {
        let actual = collect_disk_identity_census_with_parallelism(
            &root,
            None,
            NonZeroUsize::new(width).unwrap(),
        )
        .unwrap();
        assert_parity(&expected, &actual);
    }
}

#[test]
#[ignore = "manual read-only directory census benchmark on an explicitly configured library"]
fn benchmark_readonly_census_parallelism() {
    let root = std::env::var_os("EMMM_AUDIT_MODS_ROOT")
        .map(PathBuf::from)
        .expect("Set EMMM_AUDIT_MODS_ROOT to the authorized read-only Mods directory");
    assert!(root.is_absolute() && root.is_dir());
    let expected = serial_reference(&root).expect("serial census should succeed");
    eprintln!(
        "census_root={} roots={} directories={} fields_checked=4",
        root.display(),
        expected.top_level_roots,
        expected.entries.len()
    );
    for width in CENSUS_TEST_WIDTHS {
        let actual = collect_disk_identity_census_with_parallelism(
            &root,
            None,
            NonZeroUsize::new(width).unwrap(),
        )
        .unwrap();
        assert_parity(&expected, &actual);
    }
    let orders = [[1, 2, 4], [4, 1, 2], [2, 4, 1], [4, 2, 1], [1, 4, 2]];
    let mut samples = BTreeMap::<usize, Vec<Duration>>::new();
    for (round, order) in orders.iter().enumerate() {
        for &width in order {
            let started = Instant::now();
            let actual = collect_disk_identity_census_with_parallelism(
                &root,
                None,
                NonZeroUsize::new(width).unwrap(),
            )
            .unwrap();
            let elapsed = started.elapsed();
            assert_parity(&expected, &actual);
            samples.entry(width).or_default().push(elapsed);
            eprintln!(
                "census_trial round={} width={} elapsed_ms={:.3} parity=exact",
                round + 1,
                width,
                elapsed.as_secs_f64() * 1_000.0
            );
        }
    }
    for (width, mut values) in samples {
        values.sort();
        eprintln!(
            "census_summary width={} samples={} min_ms={:.3} median_ms={:.3} max_ms={:.3}",
            width,
            values.len(),
            values[0].as_secs_f64() * 1_000.0,
            values[values.len() / 2].as_secs_f64() * 1_000.0,
            values[values.len() - 1].as_secs_f64() * 1_000.0
        );
    }
    let mut reference_samples = Vec::new();
    let mut bounded_samples = Vec::new();
    for round in 0..5 {
        let order = if round % 2 == 0 {
            [true, false]
        } else {
            [false, true]
        };
        for reference in order {
            let started = Instant::now();
            let actual = if reference {
                serial_reference(&root)
            } else {
                collect_disk_identity_census(&root)
            }
            .unwrap();
            let elapsed = started.elapsed();
            assert_parity(&expected, &actual);
            let label = if reference {
                reference_samples.push(elapsed);
                "original_serial"
            } else {
                bounded_samples.push(elapsed);
                "production_bounded"
            };
            eprintln!(
                "census_control round={} mode={} elapsed_ms={:.3} parity=exact",
                round + 1,
                label,
                elapsed.as_secs_f64() * 1_000.0
            );
        }
    }
    for (label, mut samples) in [
        ("original_serial", reference_samples),
        ("production_bounded", bounded_samples),
    ] {
        samples.sort();
        eprintln!(
            "census_control_summary mode={} samples={} min_ms={:.3} median_ms={:.3} max_ms={:.3}",
            label,
            samples.len(),
            samples[0].as_secs_f64() * 1_000.0,
            samples[samples.len() / 2].as_secs_f64() * 1_000.0,
            samples[samples.len() - 1].as_secs_f64() * 1_000.0
        );
    }
}

fn normal_projection_shape(projection: &DiskProjection) -> serde_json::Value {
    serde_json::json!({
        "objects": projection.objects.iter().map(|entry| serde_json::json!({
            "folder_path": entry.folder_path,
            "folder_path_key": entry.folder_path_key,
            "name": entry.name,
            "is_disabled": entry.is_disabled,
            "absolute_path": entry.absolute_path,
            "filesystem_identity": entry.filesystem_identity,
        })).collect::<Vec<_>>(),
        "mods": projection.mods.iter().map(|entry| serde_json::json!({
            "folder_path": entry.folder_path,
            "folder_path_key": entry.folder_path_key,
            "object_folder_path_key": entry.object_folder_path_key,
            "raw_name": entry.raw_name,
            "absolute_path": entry.absolute_path,
            "filesystem_identity": entry.filesystem_identity,
        })).collect::<Vec<_>>(),
    })
}

// Freeze the pre-reuse terminal walk so parity and timings do not accidentally
// compare the production collector with itself after a performance refactor.
fn normal_terminal_mods_original_reference(
    mods: &mut Vec<DiskModEntry>,
    mods_path: &Path,
    object_folder_path_key: &str,
    path: &Path,
    classified_directories: &AtomicUsize,
) -> DiskProjectionResult<()> {
    classified_directories.fetch_add(1, Ordering::Relaxed);
    let node_type = if path.is_dir() {
        let scan = crate::modules::workspace::domain::classifier::scan_folder_strict(path)
            .map_err(|error| DiskProjectionError::Failed(error.to_string()))?;
        let (node_type, _, _) =
            crate::modules::workspace::domain::classifier::classify_folder_strict_from_scan(&scan)
                .map_err(|error| DiskProjectionError::Failed(error.to_string()))?;
        node_type
    } else {
        NodeType::ContainerFolder
    };
    match node_type {
        NodeType::ModPackRoot | NodeType::FlatModRoot | NodeType::VariantContainer => {
            let folder_path = relative_path_string(mods_path, path)?;
            let raw_name = path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .ok_or_else(|| {
                    DiskProjectionError::Failed(format!(
                        "Disk Reconcile mod path has no name: {}",
                        path.display()
                    ))
                })?;
            mods.push(DiskModEntry {
                folder_path_key: crate::shared::path_key::folder_path_key(&folder_path, None),
                folder_path,
                object_folder_path_key: object_folder_path_key.to_string(),
                raw_name,
                absolute_path: path.to_path_buf(),
                filesystem_identity: filesystem_identity(path),
            });
        }
        NodeType::InternalAssets => {}
        NodeType::ContainerFolder => {
            for child in list_runtime_dirs(path)? {
                normal_terminal_mods_original_reference(
                    mods,
                    mods_path,
                    object_folder_path_key,
                    &child,
                    classified_directories,
                )?;
            }
        }
    }
    Ok(())
}

fn normal_projection_serial_reference(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
) -> DiskProjectionResult<(DiskProjection, usize)> {
    let target_roots = if scoped {
        changed_roots
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|name| (name.clone(), mods_path.join(name)))
            .collect::<Vec<_>>()
    } else {
        list_runtime_dirs(mods_path)?
            .into_iter()
            .map(|path| (runtime_dir_name(&path), path))
            .collect::<Vec<_>>()
    };
    let classified_directories = AtomicUsize::new(0);
    let per_root = target_roots
        .par_iter()
        .map(|(name, path)| {
            if !path.exists() || !path.is_dir() {
                return Ok(None);
            }
            let object = DiskObjectEntry {
                folder_path: name.clone(),
                folder_path_key: crate::shared::path_key::folder_path_key(name, None),
                name: normalize_display_name(name).into_owned(),
                is_disabled: is_disabled_folder(name),
                absolute_path: path.clone(),
                filesystem_identity: filesystem_identity(path),
            };
            let children = list_runtime_dirs(path)?;
            let mut mods = Vec::new();
            for child in children {
                normal_terminal_mods_original_reference(
                    &mut mods,
                    mods_path,
                    &object.folder_path_key,
                    &child,
                    &classified_directories,
                )?;
            }
            Ok(Some((object, mods)))
        })
        .collect::<DiskProjectionResult<Vec<_>>>()?;
    let mut projection = DiskProjection::default();
    for (object, mods) in per_root.into_iter().flatten() {
        projection.objects.push(object);
        projection.mods.extend(mods);
    }
    Ok((projection, classified_directories.load(Ordering::Relaxed)))
}

fn normal_sync_risk_fixture(total_roots: usize) -> tempfile::TempDir {
    let fixture = mixed_onboarding_fixture_with_roots(27, total_roots);
    let first_root = fixture.path().join(if total_roots == 1 {
        "Object"
    } else {
        "Object00"
    });
    let renamed = fixture.path().join("DISABLED Ω");
    assert_eq!(first_root.parent(), Some(fixture.path()));
    assert_eq!(renamed.parent(), Some(fixture.path()));
    std::fs::rename(first_root, &renamed).unwrap();
    let nested = renamed.join("Nested/深层/DISABLED 衣装");
    std::fs::create_dir_all(nested.join("Assets/Textures")).unwrap();
    std::fs::write(nested.join("mod.ini"), "[TextureOverrideTest]\nhash=abc\n").unwrap();
    std::fs::write(nested.join("mesh.buf"), b"asset").unwrap();
    std::fs::write(
        nested.join("Assets/Textures/invalid.ini"),
        [0xff, 0xfe, 0xfd],
    )
    .unwrap();
    for hidden in [
        fixture.path().join(".hidden/Never"),
        renamed.join(".cache/Never"),
    ] {
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(hidden.join("mod.ini"), [0xff, 0xfe, 0xfd]).unwrap();
    }
    fixture
}

#[test]
fn normal_scan_reuse_preserves_nested_container_fields_order_and_errors() {
    let fixture = normal_sync_risk_fixture(3);
    let root = fixture.path().join("DISABLED Ω");
    for index in 0..12 {
        let container = root.join(format!("Container{index:02}"));
        let terminal = container.join("深層/DISABLED 衣装");
        std::fs::create_dir_all(&terminal).unwrap();
        std::fs::create_dir_all(container.join("Empty/MoreEmpty")).unwrap();
        std::fs::create_dir_all(container.join(".hidden/Never")).unwrap();
        std::fs::write(
            terminal.join("mod.ini"),
            "[TextureOverrideTest]\nhash=abc\n",
        )
        .unwrap();
        std::fs::write(container.join("desktop.ini"), [0xff, 0xfe, 0xfd]).unwrap();
        std::fs::write(container.join(".hidden/Never/mod.ini"), [0xff, 0xfe, 0xfd]).unwrap();
    }
    for scoped in [false, true] {
        let changed = ["DISABLED Ω".to_string()];
        let (expected, expected_count) =
            normal_projection_serial_reference(fixture.path(), &changed, scoped).unwrap();
        for width in [1, 2, 4, 8] {
            let (actual, actual_count) = collect_disk_projection_with_parallelism(
                fixture.path(),
                &changed,
                scoped,
                None,
                NonZeroUsize::new(width).unwrap(),
            )
            .unwrap();
            assert_eq!(
                normal_projection_shape(&expected),
                normal_projection_shape(&actual)
            );
            assert_eq!(expected_count, actual_count);
        }
    }
    let bad_ini = root.join("Container00/深層/DISABLED 衣装/mod.ini");
    std::fs::write(&bad_ini, [0xff, 0xfe, 0xfd]).unwrap();
    let expected = normal_projection_serial_reference(fixture.path(), &[], false)
        .unwrap_err()
        .into_message();
    let actual = collect_disk_projection(fixture.path(), &[], false)
        .unwrap_err()
        .into_message();
    assert_eq!(expected, actual);
    assert!(actual.contains("encoding"));
}

#[test]
fn normal_scan_reuse_preserves_missing_and_file_path_errors() {
    let fixture = tempfile::tempdir().unwrap();
    let file = fixture.path().join("File");
    std::fs::write(&file, b"not a directory").unwrap();
    for path in [fixture.path().join("Missing"), file] {
        let original_count = AtomicUsize::new(0);
        let actual_count = AtomicUsize::new(0);
        let expected = normal_terminal_mods_original_reference(
            &mut Vec::new(),
            fixture.path(),
            "object",
            &path,
            &original_count,
        )
        .unwrap_err();
        let actual = collect_terminal_mods(
            &mut Vec::new(),
            fixture.path(),
            "object",
            &path,
            &actual_count,
        )
        .unwrap_err();
        assert!(matches!(
            expected,
            DiskProjectionError::SourceUnavailable(_)
        ));
        assert!(matches!(actual, DiskProjectionError::SourceUnavailable(_)));
        assert_eq!(expected.into_message(), actual.into_message());
        assert_eq!(original_count.load(Ordering::Relaxed), 1);
        assert_eq!(actual_count.load(Ordering::Relaxed), 1);
    }
}

#[cfg(windows)]
#[test]
fn normal_scan_reuse_preserves_native_directory_link_policy() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("Mods");
    let container = root.join("Object/Container");
    let target = fixture.path().join("OwnedTarget/Terminal");
    std::fs::create_dir_all(container.join("Visible")).unwrap();
    std::fs::create_dir_all(&target).unwrap();
    for terminal in [container.join("Visible"), target] {
        std::fs::write(
            terminal.join("mod.ini"),
            "[TextureOverrideTest]\nhash=abc\n",
        )
        .unwrap();
    }
    match std::os::windows::fs::symlink_dir(
        fixture.path().join("OwnedTarget"),
        container.join("Link"),
    ) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("Directory link policy fixture unavailable: {error}");
            return;
        }
        Err(error) => panic!("Directory link fixture failed: {error}"),
    }
    let (expected, expected_count) = normal_projection_serial_reference(&root, &[], false).unwrap();
    assert_eq!(expected.mods.len(), 1);
    for width in [1, 2, 4, 8] {
        let (actual, actual_count) = collect_disk_projection_with_parallelism(
            &root,
            &[],
            false,
            None,
            NonZeroUsize::new(width).unwrap(),
        )
        .unwrap();
        assert_eq!(
            normal_projection_shape(&expected),
            normal_projection_shape(&actual)
        );
        assert_eq!(expected_count, actual_count);
    }
}

#[test]
fn normal_sync_child_parallelism_preserves_full_scoped_fields_order_and_progress() {
    for total_roots in [1, 3, 9] {
        let fixture = normal_sync_risk_fixture(total_roots);
        for scoped in [false, true] {
            let changed_roots = if scoped {
                vec!["DISABLED Ω".to_string(), "DISABLED Ω".to_string()]
            } else {
                Vec::new()
            };
            let (expected, expected_classified) =
                normal_projection_serial_reference(fixture.path(), &changed_roots, scoped).unwrap();
            assert!(expected
                .mods
                .iter()
                .any(|entry| entry.raw_name == "DISABLED 衣装"));
            assert!(expected.objects.iter().any(|entry| entry.is_disabled));
            assert!(expected
                .mods
                .iter()
                .all(|entry| !entry.folder_path.contains(".cache")
                    && !entry.folder_path.contains("Assets")
                    && !entry.folder_path.contains("VariantA")));
            for width in [1, 2, 4, 8] {
                let events = Mutex::new(Vec::new());
                let progress = |event| events.lock().unwrap().push(event);
                let (actual, classified) = collect_disk_projection_with_parallelism(
                    fixture.path(),
                    &changed_roots,
                    scoped,
                    Some(&progress),
                    NonZeroUsize::new(width).unwrap(),
                )
                .unwrap();
                assert_eq!(
                    normal_projection_shape(&expected),
                    normal_projection_shape(&actual)
                );
                assert_eq!(expected_classified, classified);
                let reports = events.into_inner().unwrap();
                let roots = expected.objects.len();
                assert_eq!(reports.len(), roots + 1);
                assert_eq!(reports[0].completed_roots, 0);
                assert_eq!(reports[0].classified_directories, 0);
                assert!(reports.iter().all(|event| event.total_roots == roots));
                let complete = reports
                    .iter()
                    .find(|event| event.completed_roots == roots)
                    .unwrap();
                assert_eq!(complete.classified_directories, classified);
            }
        }
    }
}

#[test]
fn normal_sync_child_parallelism_rejects_invalid_ini_without_complete_progress() {
    for width in [1, 2, 4, 8] {
        let fixture = normal_sync_risk_fixture(3);
        std::fs::write(
            fixture.path().join("DISABLED Ω/Mod00000/mod.ini"),
            [0xff, 0xfe, 0xfd],
        )
        .unwrap();
        for scoped in [false, true] {
            let events = Mutex::new(Vec::new());
            let progress = |event| events.lock().unwrap().push(event);
            let result = collect_disk_projection_with_parallelism(
                fixture.path(),
                &["DISABLED Ω".to_string()],
                scoped,
                Some(&progress),
                NonZeroUsize::new(width).unwrap(),
            );
            assert!(
                matches!(result, Err(DiskProjectionError::Failed(message)) if message.contains("encoding"))
            );
            assert!(events
                .into_inner()
                .unwrap()
                .iter()
                .all(|event| event.completed_roots < event.total_roots));
        }
    }
}

#[test]
fn normal_sync_child_parallelism_handles_empty_collection_and_empty_root() {
    let fixture = tempfile::tempdir().unwrap();
    for roots in [0, 1] {
        if roots == 1 {
            std::fs::create_dir(fixture.path().join("EmptyObject")).unwrap();
        }
        for width in [1, 2, 4, 8] {
            let (projection, classified) = collect_disk_projection_with_parallelism(
                fixture.path(),
                &[],
                false,
                None,
                NonZeroUsize::new(width).unwrap(),
            )
            .unwrap();
            assert_eq!(projection.objects.len(), roots);
            assert!(projection.mods.is_empty());
            assert_eq!(classified, 0);
        }
    }
}

#[test]
fn normal_sync_child_parallelism_rejects_removed_child_without_partial_success() {
    let fixture = mixed_onboarding_fixture(3);
    let children = [
        fixture.path().join("Object/Mod00000"),
        fixture.path().join("Object/Missing"),
    ];
    for width in [1, 2, 4, 8] {
        let classified = AtomicUsize::new(0);
        let result = collect_terminal_mod_children(
            fixture.path(),
            "object",
            &children,
            &classified,
            NonZeroUsize::new(width).unwrap(),
        );
        assert!(matches!(
            result,
            Err(DiskProjectionError::SourceUnavailable(_))
        ));
    }
}

#[test]
#[ignore = "manual synthetic normal-sync production classification versus original serial reference"]
fn benchmark_normal_sync_child_classification() {
    eprintln!("normal_sync_context pool_threads={} cache=warm_uncontrolled progress_callback=false baseline=test_only_serial candidate=production existing_root_parallelism=unchanged", rayon::current_num_threads());
    for (scenario, total_mods, total_roots) in [
        ("single_large", 2_000, 1),
        ("many_small", 64, 32),
        ("balanced", 2_048, 16),
    ] {
        let fixture = mixed_onboarding_fixture_with_roots(total_mods, total_roots);
        let (expected, classified) =
            normal_projection_serial_reference(fixture.path(), &[], false).unwrap();
        let expected_shape = normal_projection_shape(&expected);
        assert_eq!(expected.mods.len(), total_mods);
        let scoped_modes: &[bool] = if total_roots == 1 {
            &[false, true]
        } else {
            &[false]
        };
        for &scoped in scoped_modes {
            let changed_roots = if scoped {
                vec!["Object".to_string()]
            } else {
                Vec::new()
            };
            let mut timings = BTreeMap::<usize, Vec<Duration>>::new();
            for round in 0..5 {
                let order = if round % 2 == 0 {
                    [0, 1, 2, 4, 8]
                } else {
                    [8, 4, 2, 1, 0]
                };
                for width in order {
                    let started = Instant::now();
                    let (actual, actual_classified) = if width == 0 {
                        normal_projection_serial_reference(fixture.path(), &changed_roots, scoped)
                    } else {
                        collect_disk_projection_with_parallelism(
                            fixture.path(),
                            &changed_roots,
                            scoped,
                            None,
                            NonZeroUsize::new(width).unwrap(),
                        )
                    }
                    .unwrap();
                    let elapsed = started.elapsed();
                    assert_eq!(classified, actual_classified);
                    assert_eq!(expected_shape, normal_projection_shape(&actual));
                    timings.entry(width).or_default().push(elapsed);
                    eprintln!("normal_sync_trial scenario={} scoped={} round={} width={} roots={} mods={} classified={} elapsed_ms={:.3} parity=exact", scenario, scoped, round + 1, width, total_roots, total_mods, actual_classified, elapsed.as_secs_f64() * 1_000.0);
                }
            }
            for (width, mut values) in timings {
                values.sort();
                eprintln!("normal_sync_summary scenario={} scoped={} width={} samples={} min_ms={:.3} median_ms={:.3} max_ms={:.3}", scenario, scoped, width, values.len(), values[0].as_secs_f64() * 1_000.0, values[values.len()/2].as_secs_f64() * 1_000.0, values[values.len()-1].as_secs_f64() * 1_000.0);
            }
        }
        for round in 0..5 {
            let started = Instant::now();
            let discovery =
                collect_scoped_disk_discovery_with_progress(fixture.path(), &[], false, None)
                    .unwrap();
            let elapsed = started.elapsed();
            assert_eq!(classified, discovery.scan_counts.classified_directories);
            assert_eq!(
                expected_shape,
                normal_projection_shape(&discovery.projection)
            );
            eprintln!("normal_sync_discovery scenario={} round={} roots={} mods={} census_directories={} elapsed_ms={:.3} census_ms={} classification_ms={} parity=exact", scenario, round + 1, total_roots, total_mods, discovery.census.entries.len(), elapsed.as_secs_f64() * 1_000.0, discovery.timings.census_ms, discovery.timings.classification_ms);
        }
    }
}

fn benchmark_normal_scan_reuse(root: &Path, scenario: &str) {
    let (expected, classified) = normal_projection_serial_reference(root, &[], false).unwrap();
    let expected_shape = normal_projection_shape(&expected);
    eprintln!(
        "normal_scan_reuse_context scenario={} roots={} mods={} classified={} pool_threads={} child_width=1 cache=warm_uncontrolled progress_callback=false fields_checked=12",
        scenario,
        expected.objects.len(),
        expected.mods.len(),
        classified,
        rayon::current_num_threads()
    );
    // Both paths keep the same root parallelism and serial child scheduling.
    let mut samples = BTreeMap::<bool, Vec<Duration>>::new();
    for round in 0..7 {
        let order = if round % 2 == 0 {
            [true, false]
        } else {
            [false, true]
        };
        for original in order {
            let started = Instant::now();
            let (actual, actual_count) = if original {
                normal_projection_serial_reference(root, &[], false)
            } else {
                collect_disk_projection_with_parallelism(root, &[], false, None, NonZeroUsize::MIN)
            }
            .unwrap();
            let elapsed = started.elapsed();
            assert_eq!(classified, actual_count);
            assert_eq!(expected_shape, normal_projection_shape(&actual));
            samples.entry(original).or_default().push(elapsed);
            eprintln!(
                "normal_scan_reuse_trial scenario={} round={} mode={} elapsed_ms={:.3} parity=exact",
                scenario,
                round + 1,
                if original { "original" } else { "production" },
                elapsed.as_secs_f64() * 1_000.0
            );
        }
    }
    for (original, mut timings) in samples {
        timings.sort();
        eprintln!(
            "normal_scan_reuse_summary scenario={} mode={} samples={} min_ms={:.3} median_ms={:.3} max_ms={:.3}",
            scenario,
            if original { "original" } else { "production" },
            timings.len(),
            timings[0].as_secs_f64() * 1_000.0,
            timings[timings.len() / 2].as_secs_f64() * 1_000.0,
            timings[timings.len() - 1].as_secs_f64() * 1_000.0
        );
    }
}

#[test]
#[ignore = "manual owned container-rich normal classification benchmark"]
fn benchmark_normal_scan_reuse_synthetic() {
    let fixture = mixed_onboarding_fixture_with_roots(1_024, 16);
    for index in 0..1_024 {
        let object = fixture.path().join(format!("Object{:02}", index % 16));
        let container = object.join(format!("Container{:03}/深層", index / 16));
        std::fs::create_dir_all(container.join("Empty/MoreEmpty")).unwrap();
        let mod_name = format!("Mod{index:05}");
        std::fs::rename(object.join(&mod_name), container.join(mod_name)).unwrap();
    }
    benchmark_normal_scan_reuse(fixture.path(), "container_rich_owned");
}

#[test]
#[ignore = "manual read-only normal classification on an explicitly configured library"]
fn benchmark_normal_scan_reuse_readonly() {
    let root = std::env::var_os("EMMM_AUDIT_MODS_ROOT")
        .map(PathBuf::from)
        .expect("Set EMMM_AUDIT_MODS_ROOT to the authorized read-only Mods directory");
    assert!(root.is_absolute() && root.is_dir());
    benchmark_normal_scan_reuse(&root, "readonly_library");
}

// Freeze the original width-four batches independently of the production
// scheduler, including its progress, root validation and cached-key sort.
fn census_original_batches_with_progress(
    mods_path: &Path,
    progress: Option<&(dyn Fn(OnboardingDiscoveryProgress) + Send + Sync)>,
) -> DiskProjectionResult<DiskIdentityCensus> {
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err(DiskProjectionError::SourceUnavailable(format!(
            "Disk Reconcile mods path is unavailable: {}",
            mods_path.display()
        )));
    }
    let roots = list_runtime_dirs(mods_path)?;
    let total_roots = roots.len();
    if let Some(progress) = progress {
        progress(OnboardingDiscoveryProgress {
            phase: OnboardingDiscoveryPhase::Metadata,
            completed_roots: 0,
            total_roots,
            folders_classified: 0,
            current_root: None,
            is_terminal: false,
        });
    }
    let mut entries = Vec::new();
    let mut completed_roots = 0;
    for batch in roots.chunks(CENSUS_ROOT_PARALLELISM.get()) {
        let results = batch
            .par_iter()
            .map(|root| collect_census_root(mods_path, root))
            .collect::<Vec<_>>();
        for result in results {
            entries.extend(result?);
            let root_name = runtime_dir_name(&roots[completed_roots]);
            completed_roots += 1;
            if let Some(progress) = progress {
                progress(OnboardingDiscoveryProgress {
                    phase: OnboardingDiscoveryPhase::Metadata,
                    completed_roots,
                    total_roots,
                    folders_classified: 0,
                    current_root: Some(root_name),
                    is_terminal: completed_roots == total_roots,
                });
            }
        }
    }
    if total_roots == 0 {
        if let Some(progress) = progress {
            progress(OnboardingDiscoveryProgress {
                phase: OnboardingDiscoveryPhase::Metadata,
                completed_roots: 0,
                total_roots,
                folders_classified: 0,
                current_root: None,
                is_terminal: true,
            });
        }
    }
    entries.sort_by_cached_key(|entry| entry.folder_path.to_ascii_lowercase());
    Ok(DiskIdentityCensus {
        entries,
        top_level_roots: total_roots,
    })
}

fn census_bounded_pool_upper_bound(root: &Path) -> DiskProjectionResult<DiskIdentityCensus> {
    let roots = list_runtime_dirs(root)?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(CENSUS_ROOT_PARALLELISM.get())
        .build()
        .expect("bounded benchmark pool");
    let results = pool.install(|| {
        roots
            .par_iter()
            .map(|path| collect_census_root(root, path))
            .collect::<Vec<_>>()
    });
    let mut entries = Vec::new();
    for result in results {
        entries.extend(result?);
    }
    sort_census_entries(&mut entries);
    Ok(DiskIdentityCensus {
        entries,
        top_level_roots: roots.len(),
    })
}

const CENSUS_SCHEDULING_TEST_WAIT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct CensusReaderGate {
    released: Mutex<bool>,
    changed: std::sync::Condvar,
}

impl CensusReaderGate {
    fn wait(&self) {
        let released = self.released.lock().unwrap();
        let (released, _) = self
            .changed
            .wait_timeout_while(released, CENSUS_SCHEDULING_TEST_WAIT, |released| !*released)
            .unwrap();
        assert!(*released, "controlled census reader was not released");
    }

    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

fn scheduling_roots(count: usize) -> Vec<PathBuf> {
    (0..count)
        .map(|index| PathBuf::from(format!("Root{index}")))
        .collect()
}

fn scheduling_entry(path: &Path) -> DiskIdentityCensusEntry {
    let folder_path = path.to_string_lossy().into_owned();
    DiskIdentityCensusEntry {
        folder_path_key: crate::shared::path_key::folder_path_key(&folder_path, None),
        folder_path,
        raw_name: runtime_dir_name(path),
        absolute_path: path.to_path_buf(),
    }
}

#[test]
fn bounded_census_scheduler_starts_later_roots_while_first_root_is_blocked() {
    let roots = scheduling_roots(8);
    let gate = CensusReaderGate::default();
    let (started, received) = std::sync::mpsc::channel();
    let events = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            collect_census_roots(
                &roots,
                CENSUS_ROOT_PARALLELISM,
                &|path| {
                    started.send(runtime_dir_name(path)).unwrap();
                    if path == roots[0].as_path() {
                        gate.wait();
                    }
                    Ok(vec![scheduling_entry(path)])
                },
                &|completed| events.lock().unwrap().push(completed),
            )
        });
        let mut first_started = false;
        let mut later_started = false;
        let deadline = Instant::now() + Duration::from_secs(2);
        while !(first_started && later_started) {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            let Ok(name) = received.recv_timeout(remaining) else {
                break;
            };
            first_started |= name == "Root0";
            later_started |= name == "Root4";
        }
        let before_release = events.lock().unwrap().clone();
        gate.release();
        let entries = worker.join().unwrap().unwrap();
        assert!(first_started, "first controlled reader did not start");
        assert!(later_started, "Root4 waited for the Root0 batch barrier");
        assert!(
            before_release.is_empty(),
            "progress passed the blocked root"
        );
        assert_eq!(
            *events.lock().unwrap(),
            (1..=roots.len()).collect::<Vec<_>>()
        );
        assert_parity(
            &DiskIdentityCensus {
                entries: roots.iter().map(|path| scheduling_entry(path)).collect(),
                top_level_roots: roots.len(),
            },
            &DiskIdentityCensus {
                entries,
                top_level_roots: roots.len(),
            },
        );
    });
}

#[test]
fn bounded_census_scheduler_limits_active_readers() {
    let roots = scheduling_roots(8);
    let gate = CensusReaderGate::default();
    let active = AtomicUsize::new(0);
    let maximum = AtomicUsize::new(0);
    let (started, received) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            collect_census_roots(
                &roots,
                CENSUS_ROOT_PARALLELISM,
                &|path| {
                    let readers = active.fetch_add(1, Ordering::SeqCst) + 1;
                    maximum.fetch_max(readers, Ordering::SeqCst);
                    started.send(()).unwrap();
                    gate.wait();
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(vec![scheduling_entry(path)])
                },
                &|_| {},
            )
        });
        let mut started_count = 0;
        for _ in 0..CENSUS_ROOT_PARALLELISM.get() {
            if received.recv_timeout(CENSUS_SCHEDULING_TEST_WAIT).is_err() {
                break;
            }
            started_count += 1;
        }
        let extra_reader = received.recv_timeout(Duration::from_millis(100)).is_ok();
        gate.release();
        let entries = worker.join().unwrap().unwrap();
        assert_eq!(started_count, CENSUS_ROOT_PARALLELISM.get());
        assert!(!extra_reader, "more than four blocked readers started");
        assert_eq!(
            maximum.load(Ordering::SeqCst),
            CENSUS_ROOT_PARALLELISM.get()
        );
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(entries.len(), roots.len());
    });
}

#[test]
fn bounded_census_merge_reports_contiguous_progress_online() {
    let roots = scheduling_roots(3);
    let events = Mutex::new(Vec::new());
    let caller = std::thread::current().id();
    let results = [0, 2, 1].into_iter().map(|index| {
        if index != 0 {
            assert_eq!(*events.lock().unwrap(), [1]);
        }
        (index, Ok(vec![scheduling_entry(&roots[index])]))
    });
    let entries = merge_census_root_results(results, roots.len(), &|completed| {
        assert_eq!(std::thread::current().id(), caller);
        events.lock().unwrap().push(completed);
    })
    .unwrap();
    assert_eq!(*events.lock().unwrap(), [1, 2, 3]);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.raw_name.as_str())
            .collect::<Vec<_>>(),
        ["Root0", "Root1", "Root2"]
    );
}

#[test]
fn bounded_census_merge_selects_earliest_indexed_error_after_later_error_arrives() {
    let events = Mutex::new(Vec::new());
    let result = merge_census_root_results(
        [
            (2, Err(DiskProjectionError::Failed("later failure".into()))),
            (0, Ok(vec![scheduling_entry(Path::new("Root0"))])),
            (
                1,
                Err(DiskProjectionError::SourceUnavailable(
                    "earlier failure".into(),
                )),
            ),
        ],
        3,
        &|completed| events.lock().unwrap().push(completed),
    );
    assert!(
        matches!(result, Err(DiskProjectionError::SourceUnavailable(message)) if message == "earlier failure")
    );
    assert_eq!(*events.lock().unwrap(), [1]);
}

#[test]
fn bounded_census_merge_rejects_incomplete_completion_transport() {
    let result = merge_census_root_results([(1, Ok(Vec::new()))], 2, &|_| {
        panic!("progress cannot pass an absent first completion")
    });
    assert!(
        matches!(result, Err(DiskProjectionError::Failed(message)) if message.contains("0 of 2"))
    );
}

#[test]
fn bounded_census_scheduler_preserves_error_when_completion_receiver_closes() {
    let roots = scheduling_roots(8);
    let result = collect_census_roots(
        &roots,
        CENSUS_ROOT_PARALLELISM,
        &|path| {
            if path == roots[0].as_path() {
                Err(DiskProjectionError::SourceUnavailable(
                    "first root removed".into(),
                ))
            } else {
                Ok(vec![scheduling_entry(path)])
            }
        },
        &|_| panic!("a failed first root has no successful prefix"),
    );
    assert!(
        matches!(result, Err(DiskProjectionError::SourceUnavailable(message)) if message == "first root removed")
    );
}

#[test]
fn bounded_census_scheduler_handles_empty_single_and_serial_width() {
    for count in [0, 1, 2, 5] {
        let roots = scheduling_roots(count);
        for width in [1, 2, 4] {
            let events = Mutex::new(Vec::new());
            let entries = collect_census_roots(
                &roots,
                NonZeroUsize::new(width).unwrap(),
                &|path| Ok(vec![scheduling_entry(path)]),
                &|completed| events.lock().unwrap().push(completed),
            )
            .unwrap();
            assert_eq!(*events.lock().unwrap(), (1..=count).collect::<Vec<_>>());
            assert_parity(
                &DiskIdentityCensus {
                    entries: roots.iter().map(|path| scheduling_entry(path)).collect(),
                    top_level_roots: count,
                },
                &DiskIdentityCensus {
                    entries,
                    top_level_roots: count,
                },
            );
        }
    }
}

#[test]
#[ignore = "manual read-only census production scheduling versus frozen batches with progress"]
fn benchmark_readonly_census_production_scheduling() {
    let root = std::env::var_os("EMMM_AUDIT_MODS_ROOT")
        .map(PathBuf::from)
        .expect("Set EMMM_AUDIT_MODS_ROOT to the authorized read-only Mods directory");
    assert!(root.is_absolute() && root.is_dir());
    let expected_progress = Mutex::new(Vec::new());
    let expected = census_original_batches_with_progress(
        &root,
        Some(&|event| expected_progress.lock().unwrap().push(event)),
    )
    .unwrap();
    let expected_progress = expected_progress.into_inner().unwrap();
    eprintln!(
        "census_production_context roots={} directories={} width={} fields_checked=4 progress_events={} progress_fields_checked=6 cache=warm_uncontrolled pool_creation_shutdown_sort=included per_pass_limit=true",
        expected.top_level_roots,
        expected.entries.len(),
        CENSUS_ROOT_PARALLELISM,
        expected_progress.len()
    );
    let mut samples = BTreeMap::<bool, Vec<Duration>>::new();
    for round in 0..7 {
        let order = if round % 2 == 0 {
            [true, false]
        } else {
            [false, true]
        };
        for original in order {
            let progress = Mutex::new(Vec::new());
            let on_progress = |event| progress.lock().unwrap().push(event);
            let started = Instant::now();
            let actual = if original {
                census_original_batches_with_progress(&root, Some(&on_progress))
            } else {
                collect_disk_identity_census_with_progress(&root, Some(&on_progress))
            }
            .unwrap();
            let elapsed = started.elapsed();
            assert_parity(&expected, &actual);
            assert_eq!(expected_progress, progress.into_inner().unwrap());
            samples.entry(original).or_default().push(elapsed);
            eprintln!(
                "census_production_trial round={} mode={} elapsed_ms={:.3} entry_order_progress_parity=exact",
                round + 1,
                if original { "original_batches" } else { "production_continuous" },
                elapsed.as_secs_f64() * 1_000.0
            );
        }
    }
    for (original, mut timings) in samples {
        timings.sort();
        eprintln!(
            "census_production_summary mode={} samples={} min_ms={:.3} median_ms={:.3} max_ms={:.3}",
            if original { "original_batches" } else { "production_continuous" },
            timings.len(),
            timings[0].as_secs_f64() * 1_000.0,
            timings[timings.len() / 2].as_secs_f64() * 1_000.0,
            timings[timings.len() - 1].as_secs_f64() * 1_000.0
        );
    }
}

#[test]
#[ignore = "manual read-only census root skew and bounded-pool timing ceiling"]
fn benchmark_readonly_census_root_skew() {
    let root = std::env::var_os("EMMM_AUDIT_MODS_ROOT")
        .map(PathBuf::from)
        .expect("Set EMMM_AUDIT_MODS_ROOT to the authorized read-only Mods directory");
    assert!(root.is_absolute() && root.is_dir());
    let expected = collect_disk_identity_census(&root).unwrap();
    let roots = list_runtime_dirs(&root).unwrap();
    for round in 0..3 {
        let started = Instant::now();
        let mut entries = Vec::new();
        let mut root_timings = Vec::new();
        let mut batch_work = Duration::ZERO;
        for (batch_index, batch) in roots.chunks(CENSUS_ROOT_PARALLELISM.get()).enumerate() {
            let batch_started = Instant::now();
            let results = batch
                .par_iter()
                .map(|path| {
                    let started = Instant::now();
                    let result = collect_census_root(&root, path);
                    (result, started.elapsed())
                })
                .collect::<Vec<_>>();
            batch_work += batch_started.elapsed();
            for (offset, (result, elapsed)) in results.into_iter().enumerate() {
                let branch = result.unwrap();
                eprintln!(
                    "census_root_skew round={} batch={} root={} directories={} elapsed_ms={:.3}",
                    round + 1,
                    batch_index,
                    runtime_dir_name(&batch[offset]),
                    branch.len(),
                    elapsed.as_secs_f64() * 1_000.0
                );
                entries.extend(branch);
                root_timings.push(elapsed);
            }
        }
        sort_census_entries(&mut entries);
        assert_parity(
            &expected,
            &DiskIdentityCensus {
                entries,
                top_level_roots: roots.len(),
            },
        );
        let total_work = root_timings.iter().copied().sum::<Duration>();
        let maximum_root = root_timings.iter().copied().max().unwrap_or_default();
        let ideal_bound = (total_work / CENSUS_ROOT_PARALLELISM.get() as u32).max(maximum_root);
        eprintln!(
            "census_root_skew_summary round={} roots={} width={} wall_ms={:.3} batch_work_ms={:.3} summed_root_work_ms={:.3} maximum_root_ms={:.3} ideal_lower_bound_ms={:.3} possible_barrier_fraction={:.3} parity=exact",
            round + 1,
            roots.len(),
            CENSUS_ROOT_PARALLELISM,
            started.elapsed().as_secs_f64() * 1_000.0,
            batch_work.as_secs_f64() * 1_000.0,
            total_work.as_secs_f64() * 1_000.0,
            maximum_root.as_secs_f64() * 1_000.0,
            ideal_bound.as_secs_f64() * 1_000.0,
            if batch_work.is_zero() {
                0.0
            } else {
                1.0 - ideal_bound.as_secs_f64() / batch_work.as_secs_f64()
            }
        );
    }
    // This is a timing ceiling for a potential scheduling improvement: it has
    // no online progress callback and is not a production-compatible candidate.
    let mut samples = BTreeMap::<bool, Vec<Duration>>::new();
    for round in 0..7 {
        let order = if round % 2 == 0 {
            [true, false]
        } else {
            [false, true]
        };
        for upper_bound in order {
            let started = Instant::now();
            let actual = if upper_bound {
                census_bounded_pool_upper_bound(&root)
            } else {
                census_original_batches_with_progress(&root, None)
            }
            .unwrap();
            let elapsed = started.elapsed();
            assert_parity(&expected, &actual);
            samples.entry(upper_bound).or_default().push(elapsed);
            eprintln!(
                "census_scheduling_trial round={} mode={} elapsed_ms={:.3} parity=exact pool_creation=included progress_callback=false",
                round + 1,
                if upper_bound { "bounded_pool_upper_bound" } else { "original_batches" },
                elapsed.as_secs_f64() * 1_000.0
            );
        }
    }
    for (upper_bound, mut timings) in samples {
        timings.sort();
        eprintln!(
            "census_scheduling_summary mode={} samples={} min_ms={:.3} median_ms={:.3} max_ms={:.3}",
            if upper_bound { "bounded_pool_upper_bound" } else { "original_batches" },
            timings.len(),
            timings[0].as_secs_f64() * 1_000.0,
            timings[timings.len() / 2].as_secs_f64() * 1_000.0,
            timings[timings.len() - 1].as_secs_f64() * 1_000.0
        );
    }
}
