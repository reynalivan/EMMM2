use std::fs;

use tempfile::tempdir;

use crate::domain::collection::{CollectionMod, CollectionObject, MemberKind, PreviewTreeNodeKind};
use crate::services::collection_preview_tree::{
    build_preview_tree, count_preview_mods, resolve_preview_terminal_metadata,
};

fn object(path_key: &str) -> CollectionObject {
    CollectionObject {
        kind: MemberKind::Object,
        collection_id: "collection-1".to_string(),
        object_id: "object-1".to_string(),
        is_enabled: true,
        display_name: Some("AINOZ".to_string()),
        path_key: Some(path_key.to_string()),
    }
}

fn member(mod_path: &str, display_name: &str) -> CollectionMod {
    CollectionMod {
        kind: MemberKind::Mod,
        collection_id: "collection-1".to_string(),
        mod_id: Some(display_name.to_string()),
        mod_path: mod_path.to_string(),
        mod_path_key: Some(display_name.to_string()),
        object_id: "object-1".to_string(),
        display_name: Some(display_name.to_string()),
        preview_path: None,
        node_type: None,
        warnings: Vec::new(),
        is_enabled: true,
    }
}

fn write_mod_ini(path: &std::path::Path) {
    fs::create_dir_all(path).expect("create mod dir");
    fs::write(path.join("mod.ini"), "[TextureOverrideTest]\nhash = 1234\n").expect("write ini");
}

#[test]
fn builds_multi_level_container_chain_for_terminal_modpack_root() {
    let temp = tempdir().expect("tempdir");
    let mods_root = temp.path().join("Mods");
    let mod_dir = mods_root
        .join("AINOZ")
        .join("ContainerA")
        .join("ContainerB")
        .join("Blue");
    write_mod_ini(&mod_dir);
    fs::write(mod_dir.join("body.dds"), b"dds").expect("write asset");
    fs::create_dir_all(mod_dir.join("Assets")).expect("create assets");

    let tree = build_preview_tree(
        &[object("AINOZ")],
        &[member("AINOZ/ContainerA/ContainerB/Blue", "Blue")],
        Some(&mods_root.to_string_lossy()),
    );

    let object_node = &tree[0];
    assert_eq!(object_node.kind, PreviewTreeNodeKind::Object);
    assert_eq!(object_node.children[0].name, "ContainerA");
    assert_eq!(object_node.children[0].children[0].name, "ContainerB");
    assert_eq!(object_node.children[0].children[0].children[0].name, "Blue");
    assert_eq!(
        object_node.children[0].children[0].children[0]
            .node_type
            .as_deref(),
        Some("ModPackRoot")
    );
    assert_eq!(count_preview_mods(&tree), 1);
}

#[test]
fn routes_disabled_container_branch_to_inactive_section() {
    let temp = tempdir().expect("tempdir");
    let mods_root = temp.path().join("Mods");
    let mod_dir = mods_root
        .join("AINOZ")
        .join("Outer")
        .join("DISABLED Inner")
        .join("Blue");
    write_mod_ini(&mod_dir);

    let tree = build_preview_tree(
        &[object("AINOZ")],
        &[member("AINOZ/Outer/DISABLED Inner/Blue", "Blue")],
        Some(&mods_root.to_string_lossy()),
    );

    let inactive_section = tree[0]
        .children
        .iter()
        .find(|child| child.node_type.as_deref() == Some("InactiveContainerSection"))
        .expect("inactive section");
    let outer = &inactive_section.children[0];
    let inner = &outer.children[0];
    let blue = &inner.children[0];

    assert_eq!(outer.name, "Outer");
    assert_eq!(inner.status_kind.as_deref(), Some("inactive_container"));
    assert_eq!(blue.status_kind.as_deref(), Some("disabled_by_container"));
    assert_eq!(count_preview_mods(&tree), 0);
}

#[test]
fn collapses_variant_container_at_main_folder_only_and_keeps_warning() {
    let temp = tempdir().expect("tempdir");
    let mods_root = temp.path().join("Mods");
    let variants_dir = mods_root.join("AINOZ").join("Looks").join("Variants");
    fs::create_dir_all(&variants_dir).expect("create variants");
    fs::write(variants_dir.join("variants.ini"), "").expect("write corrupt root ini");
    write_mod_ini(&variants_dir.join("PresetA"));
    write_mod_ini(&variants_dir.join("PresetB"));
    write_mod_ini(&variants_dir.join("PresetC"));

    let collection_object = object("AINOZ");
    let collection_member = member("AINOZ/Looks/Variants/PresetA", "PresetA");
    let preview_metadata = resolve_preview_terminal_metadata(
        Some(&collection_object),
        &collection_member,
        Some(&mods_root.to_string_lossy()),
    );

    assert_eq!(
        preview_metadata.node_type.as_deref(),
        Some("VariantContainer")
    );
    assert!(preview_metadata.preview_path.is_some());
    assert_eq!(preview_metadata.warnings.len(), 1);

    let tree = build_preview_tree(
        &[collection_object],
        &[CollectionMod {
            preview_path: preview_metadata.preview_path,
            node_type: preview_metadata.node_type,
            warnings: preview_metadata.warnings,
            ..collection_member
        }],
        Some(&mods_root.to_string_lossy()),
    );

    let looks = &tree[0].children[0];
    let variants = &looks.children[0];
    assert_eq!(looks.name, "Looks");
    assert_eq!(variants.node_type.as_deref(), Some("VariantContainer"));
    assert!(variants.collapse_children);
    assert!(variants.children.is_empty());
    assert_eq!(variants.warnings.len(), 1);
    assert_eq!(count_preview_mods(&tree), 1);
}

#[test]
fn keeps_flat_mod_terminal_name_without_extra_subfolders() {
    let temp = tempdir().expect("tempdir");
    let mods_root = temp.path().join("Mods");
    let mod_dir = mods_root.join("AINOZ").join("Loose Skin.ini");
    write_mod_ini(&mod_dir);

    let tree = build_preview_tree(
        &[object("AINOZ")],
        &[member("AINOZ/Loose Skin.ini", "Loose Skin.ini")],
        Some(&mods_root.to_string_lossy()),
    );

    let leaf = &tree[0].children[0];
    assert_eq!(leaf.kind, PreviewTreeNodeKind::Mod);
    assert_eq!(leaf.name, "Loose Skin.ini");
    assert_eq!(leaf.node_type.as_deref(), Some("FlatModRoot"));
}

#[test]
fn drops_container_only_branches_when_no_terminal_mods_exist() {
    let tree = build_preview_tree(
        &[object("AINOZ")],
        &[CollectionMod {
            preview_path: Some("AINOZ/New folder".to_string()),
            node_type: Some("ContainerFolder".to_string()),
            ..member("AINOZ/New folder", "New folder")
        }],
        None,
    );

    assert!(tree.is_empty());
}
