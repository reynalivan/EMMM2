use crate::services::explorer::helpers::{
    analyze_mod_metadata, contains_filtered_keyword, normalize_keywords,
};
use crate::services::explorer::types::ModFolder;
use tempfile::TempDir;

#[test]
fn test_normalize_keywords() {
    let input = vec!["  Hello ".to_string(), "WORLD".to_string(), "".to_string()];
    let output = normalize_keywords(&input);
    assert_eq!(output, vec!["hello".to_string(), "world".to_string()]);
}

#[test]
fn test_contains_filtered_keyword() {
    let folder = ModFolder {
        id: None,
        node_type: "ModPackRoot".into(),
        classification_reasons: vec![],
        name: "Beautiful Sky".into(),
        folder_name: "beautiful_sky".into(),
        path: "/mock/path".into(),
        is_enabled: true,
        is_directory: true,
        thumbnail_path: None,
        modified_at: 0,
        size_bytes: 0,
        has_info_json: false,
        is_favorite: false,
        is_misplaced: false,
        is_safe: true,
        metadata: None,
        category: None,
        conflict_group_id: None,
        conflict_state: None,
    };

    assert!(contains_filtered_keyword(&folder, &["sky".into()]));
    assert!(!contains_filtered_keyword(&folder, &["ocean".into()]));

    // Empty keywords should return false
    assert!(!contains_filtered_keyword(&folder, &[]));
}

#[test]
fn test_analyze_mod_metadata_missing() {
    let temp_dir = TempDir::new().unwrap();
    let result = analyze_mod_metadata(temp_dir.path(), None);
    assert!(!result.has_info_json);
}
