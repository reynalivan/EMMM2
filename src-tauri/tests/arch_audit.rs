use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn production_state_consumers_do_not_bypass_mutation_coordinator() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut rust_files = Vec::new();
    collect_rust_files(&source_root, &mut rust_files);

    let mut violations = Vec::new();
    for path in rust_files {
        if is_test_source(&path) {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        for (index, line) in source.lines().enumerate() {
            let compact: String = line
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect();
            let consumes_state = compact.contains("State<")
                || compact.contains("state::<")
                || compact.contains("try_state::<")
                || compact.contains("require::<");
            if consumes_state && compact.contains("OperationLock") {
                violations.push(format!(
                    "{}:{}: {}",
                    path.strip_prefix(&source_root).unwrap().display(),
                    index + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "production code must resolve MutationCoordinator instead of OperationLock:\n{}",
        violations.join("\n")
    );
}

#[test]
fn frontend_has_no_direct_filesystem_plugin_or_broad_capability() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("src-tauri must be inside the workspace root");
    let package_source = fs::read_to_string(workspace_root.join("package.json"))
        .expect("read frontend package manifest");
    let capability_source = fs::read_to_string(manifest_dir.join("capabilities/default.json"))
        .expect("read default Tauri capability");
    let backend_source =
        fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read Tauri bootstrap");

    let mut violations = Vec::new();
    if package_source.contains("@tauri-apps/plugin-fs") {
        violations.push("package.json depends on @tauri-apps/plugin-fs".to_string());
    }
    for forbidden in ["fs:default", "fs:read-all", "fs:write-all"] {
        if capability_source.contains(forbidden) {
            violations.push(format!("capabilities/default.json grants {forbidden}"));
        }
    }
    if backend_source.contains("tauri_plugin_fs::init") {
        violations.push("src-tauri/src/lib.rs initializes tauri-plugin-fs".to_string());
    }

    let frontend_root = workspace_root.join("src");
    let mut frontend_files = Vec::new();
    collect_frontend_sources(&frontend_root, &mut frontend_files);
    for path in frontend_files {
        if is_frontend_test_source(&path) {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read frontend source");
        if source.contains("@tauri-apps/plugin-fs") {
            violations.push(format!(
                "{} imports @tauri-apps/plugin-fs",
                path.strip_prefix(workspace_root).unwrap().display()
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "frontend filesystem access must be mediated by Rust commands:\n{}",
        violations.join("\n")
    );
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn collect_frontend_sources(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_frontend_sources(&path, files);
        } else if path.extension().is_some_and(|extension| {
            matches!(extension.to_str(), Some("ts" | "tsx" | "js" | "jsx"))
        }) {
            files.push(path);
        }
    }
}

fn is_test_source(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "tests")
        || path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with("_tests.rs"))
}

fn is_frontend_test_source(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component.as_os_str().to_str(), Some("tests" | "__tests__")))
        || path.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name.contains(".test.") || name.contains(".spec.")
        })
}
