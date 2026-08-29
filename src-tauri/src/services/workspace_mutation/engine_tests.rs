use super::*;

#[tokio::test]
async fn toggle_mods_mixed_returns_runtime_path_rewrites() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(mods_path.join("Variant")).expect("mod folder");

    let result = toggle_mods_mixed(RuntimeToggleBatchRequest {
        mods_path: mods_path.clone(),
        operations: vec![RuntimeToggleOperation {
            folder_path: "Variant".to_string(),
            target_enabled: false,
        }],
    })
    .await
    .expect("toggle");

    assert_eq!(result.path_rewrites.len(), 1);
    assert_eq!(
        result.path_rewrites[0].old_path,
        mods_path.join("Variant").to_string_lossy().to_string()
    );
    assert_eq!(
        result.path_rewrites[0].new_path,
        mods_path
            .join("DISABLED Variant")
            .to_string_lossy()
            .to_string()
    );
    // Both sides reported for the caller's scoped reconcile.
    assert_eq!(result.changed_paths.len(), 2);
}

#[tokio::test]
async fn toggle_mods_mixed_repairs_stale_disabled_request_when_disk_is_enabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let enabled_path = mods_path.join("Variant");
    std::fs::create_dir_all(&enabled_path).expect("mod folder");

    // The caller asks by the stale DISABLED spelling; disk already enabled.
    let result = toggle_mods_mixed(RuntimeToggleBatchRequest {
        mods_path: mods_path.clone(),
        operations: vec![RuntimeToggleOperation {
            folder_path: "DISABLED Variant".to_string(),
            target_enabled: true,
        }],
    })
    .await
    .expect("toggle");

    assert_eq!(result.enabled_count + result.disabled_count, 1);
    assert_eq!(result.path_rewrites.len(), 1);
    assert_eq!(
        result.path_rewrites[0].old_path,
        mods_path
            .join("DISABLED Variant")
            .to_string_lossy()
            .to_string()
    );
    assert_eq!(
        result.path_rewrites[0].new_path,
        enabled_path.to_string_lossy().to_string()
    );
}

#[tokio::test]
async fn toggle_mods_mixed_returns_empty_result_for_empty_operations() {
    let result = toggle_mods_mixed(RuntimeToggleBatchRequest {
        mods_path: PathBuf::from("does-not-matter"),
        operations: Vec::new(),
    })
    .await
    .expect("empty batch is a no-op");

    assert_eq!(result.enabled_count + result.disabled_count, 0);
    assert!(result.warnings.is_empty());
    assert!(result.path_rewrites.is_empty());
    assert!(result.changed_paths.is_empty());
}

#[tokio::test]
async fn toggle_mods_mixed_rejects_absolute_and_traversal_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let absolute = temp.path().join("Variant").to_string_lossy().to_string();

    let absolute_error = toggle_mods_mixed(RuntimeToggleBatchRequest {
        mods_path: temp.path().to_path_buf(),
        operations: vec![RuntimeToggleOperation {
            folder_path: absolute,
            target_enabled: false,
        }],
    })
    .await
    .expect_err("absolute path must be rejected");
    assert!(
        absolute_error
            .to_string()
            .contains("Absolute mod folder path is not allowed"),
        "{absolute_error}"
    );

    let traversal_error = toggle_mods_mixed(RuntimeToggleBatchRequest {
        mods_path: temp.path().to_path_buf(),
        operations: vec![RuntimeToggleOperation {
            folder_path: "../Escape".to_string(),
            target_enabled: false,
        }],
    })
    .await
    .expect_err("parent traversal must be rejected");
    assert!(
        traversal_error
            .to_string()
            .contains("Unsafe mod folder path is not allowed"),
        "{traversal_error}"
    );
}

#[tokio::test]
async fn toggle_mods_mixed_errors_when_mod_folder_is_missing_on_disk() {
    let temp = tempfile::tempdir().expect("tempdir");

    let error = toggle_mods_mixed(RuntimeToggleBatchRequest {
        mods_path: temp.path().to_path_buf(),
        operations: vec![RuntimeToggleOperation {
            folder_path: "Ghost".to_string(),
            target_enabled: false,
        }],
    })
    .await
    .expect_err("missing folder must error");
    assert!(
        error.to_string().contains("Mod folder does not exist"),
        "{error}"
    );
}

#[tokio::test]
async fn toggle_mods_mixed_rejects_duplicate_source_paths_before_renaming() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(mods_path.join("Variant")).expect("mod folder");

    let operation = RuntimeToggleOperation {
        folder_path: "Variant".to_string(),
        target_enabled: false,
    };
    let error = toggle_mods_mixed(RuntimeToggleBatchRequest {
        mods_path: mods_path.clone(),
        operations: vec![operation.clone(), operation],
    })
    .await
    .expect_err("duplicate source must be rejected");

    assert!(
        error
            .to_string()
            .contains("Duplicate mutation source path detected"),
        "{error}"
    );
    // Validation happens before any rename: disk untouched.
    assert!(mods_path.join("Variant").exists());
    assert!(!mods_path.join("DISABLED Variant").exists());
}

#[test]
fn incomplete_rollback_is_reported_for_reconciliation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let old_abs = temp.path().join("Variant");
    let new_abs = temp.path().join("DISABLED Variant");
    std::fs::create_dir_all(&old_abs).expect("old path");
    std::fs::create_dir_all(&new_abs).expect("new path");
    let plan = RenamePlan {
        old_abs: old_abs.clone(),
        requested_abs: old_abs,
        new_abs,
        target_enabled: false,
    };
    let mut warnings = Vec::new();

    rollback_successes(&[plan], &mut warnings);

    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("target already exists"));
}
