use crate::modules::workspace::domain::classifier::{
    classify_folder, classify_folder_strict, classify_folder_strict_from_scan, scan_folder_strict,
    NodeType,
};
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

    let (nt, reasons, _warnings) = classify_folder(&variant_root);

    assert_eq!(nt, NodeType::FlatModRoot);
    assert!(reasons.iter().any(|reason| reason.contains("内部")));
}

#[test]
fn shift_jis_orchestrator_is_classified_as_variant_container() {
    let tmp = TempDir::new().unwrap();
    let variant_root = tmp.path().join("MergedMod");
    for child in ["VariantA", "VariantB"] {
        let child_path = variant_root.join(child);
        fs::create_dir_all(&child_path).unwrap();
        fs::write(
            child_path.join("DISABLED child.ini"),
            "[TextureOverrideChild]\nhash = 12345678\n",
        )
        .unwrap();
    }

    let source = "; Japanese: 日本語\n[ResourceA]\nfilename = VariantA/body.buf\n\
[ResourceB]\nfilename = VariantB/body.buf\n";
    let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(source);
    assert!(!had_errors);
    fs::write(variant_root.join("merged.ini"), encoded.as_ref()).unwrap();

    let (node_type, _, _) = classify_folder(&variant_root);

    assert_eq!(node_type, NodeType::VariantContainer);
}

#[test]
fn strict_classification_reuses_a_previously_scanned_directory() {
    let tmp = TempDir::new().unwrap();
    let mod_root = tmp.path().join("Blue Dress");
    fs::create_dir_all(mod_root.join("Assets")).unwrap();
    fs::write(
        mod_root.join("mod.ini"),
        "[TextureOverrideTest]\nhash = abc\n",
    )
    .unwrap();
    fs::write(mod_root.join("mesh.buf"), b"asset").unwrap();

    let expected = classify_folder_strict(&mod_root).expect("strict classification");
    let scan = scan_folder_strict(&mod_root).expect("strict folder scan");
    let reused =
        classify_folder_strict_from_scan(&scan).expect("classification from strict folder scan");

    assert_eq!(reused, expected);
    assert_eq!(scan.child_dirs, vec![mod_root.join("Assets")]);
}
