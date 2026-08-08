//! Architecture gates (docs/architecture-refactor-plan.md).
//!
//! Unlike `dal_audit.rs`, these are not shrinking baselines: the app is
//! pre-release, so each gate goes to zero in the step that introduces it and
//! stays there. A failure means a layer contract was broken, not that a
//! number needs updating.

use std::fs;
use std::path::{Path, PathBuf};

fn is_test_source(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if file_name == "tests.rs" || file_name.ends_with("_tests.rs") {
        return true;
    }
    path.components()
        .any(|component| component.as_os_str() == "tests")
}

fn collect_rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", dir.display());
    });
    for entry in entries {
        let path = entry.expect("readable dir entry").path();
        if path.is_dir() {
            collect_rust_sources(&path, out);
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "rs") && !is_test_source(&path) {
            out.push(path);
        }
    }
}

fn violations(dir: &str, needles: &[&str]) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut files = Vec::new();
    collect_rust_sources(&root, &mut files);

    let mut found = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for (idx, line) in source.lines().enumerate() {
            if needles.iter().any(|needle| line.contains(needle)) {
                found.push(format!("{}:{}: {}", path.display(), idx + 1, line.trim()));
            }
        }
    }
    found
}

/// The operation lock is acquired at entry points only (commands, hotkey
/// handlers, queue workers); services prove they hold it by taking `&OpGuard`.
/// A service that acquires internally reintroduces the two-altitude split —
/// and the re-acquire deadlock — that `OpGuard` exists to prevent.
#[test]
fn services_never_acquire_the_operation_lock() {
    let allowed = [
        // Entry points with no command above them.
        "cycle_preset.rs",   // hotkey handler
        "placement.rs",      // browser import queue worker
        "operation_lock.rs", // the lock itself
    ];
    let found: Vec<String> = violations("src/services", &["op_lock.acquire()", ".acquire().await"])
        .into_iter()
        .filter(|line| {
            // Semaphores and sqlx pools have their own acquire; only the
            // operation lock is gated.
            line.contains("op_lock")
                && !allowed.iter().any(|entry_point| line.contains(entry_point))
        })
        .collect();
    assert!(
        found.is_empty(),
        "services must take &OpGuard instead of acquiring the operation lock:\n{}",
        found.join("\n")
    );
}

/// A service that receives a path takes `&ValidatedPath`, so the containment
/// check cannot be skipped by a caller. The guard is still called where a path
/// is *derived* rather than received — those sites are listed here, and a new
/// one has to be argued for rather than added silently.
#[test]
fn services_only_validate_paths_they_derive() {
    let allowed = [
        "guard.rs", // the guard itself
        // Resolves the client's switch target against the DB, then proves it.
        "workspace_switch_service.rs",
        // Import/download flows build their own target directory.
        "placement.rs",
        "jobs.rs",
    ];
    let found: Vec<String> = violations("src/services", &["validate_path(", "validate_paths("])
        .into_iter()
        .filter(|line| !allowed.iter().any(|site| line.contains(site)))
        .collect();
    assert!(
        found.is_empty(),
        "services must accept &ValidatedPath instead of validating a path they \
         were handed:\n{}",
        found.join("\n")
    );
}

/// The data-access layer does not define the IPC contract.
///
/// A `specta::Type` in `repo/` makes the SQL result-set shape the generated
/// TypeScript type, so the frontend imports its vocabulary from whichever
/// module happens to run the query. Those types live in `domain/` now.
///
/// Note this does not by itself decouple column names from the wire: the
/// domain types still derive `sqlx::FromRow` where the row and wire shapes
/// coincide. Splitting a table into a row struct plus a DTO is worth doing
/// per table, when one actually diverges — not pre-emptively for all of them.
#[test]
fn repos_do_not_define_ipc_types() {
    let found = violations("src/repo", &["specta::Type", "specta(type"]);
    assert!(
        found.is_empty(),
        "types the frontend consumes belong in domain/, not in the repo that \
         queries them:\n{}",
        found.join("\n")
    );
}

/// The data-access layer does not read the disk.
///
/// `object_repo::counts` used to resolve terminal nodes by walking
/// directories and parsing INI headers — once per row per ancestor, with no
/// memo, from inside a repo. Those rules now live in
/// `services::objects::terminal`, where the walk can be cached and kept off
/// the async runtime.
#[test]
fn repos_never_touch_the_filesystem() {
    let found = violations(
        "src/repo",
        &["std::fs", "read_dir(", "classify_folder", "File::open"],
    );
    assert!(
        found.is_empty(),
        "repos issue SQL; disk access belongs in a service:\n{}",
        found.join("\n")
    );
}

/// The data-access layer does not decide what an object *is*.
///
/// `ensure_object_exists` used to resolve identity from inside the repo: match
/// by name key, else by folder key, refuse a folder another row holds, and
/// decide which fields a re-match may overwrite. Every scan and every disk
/// reconcile runs those rules, and getting one wrong silently merges two
/// objects or splits one in two — which is why they belong somewhere they can
/// be read on their own, `services::objects::reconcile`.
///
/// The repo still runs the lookups and the UPDATEs. What it must not hold is
/// the choice between them.
#[test]
fn repos_do_not_decide_object_identity() {
    let found = violations(
        "src/repo",
        &["type_is_authoritative", "fn ensure_object_exists"],
    );
    assert!(
        found.is_empty(),
        "identity resolution and merge policy belong in \
         services::objects::reconcile, not in the repo that stores the row:\n{}",
        found.join("\n")
    );
}

/// A stored mod path cannot be mistaken for a filesystem path.
///
/// `mods.folder_path` is relative to the game's mods root. Six readers treated
/// it as a complete path, and every one failed identically and silently: the
/// path resolves against the process working directory, the check says "not
/// there", and the code takes its nothing-found branch. Conflict detection
/// reported no conflicts, the duplicate scanner reported no folders, and the
/// stale-mod resolver concluded the folder was gone and deleted the row.
///
/// `ModFolderPath` makes that a compile error by having no conversion to a
/// path — `resolve(mods_root)` is the only way through, and it cannot be
/// called without naming a root. This gate guards the absence: adding any of
/// these impls quietly restores the whole bug family.
#[test]
fn the_stored_mod_path_has_no_silent_path_conversion() {
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain/mod_path.rs"))
            .expect("domain/mod_path.rs");

    let banned = [
        "impl AsRef<Path> for ModFolderPath",
        "impl AsRef<std::path::Path> for ModFolderPath",
        "impl AsRef<OsStr> for ModFolderPath",
        "impl Deref for ModFolderPath",
        "impl std::ops::Deref for ModFolderPath",
        "impl From<ModFolderPath> for PathBuf",
        "impl From<ModFolderPath> for std::path::PathBuf",
    ];
    let found: Vec<&str> = banned
        .iter()
        .copied()
        .filter(|impl_line| source.contains(impl_line))
        .collect();

    assert!(
        found.is_empty(),
        "ModFolderPath must not convert to a path implicitly; resolve(mods_root) \
         is the way through:\n{}",
        found.join("\n")
    );
}

/// One owner settles a mutation.
///
/// The runtime projection is a read-model: a mutation that returns without
/// refreshing it leaves the grid, the counts and the in-game overlay
/// describing a library that no longer exists. `finalize_mutation` owns the
/// refresh and the side effects that read it, in that order — the pair used
/// to be spelled out at each mutation site, every copy discarding both
/// results with `let _ =`.
///
/// The listed files are the projection maintaining itself (a cold-projection
/// self-heal and two whole-library rebuilds), not mutations forgetting to.
#[test]
fn only_the_finalizer_refreshes_the_projection() {
    let allowed = [
        "runtime_effects.rs", // the finalizer
        // Reads, not mutations: these ask the projection to catch up with
        // rows it has not built yet, or rebuild it wholesale.
        "objects\\query.rs", // self-heal when the projection is cold
        "objects/query.rs",
        "post_apply.rs", // whole-library rebuild after an apply
        "reconcile.rs",  // whole-library rebuild after disk reconcile
    ];
    let mut found = violations(
        "src/services",
        &[
            "rebuild_game_projection",
            "refresh_projection_for_object_ids",
        ],
    );
    found.extend(violations(
        "src/commands",
        &[
            "rebuild_game_projection",
            "refresh_projection_for_object_ids",
        ],
    ));
    let found: Vec<String> = found
        .into_iter()
        .filter(|line| !allowed.iter().any(|owner| line.contains(owner)))
        .collect();
    assert!(
        found.is_empty(),
        "return a MutationOutcome and let finalize_mutation settle it:\n{}",
        found.join("\n")
    );
}

/// No service returns a stringly error.
///
/// `Result<_, String>` cannot carry a discriminant, so a service on that path
/// can never produce `FileInUse`, `PathBusy` or any other variant the frontend
/// matches on — every error it raises collapses into one opaque string.
#[test]
fn services_have_no_string_errors() {
    let found: Vec<String> = violations("src/services", &[", String>"])
        .into_iter()
        .filter(|line| line.contains("Result<"))
        .collect();
    assert!(
        found.is_empty(),
        "services return a typed error (a subsystem enum, or AppError); do not \
         reintroduce Result<_, String>:\n{}",
        found.join("\n")
    );
}

/// Commands surface the service's error, not a flattened string.
///
/// `map_err(AppError::Internal)` throws away whatever variant the service
/// produced, which is the discriminant the frontend switches on.
#[test]
fn commands_never_flatten_service_errors() {
    let found = violations("src/commands", &["map_err(AppError::Internal)"]);
    assert!(
        found.is_empty(),
        "let the service's typed error through with `?` instead of collapsing \
         it into Internal:\n{}",
        found.join("\n")
    );
}

/// The Safe Mode corridor is a privacy gate. It is derived server-side
/// (`ConfigService::current_corridor`) — a command that accepts it as an IPC
/// parameter reintroduces a spoofable gate.
#[test]
fn commands_never_take_corridor_flags() {
    let found = violations(
        "src/commands",
        &["safe_mode: bool", "is_safe: bool", "is_safe: Option<bool>"],
    );
    assert!(
        found.is_empty(),
        "corridor flags must not be command parameters; derive via \
         ConfigService::current_corridor() instead:\n{}",
        found.join("\n")
    );
}
