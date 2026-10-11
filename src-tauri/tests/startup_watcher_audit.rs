use emmm_lib::modules::workspace::api::testing::application::scanner::watcher::{
    watch_mod_directory, WatcherState,
};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer_opt, DebounceEventResult, NoCache, RecommendedCache};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
#[ignore = "explicit read-only audit of a configured Mods root"]
fn measure_watcher_registration_without_filesystem_writes() {
    env_logger::Builder::new()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .expect("audit logger");
    let root = PathBuf::from(std::env::var("EMMM_AUDIT_MODS_ROOT").expect("audit root required"));
    assert!(
        root.is_absolute() && root.is_dir(),
        "existing absolute root required"
    );

    // Diagnostic comparison only. Production keeps its file-ID rename proofs.
    let mut native = new_debouncer_opt::<_, RecommendedWatcher, NoCache>(
        Duration::from_millis(500),
        None,
        |_: DebounceEventResult| {},
        NoCache::new(),
        notify::Config::default(),
    )
    .expect("diagnostic native watcher");
    let started = Instant::now();
    native
        .watch(&root, RecursiveMode::Recursive)
        .expect("native watch registration");
    println!(
        "native_registration_without_id_cache_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    drop(native);

    if std::env::var("EMMM_AUDIT_FULL_CACHE").as_deref() == Ok("1") {
        let mut full = new_debouncer_opt::<_, RecommendedWatcher, RecommendedCache>(
            Duration::from_millis(500),
            None,
            |_: DebounceEventResult| {},
            RecommendedCache::new(),
            notify::Config::default(),
        )
        .expect("full SDK cache control");
        let started = Instant::now();
        full.watch(&root, RecursiveMode::Recursive)
            .expect("full cache registration");
        println!(
            "sdk_full_cache_registration_ms={:.3}",
            started.elapsed().as_secs_f64() * 1000.0
        );
        drop(full);
    }

    let state = WatcherState::new();
    let session = state.begin_session(&root);
    let started = Instant::now();
    let (watcher, receiver) = watch_mod_directory(&root, Arc::clone(&state.suppressor), session)
        .expect("production watcher registration");
    println!(
        "production_registration_with_id_cache_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    drop(watcher);
    drop(receiver);
}
