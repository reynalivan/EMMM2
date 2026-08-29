use std::path::Path;

use crate::services::explorer::types::InfoAnalysis;

pub fn analyze_mod_metadata(path: &Path, sub_path: Option<&str>) -> InfoAnalysis {
    // `read_info_json` already reports a missing file as `Ok(None)`; a
    // pre-flight `exists()` would just be a second stat per listed folder.
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
