use crate::services::explorer::classifier::{classify_folder, NodeType};
use std::fs;
use tempfile::TempDir;

#[test]
fn empty_dir_is_container() {
    let tmp = TempDir::new().unwrap();
    let (node_type, _, _) = classify_folder(tmp.path());
    assert_eq!(node_type, NodeType::ContainerFolder);
}


#[test]
fn unicode_folder_names_are_classified_without_dropping_children() {
    let tmp = TempDir::new().unwrap();
    let variant_root = tmp.path().join("日本語Mod");
    fs::create_dir_all(variant_root.join("内部")).unwrap();
    fs::write(
        variant_root.join("mod.ini"),
        "[TextureOverrideTest]\nfilename=内部\\texture.dds\n",
    )
    .unwrap();

    let (node_type, reasons, _) = classify_folder(&variant_root);

    assert_eq!(node_type, NodeType::FlatModRoot);
    assert!(reasons.iter().any(|reason| reason.contains("内部")));
}

