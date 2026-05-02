use crate::services::explorer::helpers::{
    analyze_mod_metadata, apply_runtime_corridor_filter_to_response, contains_filtered_keyword,
    normalize_keywords,
};
use crate::services::explorer::types::{
    ConflictGroup, ConflictMember, FolderGridResponse, ModFolder,
};
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
        node_type: "ModPackRoot".into(),
        classification_reasons: vec![],
        id: None,
        owner_object_id: None,
        owner_object_folder_path: None,
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
        warnings: vec![],
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

fn build_folder(path: &str, is_safe: bool) -> ModFolder {
    ModFolder {
        node_type: "ModPackRoot".into(),
        classification_reasons: vec!["has-mod-ini".into()],
        id: None,
        owner_object_id: None,
        owner_object_folder_path: None,
        name: path.rsplit('/').next().unwrap_or(path).into(),
        folder_name: path.rsplit('/').next().unwrap_or(path).into(),
        path: path.into(),
        is_enabled: true,
        is_directory: true,
        thumbnail_path: None,
        modified_at: 0,
        size_bytes: 0,
        has_info_json: false,
        is_favorite: false,
        is_misplaced: false,
        is_safe,
        metadata: None,
        category: None,
        conflict_group_id: None,
        conflict_state: None,
        warnings: vec!["corrupt-ini-0kb".into()],
    }
}

#[test]
fn runtime_corridor_filter_keeps_only_matching_corridor_and_prunes_conflicts() {
    let safe_folder = build_folder("E:/Mods/Safe Outfit", true);
    let unsafe_folder = build_folder("E:/Mods/Private Outfit", false);
    let response = FolderGridResponse {
        self_node_type: Some("ContainerFolder".into()),
        self_is_mod: false,
        self_is_enabled: true,
        self_owner_object_id: None,
        self_owner_object_folder_path: None,
        self_classification_reasons: vec!["container".into()],
        children: vec![safe_folder.clone(), unsafe_folder.clone()],
        conflicts: vec![ConflictGroup {
            group_id: "cg-1".into(),
            base_name: "Outfit".into(),
            members: vec![
                ConflictMember {
                    path: safe_folder.path.clone(),
                    folder_name: safe_folder.folder_name.clone(),
                    is_enabled: true,
                    modified_at: 0,
                    size_bytes: 0,
                },
                ConflictMember {
                    path: unsafe_folder.path.clone(),
                    folder_name: unsafe_folder.folder_name.clone(),
                    is_enabled: true,
                    modified_at: 0,
                    size_bytes: 0,
                },
            ],
        }],
        ancestor_disabled_by: Some("Variants".into()),
        ancestor_disabled_path: Some("E:/Mods/Object/DISABLED Variants".into()),
    };

    let filtered = apply_runtime_corridor_filter_to_response(response, true);

    assert_eq!(filtered.children.len(), 1);
    assert_eq!(filtered.children[0].path, safe_folder.path);
    assert_eq!(
        filtered.children[0].classification_reasons,
        vec!["has-mod-ini"]
    );
    assert_eq!(filtered.children[0].warnings, vec!["corrupt-ini-0kb"]);
    assert!(filtered.conflicts.is_empty());
    assert_eq!(filtered.ancestor_disabled_by.as_deref(), Some("Variants"));
    assert_eq!(
        filtered.ancestor_disabled_path.as_deref(),
        Some("E:/Mods/Object/DISABLED Variants")
    );
}
