use std::path::Path;

use crate::services::config::ConfigService;

use crate::services::explorer::types::{InfoAnalysis, ModFolder};

pub fn analyze_mod_metadata(path: &Path, sub_path: Option<&str>) -> InfoAnalysis {
    if !path.join("info.json").exists() {
        return InfoAnalysis::default();
    }

    match crate::services::mods::info_json::read_info_json(path) {
        Ok(Some(info)) => {
            let is_misplaced = sub_path.is_some_and(|sp| {
                let current_cat = sp.split(['/', '\\']).next().unwrap_or(sp);
                info.metadata
                    .get("character")
                    .is_some_and(|meta_char| !meta_char.eq_ignore_ascii_case(current_cat))
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

pub fn normalize_keywords(keywords: &[String]) -> Vec<String> {
    keywords
        .iter()
        .map(|k| k.trim().to_lowercase())
        .filter(|k| !k.is_empty())
        .collect()
}

pub fn contains_filtered_keyword(folder: &ModFolder, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return false;
    }

    let mut haystacks = vec![
        folder.name.to_lowercase(),
        folder.folder_name.to_lowercase(),
    ];

    if let Ok(Some(info)) =
        crate::services::mods::info_json::read_info_json(Path::new(&folder.path))
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

pub fn apply_safe_mode_filter(folders: Vec<ModFolder>, config: &ConfigService) -> Vec<ModFolder> {
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

#[cfg(test)]
#[path = "tests/helpers_tests.rs"]
mod tests;
