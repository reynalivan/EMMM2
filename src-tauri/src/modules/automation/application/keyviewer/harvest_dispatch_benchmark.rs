use super::*;
use std::time::Instant;

const MODS: usize = 2_048;
const PAIRS: usize = 5;

#[derive(Clone, Copy, Debug)]
enum Dispatch {
    PerMod,
    SerialBatch(usize),
}

async fn harvest_with_dispatch(
    paths: &[PathBuf],
    capabilities: &HarvestCapabilities,
    dispatch: Dispatch,
) -> Result<Vec<Arc<ModHarvest>>, AppError> {
    let mut output = Vec::with_capacity(paths.len());
    match dispatch {
        Dispatch::PerMod => {
            for path in paths {
                let path = path.clone();
                let capabilities = capabilities.clone();
                output.push(
                    tokio::task::spawn_blocking(move || harvest_mod(&path, &capabilities))
                        .await??,
                );
            }
        }
        Dispatch::SerialBatch(size) => {
            for batch in paths.chunks(size) {
                let batch = batch.to_vec();
                let capabilities = capabilities.clone();
                let harvested = tokio::task::spawn_blocking(move || {
                    batch
                        .iter()
                        .map(|path| harvest_mod(path, &capabilities))
                        .collect::<Result<Vec<_>, AppError>>()
                })
                .await??;
                output.extend(harvested);
            }
        }
    }
    Ok(output)
}

fn assert_harvest_parity(expected: &[Arc<ModHarvest>], actual: &[Arc<ModHarvest>]) {
    assert_eq!(expected.len(), actual.len());
    for (left, right) in expected.iter().zip(actual) {
        assert_eq!(left.targets, right.targets);
        assert_eq!(left.keybinds, right.keybinds);
        assert_eq!(left.ini_fingerprints, right.ini_fingerprints);
    }
}

fn snapshot_cache(root: &Path) -> HashMap<HarvestCacheKey, CachedModHarvest> {
    let prefix = format!("{}/", cache_path_key(root));
    HARVEST_CACHE
        .lock()
        .expect("cache lock")
        .iter()
        .filter(|(key, _)| key.mod_path.starts_with(&prefix))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

async fn measure_dispatch(candidates: &[Dispatch]) {
    let fixture = tempfile::tempdir().expect("owned dispatch fixture");
    let root = fixture.path().join("Mods");
    let capability = HarvestCapabilities::from_callback_slots(["vb0"]);
    let content = format!(
        "[TextureOverrideBody]\nhash = df65bb00\nvb0 = ResourceBody\n[KeyBody]\nkey = CTRL+K\n{}",
        "; representative resource configuration padding\n".repeat(80)
    );
    let paths: Vec<_> = (0..MODS)
        .map(|index| {
            let path = root.join(format!("Group{:02}/Mod{index:04}", index % 16));
            fs::create_dir_all(&path).expect("fixture directory");
            fs::write(path.join("mod.ini"), &content).expect("fixture INI");
            path
        })
        .collect();
    let reference: Vec<_> = paths
        .iter()
        .map(|path| harvest_mod(path, &capability).expect("reference harvest"))
        .collect();
    for (scope, count) in [("one_mod", 1), ("scoped", 64), ("full", MODS)] {
        let changed_keys: HashSet<_> = paths[..count]
            .iter()
            .map(|path| cache_path_key(path))
            .collect();
        for cold_parse_cache in [false, true] {
            for pair in 0..PAIRS {
                let modes: Vec<_> = if pair % 2 == 0 {
                    candidates.to_vec()
                } else {
                    candidates.iter().rev().copied().collect()
                };
                for dispatch in modes {
                    let cache_before = snapshot_cache(&root);
                    if cold_parse_cache {
                        invalidate_cached_mods(paths[..count].iter().map(PathBuf::as_path));
                    } else {
                        for path in &paths[..count] {
                            harvest_mod(path, &capability).expect("warm parse cache");
                        }
                    }
                    let reads_before: usize = paths[..count]
                        .iter()
                        .map(|path| harvest_read_count(&path.join("mod.ini")))
                        .sum();
                    let started = Instant::now();
                    let actual = harvest_with_dispatch(&paths[..count], &capability, dispatch)
                        .await
                        .expect("dispatch harvest");
                    let elapsed_us = started.elapsed().as_secs_f64() * 1_000_000.0;
                    assert_harvest_parity(&reference[..count], &actual);
                    let reads_after: usize = paths[..count]
                        .iter()
                        .map(|path| harvest_read_count(&path.join("mod.ini")))
                        .sum();
                    assert_eq!(reads_after - reads_before, count);
                    assert_eq!(cached_entry_count_for_root(&root), MODS);
                    let cache_after = snapshot_cache(&root);
                    assert_eq!(cache_before.len(), cache_after.len());
                    for (key, after) in &cache_after {
                        let before = cache_before.get(key).expect("same cache keys");
                        assert_eq!(before.files, after.files);
                        if !cold_parse_cache || !changed_keys.contains(&key.mod_path) {
                            assert!(Arc::ptr_eq(&before.harvest, &after.harvest));
                        }
                    }
                    if !cold_parse_cache {
                        for (path, observed) in paths[..count].iter().zip(&actual) {
                            let key = HarvestCacheKey {
                                mod_path: cache_path_key(path),
                                capability_slots: capability.cache_key(),
                            };
                            assert!(Arc::ptr_eq(observed, &cache_before[&key].harvest,));
                        }
                    }
                    let dispatches = match dispatch {
                        Dispatch::PerMod => count,
                        Dispatch::SerialBatch(size) => count.div_ceil(size),
                    };
                    println!(
                        "{}",
                        serde_json::json!({
                            "benchmark": "harvest_dispatch", "scope": scope,
                            "mods": count, "pair": pair + 1,
                            "cold_parse_cache": cold_parse_cache,
                            "filesystem_cache": "warm_uncontrolled",
                            "dispatch": format!("{dispatch:?}"), "dispatches": dispatches,
                            "elapsed_us": elapsed_us, "ini_bytes_per_mod": content.len(),
                            "exact_output_order_and_cache_parity": true,
                            "ini_reads": reads_after - reads_before,
                            "maximum_inflight_blocking_tasks": 1,
                        })
                    );
                }
            }
        }
    }
    evict_cached_root_by_key(&cache_path_key(&root));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "manual harvest dispatch amortization diagnostic; single-task mode is not production"]
async fn benchmark_harvest_dispatch_overhead() {
    measure_dispatch(&[Dispatch::PerMod, Dispatch::SerialBatch(MODS)]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "manual bounded harvest dispatch experiment; run after overhead is measured"]
async fn benchmark_harvest_bounded_dispatch() {
    measure_dispatch(&[
        Dispatch::PerMod,
        Dispatch::SerialBatch(1),
        Dispatch::SerialBatch(2),
        Dispatch::SerialBatch(4),
    ])
    .await;
}
