use crate::shared::errors::AppError;
#[cfg(any(debug_assertions, test, feature = "readonly-audit"))]
use std::path::{Path, PathBuf};
#[cfg(any(debug_assertions, feature = "readonly-audit"))]
use std::sync::OnceLock;

const AUDIT_APP_IDENTIFIER: &str = "com.reynalivan.emmm.e2e.readonly-audit";

#[cfg(not(test))]
const AUDIT_ROOT_ENVIRONMENT: &str = "EMMM_READ_ONLY_AUDIT_ROOT";
#[cfg(any(debug_assertions, feature = "readonly-audit"))]
static AUDIT_ROOT: OnceLock<PathBuf> = OnceLock::new();

#[cfg(not(test))]
/// Validate native test identity and optional read-only audit before bootstrap can write.
pub(crate) fn initialize_bootstrap_guards(
    identifier: &str,
    app_data: &std::path::Path,
) -> Result<(), AppError> {
    let expected = std::env::var_os("EMMM_E2E_EXPECTED_IDENTIFIER");
    let expected = expected
        .as_deref()
        .map(|value| {
            value.to_str().ok_or_else(|| {
                AppError::Validation(
                    "Expected native E2E identifier must be valid Unicode".to_string(),
                )
            })
        })
        .transpose()?;
    ensure_expected_native_identifier(identifier, expected)?;
    let root = std::env::var_os(AUDIT_ROOT_ENVIRONMENT);
    ensure_audit_root_present(identifier, root.is_some())?;
    let Some(root) = root else {
        return Ok(());
    };
    initialize_readonly_audit(identifier, std::path::Path::new(&root), app_data)
}

#[cfg(any(not(test), not(debug_assertions)))]
fn initialize_readonly_audit(
    identifier: &str,
    root: &std::path::Path,
    app_data: &std::path::Path,
) -> Result<(), AppError> {
    #[cfg(any(debug_assertions, feature = "readonly-audit"))]
    {
        let root = validate_audit_configuration(identifier, root, app_data)?;
        let journal = crate::modules::mutation::journal::OperationJournal::open(
            app_data.join("mutation-journal.json"),
            usize::MAX,
        )?;
        ensure_empty_journal(journal.entries().len())?;
        AUDIT_ROOT.set(root).map_err(|_| {
            AppError::Validation("Read-only startup audit was initialized twice".to_string())
        })?;
        log::warn!("Read-only native startup audit enabled; runtime publication and unapproved IPC are blocked");
        Ok(())
    }
    #[cfg(not(any(debug_assertions, feature = "readonly-audit")))]
    {
        let _ = (identifier, app_data, root);
        Err(AppError::Validation(
            "Read-only startup audit requires a debug E2E build or explicit readonly-audit feature"
                .to_string(),
        ))
    }
}

fn ensure_expected_native_identifier(
    identifier: &str,
    expected: Option<&str>,
) -> Result<(), AppError> {
    if let Some(expected) = expected {
        if identifier != expected
            || !matches!(expected, "com.reynalivan.emmm.e2e" | AUDIT_APP_IDENTIFIER)
        {
            return Err(AppError::Validation(
                "Native E2E refused a wrong-profile binary before configuration or recovery"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn ensure_audit_root_present(identifier: &str, root_present: bool) -> Result<(), AppError> {
    if identifier == AUDIT_APP_IDENTIFIER && !root_present {
        return Err(AppError::Validation(
            "The private read-only audit app requires EMMM_READ_ONLY_AUDIT_ROOT on every launch"
                .to_string(),
        ));
    }
    Ok(())
}

#[cfg(any(debug_assertions, test, feature = "readonly-audit"))]
fn canonical_existing_ancestor(path: &Path) -> Result<PathBuf, AppError> {
    if !path.is_absolute() {
        return Err(AppError::Validation(
            "Audit paths must be absolute".to_string(),
        ));
    }
    let mut current = path;
    let mut missing = Vec::new();
    while !current.exists() {
        missing.push(current.file_name().ok_or_else(|| {
            AppError::Validation("Audit app-data has no existing ancestor".to_string())
        })?);
        current = current.parent().ok_or_else(|| {
            AppError::Validation("Audit app-data has no existing ancestor".to_string())
        })?;
    }
    let mut resolved = current.canonicalize()?;
    for component in missing.into_iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

#[cfg(any(debug_assertions, test, feature = "readonly-audit"))]
fn validate_audit_configuration(
    identifier: &str,
    root: &Path,
    app_data: &Path,
) -> Result<PathBuf, AppError> {
    if identifier != AUDIT_APP_IDENTIFIER || !root.is_absolute() || !root.is_dir() {
        return Err(AppError::Validation(
            "Read-only audit requires the isolated E2E identifier and an existing absolute directory".to_string(),
        ));
    }
    let root = root.canonicalize()?;
    let data = canonical_existing_ancestor(app_data)?;
    let root_key = crate::shared::path_key::exact_location_key_for_path(&root);
    let data_key = crate::shared::path_key::exact_location_key_for_path(&data);
    if root_key == data_key
        || root_key.starts_with(&format!("{data_key}/"))
        || data_key.starts_with(&format!("{root_key}/"))
    {
        return Err(AppError::Validation(
            "Audit app-data must not overlap the protected real directory".to_string(),
        ));
    }
    Ok(root)
}

pub(crate) fn is_active() -> bool {
    #[cfg(test)]
    if TEST_AUDIT.try_with(|_| ()).is_ok() {
        return true;
    }
    #[cfg(any(debug_assertions, feature = "readonly-audit"))]
    {
        AUDIT_ROOT.get().is_some()
    }
    #[cfg(not(any(debug_assertions, feature = "readonly-audit")))]
    {
        false
    }
}

pub(crate) fn ensure_empty_journal(entry_count: usize) -> Result<(), AppError> {
    if entry_count != 0 {
        return Err(AppError::Validation(
            "Read-only audit refuses mutation recovery: the isolated journal must be empty"
                .to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn ensure_disabled_hotkeys(enabled: bool) -> Result<(), AppError> {
    if enabled {
        return Err(AppError::Validation(
            "Read-only audit requires hotkeys.enabled=false before startup".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn command_allowed_during_audit(command: &str) -> bool {
    matches!(
        command,
        "get_settings"
            | "set_active_game"
            | "check_config_status"
            | "check_path_exists_cmd"
            | "app_startup_check"
            | "get_pending_crash_report"
            | "record_native_error_metric"
            | "get_dashboard_stats"
            | "get_active_keybindings"
            | "reconcile_disk_state_cmd"
            | "get_objects_cmd"
            | "get_category_counts_cmd"
            | "list_custom_themes"
            | "load_custom_theme"
            | "get_game_schema"
            | "get_catalog_pack_status"
            | "get_object_identity_suggestion_status"
            | "list_object_identity_suggestions"
            | "get_collection_runtime_descriptor"
            | "get_collection_runtime_state"
            | "list_collections"
            | "get_apply_progress"
            | "get_onboarding_indexing_background_status"
            | "get_workspace_structure"
            | "get_workspace_explorer_page"
            | "get_workspace_preview"
            | "get_keyviewer_runtime_diagnostics"
            | "get_logs"
            | "exit_app"
    )
}

#[cfg(test)]
tokio::task_local! {
    static TEST_AUDIT: ();
}

#[cfg(test)]
pub(super) async fn with_test_audit<F: std::future::Future>(future: F) -> F::Output {
    TEST_AUDIT.scope((), future).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(not(feature = "readonly-audit"), not(debug_assertions)))]
    #[test]
    fn ordinary_release_refuses_audit_root_without_explicit_feature() {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let error = initialize_readonly_audit(AUDIT_APP_IDENTIFIER, root.path(), data.path())
            .expect_err("ordinary release must fail closed before an audit launch");
        assert!(error
            .to_string()
            .contains("explicit readonly-audit feature"));
        assert!(!is_active());
    }

    #[cfg(all(feature = "readonly-audit", not(debug_assertions)))]
    #[test]
    fn explicit_audit_feature_protects_release_bootstrap() {
        const CHILD_FLAG: &str = "EMMM_RELEASE_AUDIT_TEST_CHILD";
        if std::env::var_os(CHILD_FLAG).is_some() {
            let root = tempfile::tempdir().unwrap();
            let data = tempfile::tempdir().unwrap();
            initialize_readonly_audit(AUDIT_APP_IDENTIFIER, root.path(), data.path())
                .expect("explicit audit feature must enable protection without debug assertions");
            assert!(
                is_active(),
                "release audit must remain active after bootstrap"
            );
            assert!(!command_allowed_during_audit("write_mod_ini"));
            assert!(ensure_disabled_hotkeys(true).is_err());
            return;
        }
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "modules::system::application::app::readonly_audit::tests::explicit_audit_feature_protects_release_bootstrap",
                "--nocapture",
            ])
            .env(CHILD_FLAG, "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn expected_native_identifier_rejects_wrong_profile_before_bootstrap() {
        let expected = "com.reynalivan.emmm.e2e";
        assert!(ensure_expected_native_identifier(expected, Some(expected)).is_ok());
        assert!(ensure_expected_native_identifier(
            AUDIT_APP_IDENTIFIER,
            Some(AUDIT_APP_IDENTIFIER)
        )
        .is_ok());
        assert!(ensure_expected_native_identifier("com.reynalivan.emmm", None).is_ok());
        assert!(ensure_expected_native_identifier("com.reynalivan.emmm", Some(expected)).is_err());
        assert!(ensure_expected_native_identifier(
            "com.reynalivan.emmm",
            Some("com.reynalivan.emmm")
        )
        .is_err());
        assert!(ensure_expected_native_identifier(AUDIT_APP_IDENTIFIER, Some(expected)).is_err());
    }

    #[test]
    fn readonly_audit_rejects_wrong_identifier_or_non_absolute_root() {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        assert!(
            validate_audit_configuration("com.reynalivan.emmm.e2e", root.path(), data.path())
                .is_err()
        );
        assert!(validate_audit_configuration(
            "com.reynalivan.emmm.e2e.readonly-audit",
            root.path(),
            data.path()
        )
        .is_ok());
        assert!(
            validate_audit_configuration("com.reynalivan.emmm", root.path(), data.path()).is_err()
        );
        assert!(validate_audit_configuration(
            AUDIT_APP_IDENTIFIER,
            Path::new("relative"),
            data.path()
        )
        .is_err());
        assert!(
            validate_audit_configuration(AUDIT_APP_IDENTIFIER, root.path(), data.path()).is_ok()
        );
    }

    #[test]
    fn readonly_audit_private_profile_requires_root_on_every_launch() {
        assert!(
            ensure_audit_root_present("com.reynalivan.emmm.e2e.readonly-audit", false).is_err()
        );
        assert!(ensure_audit_root_present("com.reynalivan.emmm.e2e.readonly-audit", true).is_ok());
        assert!(ensure_audit_root_present("com.reynalivan.emmm.e2e", false).is_ok());
        assert!(ensure_audit_root_present("com.reynalivan.emmm", false).is_ok());
    }

    #[test]
    fn readonly_audit_rejects_overlapping_app_data_and_file_root() {
        let root = tempfile::tempdir().unwrap();
        assert!(validate_audit_configuration(
            AUDIT_APP_IDENTIFIER,
            root.path(),
            &root.path().join("new-data")
        )
        .is_err());
        assert!(validate_audit_configuration(
            AUDIT_APP_IDENTIFIER,
            root.path(),
            root.path().parent().unwrap()
        )
        .is_err());
        let file = root.path().join("file");
        std::fs::write(&file, b"untouched").unwrap();
        let data = tempfile::tempdir().unwrap();
        assert!(validate_audit_configuration(AUDIT_APP_IDENTIFIER, &file, data.path()).is_err());
        assert_eq!(std::fs::read(file).unwrap(), b"untouched");
    }

    #[test]
    fn readonly_audit_rejects_nonempty_mutation_journal() {
        assert!(ensure_empty_journal(0).is_ok());
        assert!(ensure_empty_journal(1).is_err());
        assert!(ensure_disabled_hotkeys(false).is_ok());
        assert!(ensure_disabled_hotkeys(true).is_err());
    }

    #[test]
    fn readonly_audit_denies_mutating_and_unknown_ipc() {
        for command in [
            "write_mod_ini",
            "execute_workspace_switch",
            "run_maintenance",
            "save_settings",
            "reset_database",
            "apply_game_mods_directory",
            "unknown_command",
        ] {
            assert!(!command_allowed_during_audit(command), "{command}");
        }
        for command in [
            "get_settings",
            "get_active_keybindings",
            "get_dashboard_stats",
            "reconcile_disk_state_cmd",
            "set_active_game",
            "get_logs",
        ] {
            assert!(command_allowed_during_audit(command), "{command}");
        }
    }
}
