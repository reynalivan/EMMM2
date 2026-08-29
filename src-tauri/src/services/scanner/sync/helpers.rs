use crate::common::safety_constants::{SAFETY_SOURCE_AUTO_TAGGED, SAFETY_SOURCE_UNKNOWN};

pub fn canonical_entry_key(entry_name: &str) -> String {
    crate::common::path_key::canonical_name_key(entry_name)
}

pub fn classify_safety(display_name: &str, safe_mode_keywords: &[String]) -> (bool, &'static str) {
    let folder_name_lower = display_name.to_lowercase();
    let keyword_match = safe_mode_keywords
        .iter()
        .any(|kw| folder_name_lower.contains(&kw.to_lowercase()));

    if keyword_match {
        return (false, SAFETY_SOURCE_AUTO_TAGGED);
    }

    (true, SAFETY_SOURCE_UNKNOWN)
}
