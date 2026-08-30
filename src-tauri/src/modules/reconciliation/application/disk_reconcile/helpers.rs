use std::path::Path;

use crate::modules::workspace::domain::normalizer::{is_disabled_folder, normalize_display_name};
use crate::shared::safety_constants::SAFETY_SOURCE_MANUAL;
use crate::modules::games::domain::models::ItemStatus;
use crate::modules::workspace::application::scanner::sync::helpers::classify_safety;

#[derive(Debug, Clone)]
pub struct RuntimeModMetadata {
    pub actual_name: String,
    pub is_safe: bool,
    pub safety_source: &'static str,
    pub status: ItemStatus,
}

/// A mod's status comes from its own folder name only. A disabled parent object
/// must never cascade into its children: re-enabling the object has to restore
/// exactly the per-mod states the user left behind.
pub fn load_runtime_mod_metadata(
    mod_path: &Path,
    raw_folder_name: &str,
    safe_mode_keywords: &[String],
    existing_manual_safe: Option<bool>,
) -> RuntimeModMetadata {
    let mut actual_name = normalize_display_name(raw_folder_name).into_owned();
    let mut info_is_safe = None;

    match crate::modules::library::application::mods::info_json::read_info_json(mod_path) {
        Ok(Some(info)) => {
            let info_name = info.actual_name.trim();
            if !info_name.is_empty() {
                actual_name = info_name.to_string();
            }
            info_is_safe = Some(info.is_safe);
        }
        Ok(None) => {}
        Err(error) => {
            log::warn!(
                "Disk Reconcile failed to read info.json for '{}': {}",
                mod_path.display(),
                error
            );
        }
    }

    // The latest explicit in-app classification is authoritative. info.json is
    // only the first-index fallback for folders that have no remembered choice.
    let (is_safe, safety_source) = match existing_manual_safe.or(info_is_safe) {
        Some(value) => (value, SAFETY_SOURCE_MANUAL),
        None => classify_safety(&actual_name, safe_mode_keywords),
    };

    RuntimeModMetadata {
        actual_name,
        is_safe,
        safety_source,
        status: ItemStatus::from_is_disabled(is_disabled_folder(raw_folder_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::load_runtime_mod_metadata;
    use crate::modules::games::domain::models::ItemStatus;

    #[test]
    fn object_disabled_does_not_mutate_child_mod_status() {
        let temp = tempfile::tempdir().expect("tempdir should be created");

        let metadata = load_runtime_mod_metadata(temp.path(), "Blue Dress", &[], None);

        assert_eq!(metadata.status, ItemStatus::Enabled);
    }

    #[test]
    fn disabled_mod_folder_controls_mod_status() {
        let temp = tempfile::tempdir().expect("tempdir should be created");

        let metadata = load_runtime_mod_metadata(temp.path(), "DISABLED Blue Dress", &[], None);

        assert_eq!(metadata.status, ItemStatus::Disabled);
    }

    #[test]
    fn remembered_manual_classification_wins_over_stale_info_json() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        std::fs::write(
            temp.path().join("info.json"),
            r#"{"actual_name":"Blue Dress","is_safe":false}"#,
        )
        .expect("write metadata");

        let metadata = load_runtime_mod_metadata(temp.path(), "Blue Dress", &[], Some(true));

        assert!(metadata.is_safe);
    }
}
