//! DAL drift guardrail (AGENT.md: "Mandatory DAL separation").
//!
//! Raw `sqlx::query*` calls belong in `src/repo/`. This ratchet test pins the
//! current violations in `src/services/` and forbids any raw SQL in
//! `src/commands/`. When you move a query into a repo function, lower (or
//! remove) the corresponding baseline entry. Never raise a number.

use std::fs;
use std::path::{Path, PathBuf};

const RAW_QUERY_MARKER: &str = "sqlx::query";

/// (file path relative to src-tauri, allowed number of lines containing `sqlx::query`)
const SERVICES_BASELINE: &[(&str, usize)] = &[
    // Startup schema patch runner, not data access; migrate when a real
    // migration layer exists.
    ("src/services/config/schema.rs", 3),
];

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

fn count_raw_query_lines(path: &Path) -> usize {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });
    source
        .lines()
        // In-file test modules sit below `#[cfg(test)]`; assertion SQL there is fine.
        .take_while(|line| line.trim() != "#[cfg(test)]")
        .filter(|line| line.contains(RAW_QUERY_MARKER))
        .count()
}

fn normalized_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("source file under manifest dir")
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn commands_layer_contains_no_raw_sql() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    collect_rust_sources(&root.join("src/commands"), &mut sources);

    let violations: Vec<String> = sources
        .iter()
        .filter(|path| count_raw_query_lines(path) > 0)
        .map(|path| normalized_relative_path(&root, path))
        .collect();

    assert!(
        violations.is_empty(),
        "raw `sqlx::query` in the command layer — commands must delegate to services/repo:\n{}",
        violations.join("\n"),
    );
}

#[test]
fn services_raw_sql_never_grows() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    collect_rust_sources(&root.join("src/services"), &mut sources);

    let mut errors = Vec::new();
    for path in &sources {
        let count = count_raw_query_lines(path);
        let relative = normalized_relative_path(&root, path);
        let baseline = SERVICES_BASELINE
            .iter()
            .find(|(file, _)| *file == relative)
            .map(|(_, allowed)| *allowed);

        match baseline {
            None if count == 0 => {}
            None => errors.push(format!(
                "{relative}: {count} raw `sqlx::query` line(s) in a file with no baseline — put the query in src/repo/ instead",
            )),
            Some(allowed) if count > allowed => errors.push(format!(
                "{relative}: raw `sqlx::query` lines grew from {allowed} to {count} — move new queries into src/repo/",
            )),
            Some(allowed) if count < allowed => errors.push(format!(
                "{relative}: down to {count} raw `sqlx::query` line(s) (baseline {allowed}) — lower its SERVICES_BASELINE entry in {}",
                file!(),
            )),
            Some(_) => {}
        }
    }

    for (file, _) in SERVICES_BASELINE {
        if !sources
            .iter()
            .any(|path| normalized_relative_path(&root, path) == *file)
        {
            errors.push(format!(
                "{file}: listed in SERVICES_BASELINE but no longer exists — remove its entry",
            ));
        }
    }

    assert!(
        errors.is_empty(),
        "DAL ratchet violations:\n{}",
        errors.join("\n")
    );
}
