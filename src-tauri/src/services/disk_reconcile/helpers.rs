use std::path::Path;

use crate::database::models::ItemStatus;
use crate::services::corridor_constants::{
    CORRIDOR_SOURCE_AUTO_TAGGED, CORRIDOR_SOURCE_MANUAL, CORRIDOR_SOURCE_UNKNOWN,
};

#[derive(Debug, Clone)]
pub struct RuntimeModMetadata {
    pub actual_name: String,
    pub is_safe: bool,
    pub corridor_source: &'static str,
    pub status: ItemStatus,
}

pub fn generate_stable_mod_id(game_id: &str, folder_path: &str) -> String {
    let key = crate::services::path_key::folder_path_key(folder_path, None);
    let input = format!("{game_id}:{key}");
    let hash = blake3::hash(input.as_bytes());
    hash.to_hex()[..32].to_string()
}

pub fn normalize_runtime_name(name: &str) -> String {
    crate::services::scanner::core::normalizer::normalize_display_name(name)
}

pub fn is_disabled_runtime_name(name: &str) -> bool {
    crate::services::scanner::core::normalizer::is_disabled_folder(name)
}

pub fn classify_runtime_corridor(
    display_name: &str,
    safe_mode_keywords: &[String],
) -> (bool, &'static str) {
    let folder_name_lower = display_name.to_lowercase();
    let keyword_match = safe_mode_keywords
        .iter()
        .any(|kw| folder_name_lower.contains(&kw.to_lowercase()));

    if keyword_match {
        return (false, CORRIDOR_SOURCE_AUTO_TAGGED);
    }

    (true, CORRIDOR_SOURCE_UNKNOWN)
}

pub fn load_runtime_mod_metadata(
    mod_path: &Path,
    raw_folder_name: &str,
    _object_disabled: bool,
    safe_mode_keywords: &[String],
    existing_manual_safe: Option<bool>,
) -> RuntimeModMetadata {
    let fallback_name = normalize_runtime_name(raw_folder_name);
    let mut actual_name = fallback_name.clone();
    let mut is_safe = None;
    let mut corridor_source = None;

    match crate::services::mods::info_json::read_info_json(mod_path) {
        Ok(Some(info)) => {
            let info_name = info.actual_name.trim();
            if !info_name.is_empty() {
                actual_name = info_name.to_string();
            }
            is_safe = Some(info.is_safe);
            corridor_source = Some(CORRIDOR_SOURCE_MANUAL);
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

    let (resolved_is_safe, resolved_corridor_source) = if let Some(value) = is_safe {
        (value, corridor_source.unwrap_or(CORRIDOR_SOURCE_MANUAL))
    } else if let Some(value) = existing_manual_safe {
        (value, CORRIDOR_SOURCE_MANUAL)
    } else {
        classify_runtime_corridor(&actual_name, safe_mode_keywords)
    };

    RuntimeModMetadata {
        actual_name,
        is_safe: resolved_is_safe,
        corridor_source: resolved_corridor_source,
        status: ItemStatus::from_is_disabled(is_disabled_runtime_name(raw_folder_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::load_runtime_mod_metadata;
    use crate::database::models::ItemStatus;

    #[test]
    fn object_disabled_does_not_mutate_child_mod_status() {
        let temp = tempfile::tempdir().expect("tempdir should be created");

        let metadata = load_runtime_mod_metadata(temp.path(), "Blue Dress", true, &[], None);

        assert_eq!(metadata.status, ItemStatus::Enabled);
    }

    #[test]
    fn disabled_mod_folder_controls_mod_status() {
        let temp = tempfile::tempdir().expect("tempdir should be created");

        let metadata =
            load_runtime_mod_metadata(temp.path(), "DISABLED Blue Dress", false, &[], None);

        assert_eq!(metadata.status, ItemStatus::Disabled);
    }
}
