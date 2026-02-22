use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::services::config::ConfigService;
use crate::DISABLED_PREFIX;

use super::types::{InfoAnalysis, ModFolder};

pub(crate) fn analyze_mod_metadata(path: &Path, sub_path: Option<&str>) -> InfoAnalysis {
    if !path.join("info.json").exists() {
        return InfoAnalysis::default();
    }

    match crate::services::mod_files::info_json::read_info_json(path) {
        Ok(Some(info)) => {
            let is_misplaced = sub_path.map_or(false, |sp| {
                let current_cat = sp.split(['/', '\\']).next().unwrap_or(sp);
                info.metadata.get("character").map_or(false, |meta_char| {
                    !meta_char.eq_ignore_ascii_case(current_cat)
                })
            });
            let category = info.metadata.get("category").cloned();
            let metadata = if info.metadata.is_empty() {
                None
            } else {
                Some(info.metadata.clone())
            };
            InfoAnalysis {
                has_info_json: true,
                is_favorite: info.is_favorite,
                is_misplaced,
                is_safe: info.is_safe,
                metadata,
                category,
            }
        }
        Ok(None) => InfoAnalysis::default(),
        Err(_) => InfoAnalysis {
            has_info_json: true,
            ..InfoAnalysis::default()
        },
    }
}

pub(crate) fn normalize_keywords(keywords: &[String]) -> Vec<String> {
    keywords
        .iter()
        .map(|k| k.trim().to_lowercase())
        .filter(|k| !k.is_empty())
        .collect()
}

pub(crate) fn contains_filtered_keyword(folder: &ModFolder, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return false;
    }

    let mut haystacks = vec![
        folder.name.to_lowercase(),
        folder.folder_name.to_lowercase(),
    ];

    if let Ok(Some(info)) =
        crate::services::mod_files::info_json::read_info_json(Path::new(&folder.path))
    {
        haystacks.push(info.actual_name.to_lowercase());
        haystacks.push(info.author.to_lowercase());
        haystacks.push(info.description.to_lowercase());
        haystacks.extend(info.tags.into_iter().map(|tag| tag.to_lowercase()));
    }

    keywords
        .iter()
        .any(|keyword| haystacks.iter().any(|value| value.contains(keyword)))
}

pub(crate) fn apply_safe_mode_filter(
    folders: Vec<ModFolder>,
    config: &ConfigService,
) -> Vec<ModFolder> {
    let settings = config.get_settings();
    if !settings.safe_mode.enabled {
        return folders;
    }

    let keywords = normalize_keywords(&settings.safe_mode.keywords);
    let force_exclusive_mode = settings.safe_mode.force_exclusive_mode;

    folders
        .into_iter()
        .filter(|folder| {
            folder.is_safe
                && (!force_exclusive_mode || !contains_filtered_keyword(folder, &keywords))
        })
        .collect()
}

/// Returns true if the mod's physical parent folder doesn't match the DB object's folder name.
pub(crate) fn is_db_misplaced(
    mod_obj_id: Option<String>,
    resolved_path: &Path,
    obj_folder_map: &HashMap<String, String>,
) -> bool {
    let oid = match mod_obj_id {
        Some(ref id) => id,
        None => return false,
    };
    let obj_folder = match obj_folder_map.get(oid) {
        Some(f) => f,
        None => return false,
    };
    let parent_name = resolved_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string());
    let parent_name = match parent_name {
        Some(ref n) => n,
        None => return false,
    };
    let clean_parent = parent_name
        .strip_prefix(DISABLED_PREFIX)
        .unwrap_or(parent_name);
    !clean_parent.eq_ignore_ascii_case(obj_folder)
}

/// Try to find a mod folder on disk when the DB path is stale.
/// Checks if the alternate-prefixed version exists (add/remove DISABLED prefix).
/// Returns (resolved_path, is_enabled) or None if truly gone.
pub(crate) fn try_resolve_alternate(db_path: &Path) -> Option<(PathBuf, bool)> {
    let parent = db_path.parent()?;
    let fname = db_path.file_name()?.to_string_lossy();

    if let Some(stripped) = fname.strip_prefix(DISABLED_PREFIX) {
        // DB says DISABLED, check if enabled version exists on disk
        let alt = parent.join(stripped);
        if alt.exists() {
            return Some((alt, true));
        }
    } else {
        // DB says enabled, check if DISABLED version exists on disk
        let alt = parent.join(format!("{}{}", DISABLED_PREFIX, fname));
        if alt.exists() {
            return Some((alt, false));
        }
    }
    None
}
