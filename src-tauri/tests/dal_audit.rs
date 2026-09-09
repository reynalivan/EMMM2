use std::fs;
use std::path::{Path, PathBuf};

fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut sources = Vec::new();

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        {
            let path = entry.expect("failed to read source entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "tests") {
                    continue;
                }
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && path.file_name().is_none_or(|name| name != "tests.rs")
            {
                sources.push(path);
            }
        }
    }

    sources
}

fn modules_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/modules")
}

#[test]
fn domain_code_is_framework_and_storage_agnostic() {
    let forbidden = ["sqlx::", "tauri::", "notify::"];
    let mut violations = Vec::new();

    for path in rust_sources(&modules_root()) {
        if !path
            .components()
            .any(|component| component.as_os_str() == "domain")
        {
            continue;
        }
        let source = fs::read_to_string(&path).expect("failed to read domain source");
        for dependency in forbidden {
            if source.contains(dependency) {
                violations.push(format!("{} imports {dependency}", path.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "domain modules must not depend on frameworks or storage:\n{}",
        violations.join("\n")
    );
}

#[test]
fn inbound_adapters_do_not_own_sql() {
    let raw_query_apis = [
        "sqlx::query(",
        "sqlx::query_as(",
        "sqlx::query_scalar(",
        "sqlx::query!(",
        "sqlx::query_as!(",
        "sqlx::query_scalar!(",
    ];
    let mut violations = Vec::new();

    for path in rust_sources(&modules_root()) {
        let normalized = path.to_string_lossy().replace('\\', "/");
        if !(normalized.contains("/adapters/tauri/")
            || normalized.ends_with("/adapters/tauri.rs")
            || normalized.contains("/adapters/inbound/"))
        {
            continue;
        }
        let source = fs::read_to_string(&path).expect("failed to read inbound adapter");
        if raw_query_apis.iter().any(|api| source.contains(api)) {
            violations.push(path.display().to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "inbound adapters must delegate persistence through module APIs:\n{}",
        violations.join("\n")
    );
}

#[test]
fn cross_module_imports_do_not_reach_inbound_internals() {
    let mut violations = Vec::new();

    for path in rust_sources(&modules_root()) {
        let normalized = path.to_string_lossy().replace('\\', "/");
        let Some(after_modules) = normalized.split("/modules/").nth(1) else {
            continue;
        };
        let Some(owner) = after_modules.split('/').next() else {
            continue;
        };
        let source = fs::read_to_string(&path).expect("failed to read module source");

        for line in source
            .lines()
            .filter(|line| line.contains("crate::modules::"))
        {
            let Some(imported) = line.split("crate::modules::").nth(1) else {
                continue;
            };
            let imported_owner = imported
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .next()
                .unwrap_or_default();
            if imported_owner != owner
                && (imported.contains("::adapters::tauri")
                    || imported.contains("::adapters::inbound"))
            {
                violations.push(format!("{}: {}", path.display(), line.trim()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "cross-module calls must use a facade, never inbound internals:\n{}",
        violations.join("\n")
    );
}

#[test]
fn legacy_horizontal_backend_layers_are_absent() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let legacy = ["commands", "services", "repo", "common", "domain", "types"];
    let existing = legacy
        .into_iter()
        .map(|name| source_root.join(name))
        .filter(|path| path.exists())
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    assert!(
        existing.is_empty(),
        "legacy horizontal backend layers must stay removed:\n{}",
        existing.join("\n")
    );
}
