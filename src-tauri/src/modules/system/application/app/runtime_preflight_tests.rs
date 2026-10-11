use super::*;
use crate::modules::games::domain::models::GameType;
use tempfile::TempDir;

#[test]
fn keyviewer_traversal_prunes_disabled_directories_before_descent() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Include");
    let active = root.join("Hidden/子/KeyViewer.ini");
    let disabled = root.join("dIsAbLeD-legacy/Deep/KeyViewer.ini");
    for path in [&active, &disabled] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "owned fixture").unwrap();
    }
    let visited = keyviewer_include_entries(&root)
        .map(|entry| entry.unwrap().into_path())
        .collect::<Vec<_>>();
    assert!(visited.contains(&active));
    assert!(visited
        .iter()
        .all(|path| !path.starts_with(root.join("dIsAbLeD-legacy"))));
    let disabled_root = root.join("dIsAbLeD-legacy/Deep");
    assert_eq!(keyviewer_include_entries(&disabled_root).count(), 0);
}

#[test]
fn keyviewer_traversal_preserves_all_roots_and_duplicate_ownership() {
    let (_fixture, mods, roots) = keyviewer_profile_fixture(32, false, true);
    let secondary = roots[1].join(".hidden/子/KeyViewer.ini");
    std::fs::create_dir_all(secondary.parent().unwrap()).unwrap();
    std::fs::write(
        &secondary,
        "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1",
    )
    .unwrap();
    let mut overlapping = roots.clone();
    overlapping.push(roots[1].join(".hidden"));
    let actual = duplicate_keyviewer_entrypoints(&overlapping, &mods);
    let expected = duplicate_entrypoints_baseline_for_profile(&overlapping, &mods);
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 3);
    assert!(actual.contains(&secondary));
}

#[cfg(windows)]
#[test]
fn keyviewer_traversal_preserves_root_and_nested_junction_policy() {
    struct OwnedJunction(PathBuf);
    impl Drop for OwnedJunction {
        fn drop(&mut self) {
            std::fs::remove_dir(&self.0).expect("remove owned junction without its target");
        }
    }
    let fixture = TempDir::new().unwrap();
    let root = fixture.path().join("Include");
    let target = fixture.path().join("Target");
    let alias = root.join("Alias");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(
        target.join("KeyViewer.ini"),
        "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1",
    )
    .unwrap();
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&alias)
        .arg(&target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _junction = OwnedJunction(alias.clone());
    for include in [root.clone(), alias] {
        assert_eq!(
            duplicate_keyviewer_entrypoints(std::slice::from_ref(&include), &root),
            duplicate_entrypoints_baseline_for_profile(std::slice::from_ref(&include), &root)
        );
    }
    assert!(duplicate_keyviewer_entrypoints(std::slice::from_ref(&root), &root).is_empty());
}

fn duplicate_entrypoints_baseline_for_profile(roots: &[PathBuf], mods: &Path) -> Vec<PathBuf> {
    let expected = normalized_windows_path(&mods.join(".emmm_data/KeyViewer.ini"));
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for root in roots {
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| entry.file_name().eq_ignore_ascii_case("KeyViewer.ini"))
            .filter(|entry| !is_disabled_include_path(entry.path()))
        {
            let normalized = normalized_windows_path(entry.path());
            if normalized == expected || !seen.insert(normalized) {
                continue;
            }
            let Ok(content) = std::fs::read(entry.path()) else {
                continue;
            };
            if is_verified_emmm_entrypoint(&content) {
                duplicates.push(entry.into_path());
            }
        }
    }
    duplicates.sort();
    duplicates
}

fn keyviewer_profile_fixture(
    mod_count: usize,
    disabled_majority: bool,
    owned_duplicates: bool,
) -> (TempDir, PathBuf, Vec<PathBuf>) {
    let fixture = TempDir::new().unwrap();
    let mods = fixture.path().join("Mods");
    let extra = fixture.path().join("Extra");
    std::fs::create_dir_all(&extra).unwrap();
    for index in 0..mod_count {
        let disabled = disabled_majority && index % 5 != 0;
        let prefix = if disabled { "DISABLED " } else { "" };
        let folder = mods
            .join(format!("{prefix}Object{:02}", index / 128))
            .join(format!("Mod{index:04}/Assets"));
        std::fs::create_dir_all(&folder).unwrap();
        for asset in 0..8 {
            std::fs::write(folder.join(format!("mesh{asset}.buf")), [0; 16]).unwrap();
        }
    }
    let marker =
        "keyviewer.ini\n; auto-generated by emmm\n[KeyEMMM_Toggle]\n[CommandList_EMM_Render]\n";
    let primary = mods.join(".emmm_data/KeyViewer.ini");
    std::fs::create_dir_all(primary.parent().unwrap()).unwrap();
    std::fs::write(primary, marker).unwrap();
    if owned_duplicates {
        std::fs::write(mods.join("Object00/Mod0000/KeyViewer.ini"), marker).unwrap();
        std::fs::write(extra.join("KeyViewer.ini"), marker).unwrap();
    }
    std::fs::create_dir_all(extra.join("Foreign")).unwrap();
    std::fs::write(extra.join("Foreign/KeyViewer.ini"), "unowned").unwrap();
    let disabled = mods.join("DISABLED Legacy");
    std::fs::create_dir_all(&disabled).unwrap();
    std::fs::write(disabled.join("KeyViewer.ini"), marker).unwrap();
    let roots = vec![mods.clone(), extra];
    (fixture, mods, roots)
}

#[derive(serde::Serialize)]
struct KeyViewerLockProfile {
    topology: &'static str,
    round: usize,
    variant: &'static str,
    mods: usize,
    entries: usize,
    scan_us: u128,
    commit_us: u128,
    revision_wait_us: u128,
    activation_wait_us: u128,
    revision_lock_confirmed: bool,
    committed: bool,
    duplicates: usize,
}

fn profile_keyviewer_commit(
    roots: &[PathBuf],
    mods: &Path,
    optimized: bool,
) -> (KeyViewerLockProfile, Vec<PathBuf>) {
    use crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState;
    use std::sync::Barrier;
    use std::time::Instant;
    let state = Arc::new(DiskReconcileState::new());
    let publisher = format!("kv-profile-publisher-{}", uuid::Uuid::new_v4());
    let other_game = format!("kv-profile-other-{}", uuid::Uuid::new_v4());
    state.begin_activation(Some(publisher.clone()));
    let authority = state.activation_authority_for(&publisher).unwrap();
    let revision = request_sync_revision(&publisher).unwrap();
    let start = Arc::new(Barrier::new(3));
    let mut scan_us = 0;
    let mut commit_us = 0;
    let mut duplicates = Vec::new();
    let mut lock_confirmed = false;
    let (committed, revision_wait, activation_wait) = std::thread::scope(|scope| {
        let barrier = Arc::clone(&start);
        let other = &other_game;
        let revision_waiter = scope.spawn(move || {
            barrier.wait();
            let started = Instant::now();
            let other_revision = request_sync_revision(other).unwrap();
            let elapsed = started.elapsed().as_micros();
            settle_sync_request(other, other_revision).unwrap();
            elapsed
        });
        let barrier = Arc::clone(&start);
        let state = Arc::clone(&state);
        let activation_waiter = scope.spawn(move || {
            barrier.wait();
            let started = Instant::now();
            state.begin_activation(Some("kv-profile-next-game".to_string()));
            started.elapsed().as_micros()
        });
        let commit_started = Instant::now();
        let committed =
            commit_if_current_sync_revision(&publisher, revision, Some(&authority), || {
                lock_confirmed = matches!(
                    KEYVIEWER_SYNC_REVISIONS.get().unwrap().try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                );
                start.wait();
                let started = Instant::now();
                duplicates = if optimized {
                    duplicate_keyviewer_entrypoints(roots, mods)
                } else {
                    duplicate_entrypoints_baseline_for_profile(roots, mods)
                };
                scan_us = started.elapsed().as_micros();
                Ok(())
            })
            .unwrap()
            .is_some();
        commit_us = commit_started.elapsed().as_micros();
        let revision_elapsed = revision_waiter.join().unwrap();
        let activation_elapsed = activation_waiter.join().unwrap();
        (committed, revision_elapsed, activation_elapsed)
    });
    {
        let mut revisions = KEYVIEWER_SYNC_REVISIONS.get().unwrap().lock().unwrap();
        revisions.remove(&publisher);
        revisions.remove(&other_game);
    }
    state.begin_activation(None);
    (
        KeyViewerLockProfile {
            topology: "",
            round: 0,
            variant: if optimized {
                "current_scan_inside_commit"
            } else {
                "baseline_scan_inside_commit"
            },
            mods: 0,
            entries: 0,
            scan_us,
            commit_us,
            revision_wait_us: revision_wait,
            activation_wait_us: activation_wait,
            revision_lock_confirmed: lock_confirmed,
            committed,
            duplicates: duplicates.len(),
        },
        duplicates,
    )
}

#[test]
#[ignore = "manual owned-fixture KeyViewer traversal/commit lock contention audit"]
fn benchmark_keyviewer_traversal_publication_lock() {
    let _activation_guard = crate::modules::reconciliation::application::disk_reconcile::orchestrator::activation_epoch_test_guard();
    for (topology, mod_count, disabled, owned_duplicates) in [
        ("small_active", 32, false, true),
        ("large_active_no_duplicates", 2048, false, false),
        ("large_mostly_disabled", 2048, true, true),
    ] {
        let (_fixture, mods, roots) =
            keyviewer_profile_fixture(mod_count, disabled, owned_duplicates);
        let expected = duplicate_keyviewer_entrypoints(&roots, &mods);
        assert_eq!(expected.len(), if owned_duplicates { 2 } else { 0 });
        let entries = roots
            .iter()
            .map(|root| WalkDir::new(root).follow_links(false).into_iter().count())
            .sum();
        for round in 0..5 {
            for optimized in if round % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let (mut row, duplicates) = profile_keyviewer_commit(&roots, &mods, optimized);
                assert_eq!(duplicates, expected);
                assert!(row.committed);
                assert!(row.revision_lock_confirmed);
                row.topology = topology;
                row.round = round;
                row.mods = mod_count;
                row.entries = entries;
                println!("KV_LOCK_PROFILE {}", serde_json::to_string(&row).unwrap());
            }
            for pruned in if round % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let started = std::time::Instant::now();
                let duplicates = if pruned {
                    duplicate_keyviewer_entrypoints(&roots, &mods)
                } else {
                    duplicate_entrypoints_baseline_for_profile(&roots, &mods)
                };
                let elapsed_us = started.elapsed().as_micros();
                assert_eq!(duplicates, expected);
                println!(
                    "KV_PRUNE_PROFILE {}",
                    serde_json::json!({"topology":topology,"round":round,"pruned":pruned,"elapsed_us":elapsed_us,"mods":mod_count,"entries":entries,"duplicates":duplicates.len()})
                );
            }
        }
    }
}

fn importer_fixture(temp: &TempDir) -> PathBuf {
    let mods = temp.path().join("Library").join("Mods");
    std::fs::create_dir_all(mods.join("Character").join("Assets")).unwrap();
    std::fs::create_dir_all(temp.path().join("Library").join("ModsExtra")).unwrap();
    std::fs::create_dir_all(temp.path().join("Core")).unwrap();
    std::fs::write(
        temp.path().join("d3dx.ini"),
        "[Include]\ninclude_recursive = Library/Mods\ninclude_recursive = Library/ModsExtra\n",
    )
    .unwrap();
    std::fs::write(
        mods.join("Character").join("ignored.ini"),
        "checktextureoverride = ps-t9\n",
    )
    .unwrap();
    std::fs::write(
        mods.join("Character").join("Assets").join("mesh.buf"),
        [0; 16],
    )
    .unwrap();
    std::fs::write(
        temp.path().join("Core").join("renderer.ini"),
        "namespace = GIMIv8\nResourceText = null\nResourceTextParams = null\n[CommandListPrintText]\nchecktextureoverride = vb0\n",
    )
    .unwrap();
    std::fs::write(
        temp.path()
            .join("Library")
            .join("ModsExtra")
            .join("callback.ini"),
        "checktextureoverride = ps-t1\n",
    )
    .unwrap();
    mods
}

#[test]
fn preflight_traversal_prunes_only_configured_mods_before_descent() {
    let temp = TempDir::new().unwrap();
    let mods = importer_fixture(&temp);
    let visited = importer_configuration_entries(temp.path(), &mods)
        .map(|entry| entry.unwrap().into_path())
        .collect::<Vec<_>>();
    assert!(
        visited.iter().all(|path| !path.starts_with(&mods)),
        "configured mod entries were visited: {visited:?}"
    );
    assert!(visited.contains(
        &temp
            .path()
            .join("Library")
            .join("ModsExtra")
            .join("callback.ini")
    ));
    assert!(visited.contains(&temp.path().join("Core").join("renderer.ini")));
}

#[test]
fn preflight_pruning_preserves_secondary_include_and_content_invalidation() {
    let temp = TempDir::new().unwrap();
    let mods = importer_fixture(&temp);
    assert_eq!(
        runtime_preflight_cache_key(temp.path(), &mods, GameType::Gimi),
        format!(
            "{}|{}|GIMI",
            crate::shared::path_key::canonical_path_key_for_path(temp.path()),
            crate::shared::path_key::canonical_path_key_for_path(&mods),
        )
    );
    let first_snapshot = runtime_preflight_snapshot(temp.path(), &mods).unwrap();
    let first = read_runtime_preflight(temp.path(), &mods, GameType::Gimi).unwrap();
    assert!(first.renderer_available);
    assert!(first.callback_slots.contains("ps-t1"));
    assert!(!first.callback_slots.contains("ps-t9"));
    std::fs::write(
        mods.join("Character").join("ignored.ini"),
        "checktextureoverride = ps-t8\n",
    )
    .unwrap();
    assert_eq!(
        first_snapshot,
        runtime_preflight_snapshot(temp.path(), &mods).unwrap()
    );
    let callback = temp
        .path()
        .join("Library")
        .join("ModsExtra")
        .join("callback.ini");
    std::fs::write(callback, "checktextureoverride = ps-t2\n").unwrap();
    assert_ne!(
        first_snapshot,
        runtime_preflight_snapshot(temp.path(), &mods).unwrap()
    );
    let changed = read_runtime_preflight(temp.path(), &mods, GameType::Gimi).unwrap();
    assert!(changed.callback_slots.contains("ps-t2"));
    assert!(!changed.callback_slots.contains("ps-t1"));
    assert!(!changed.callback_slots.contains("ps-t8"));
}

fn baseline_snapshot(importer_root: &Path, mods_root: &Path) -> Option<RuntimePreflightSnapshot> {
    let mut files = Vec::new();
    for entry in WalkDir::new(importer_root).follow_links(false).max_depth(5) {
        let entry = entry.ok()?;
        if !entry.file_type().is_file()
            || path_is_within(entry.path(), mods_root)
            || !entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"))
        {
            continue;
        }
        if files.len() >= 1_000 {
            return None;
        }
        let metadata = entry.metadata().ok()?;
        let content = std::fs::read(entry.path()).ok()?;
        files.push(RuntimePreflightFileSnapshot {
            path: crate::shared::path_key::canonical_path_key_for_path(entry.path()),
            identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(entry.path()),
            len: metadata.len(),
            modified: metadata.modified().ok(),
            content_digest: format!("{:x}", Sha256::digest(content)),
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Some(RuntimePreflightSnapshot {
        importer_identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(importer_root),
        mods_identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(mods_root),
        files,
    })
}

fn baseline_capabilities(importer_root: &Path, mods_root: &Path) -> (bool, HashSet<String>) {
    let mut renderer = false;
    let mut callbacks = HashSet::new();
    for entry in WalkDir::new(importer_root).follow_links(false).max_depth(5) {
        let entry = entry.unwrap();
        if !entry.file_type().is_file()
            || path_is_within(entry.path(), mods_root)
            || !entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"))
        {
            continue;
        }
        let content = std::fs::read(entry.path()).unwrap();
        renderer |= has_ini_assignment(&content, b"namespace", b"GIMIv8")
            && contains_ascii_case_insensitive(&content, b"resourcetext")
            && contains_ascii_case_insensitive(&content, b"resourcetextparams")
            && contains_ascii_case_insensitive(&content, b"commandlistprinttext");
        for (key, value) in ascii_ini_assignments(&content) {
            if key.eq_ignore_ascii_case(b"checktextureoverride") {
                if let Ok(slot) = std::str::from_utf8(value) {
                    if !slot.is_empty() {
                        callbacks.insert(slot.to_ascii_lowercase());
                    }
                }
            }
        }
    }
    (renderer, callbacks)
}

#[test]
fn preflight_pruning_resolves_dot_components_in_configured_root() {
    let temp = TempDir::new().unwrap();
    let mods = importer_fixture(&temp);
    let alternate_mods = mods.join("Character").join("..");
    let snapshot = runtime_preflight_snapshot(temp.path(), &alternate_mods).unwrap();
    assert_eq!(
        snapshot,
        baseline_snapshot(temp.path(), &alternate_mods).unwrap()
    );
    let visited = importer_configuration_entries(temp.path(), &alternate_mods)
        .map(|entry| entry.unwrap().into_path())
        .collect::<Vec<_>>();
    assert!(visited.iter().all(|path| !path.starts_with(&mods)));
}

#[cfg(windows)]
#[test]
fn preflight_pruning_resolves_a_configured_windows_junction_root() {
    struct OwnedJunction(PathBuf);
    impl Drop for OwnedJunction {
        fn drop(&mut self) {
            std::fs::remove_dir(&self.0).expect("remove owned fixture junction without its target");
        }
    }

    let temp = TempDir::new().unwrap();
    let mods = importer_fixture(&temp);
    let alias = temp.path().join("ModsAlias");
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&alias)
        .arg(&mods)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "could not create owned junction: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _junction = OwnedJunction(alias.clone());
    assert_eq!(canonical_or_original(&alias), canonical_or_original(&mods));
    let snapshot = runtime_preflight_snapshot(temp.path(), &alias).unwrap();
    assert_eq!(snapshot, baseline_snapshot(temp.path(), &alias).unwrap());
    let canonical_mods = canonical_or_original(&mods);
    let visited = importer_configuration_entries(temp.path(), &alias)
        .map(|entry| entry.unwrap().into_path())
        .collect::<Vec<_>>();
    assert!(visited
        .iter()
        .all(|path| !canonical_or_original(path).starts_with(&canonical_mods)));
}

#[test]
#[ignore = "owned synthetic importer traversal benchmark"]
fn benchmark_runtime_preflight_pruning() {
    for (topology, mod_count, assets_per_mod) in [("small", 32, 3), ("large", 2048, 8)] {
        let temp = TempDir::new().unwrap();
        let mods = importer_fixture(&temp);
        for index in 0..mod_count {
            let root = mods.join(format!("Mod-{index:05}"));
            let assets = root.join("Assets");
            std::fs::create_dir_all(&assets).unwrap();
            std::fs::write(root.join("mod.ini"), "checktextureoverride = ps-t9\n").unwrap();
            for asset in 0..assets_per_mod {
                std::fs::write(assets.join(format!("mesh-{asset}.buf")), [0; 64]).unwrap();
            }
        }
        let expected = baseline_snapshot(temp.path(), &mods).unwrap();
        let expected_capabilities = baseline_capabilities(temp.path(), &mods);
        let actual = read_runtime_preflight_uncached(temp.path(), &mods, GameType::Gimi).unwrap();
        assert_eq!(actual.renderer_available, expected_capabilities.0);
        assert_eq!(actual.callback_slots, expected_capabilities.1);
        let baseline_entries = WalkDir::new(temp.path())
            .follow_links(false)
            .max_depth(5)
            .into_iter()
            .count();
        let pruned_entries = importer_configuration_entries(temp.path(), &mods).count();
        for pair in 0..5 {
            for variant in if pair % 2 == 0 {
                ["baseline", "pruned"]
            } else {
                ["pruned", "baseline"]
            } {
                let started = std::time::Instant::now();
                let snapshot = if variant == "baseline" {
                    baseline_snapshot(temp.path(), &mods)
                } else {
                    runtime_preflight_snapshot(temp.path(), &mods)
                }
                .unwrap();
                let elapsed_us = started.elapsed().as_micros();
                assert_eq!(snapshot, expected);
                println!("PREFLIGHT_BENCH {{\"topology\":\"{topology}\",\"pair\":{pair},\"variant\":\"{variant}\",\"elapsed_us\":{elapsed_us},\"mods\":{mod_count},\"assets_per_mod\":{assets_per_mod},\"baseline_entries\":{baseline_entries},\"pruned_entries\":{pruned_entries},\"configuration_files\":{}}}", snapshot.files.len());
            }
        }
    }
}
