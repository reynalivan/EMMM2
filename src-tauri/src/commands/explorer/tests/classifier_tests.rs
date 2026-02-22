use crate::commands::explorer::classifier::{classify_folder, NodeType};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_empty_folder_is_container() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("EmptyMod");
    fs::create_dir(&dir).unwrap();

    let (node_type, _reasons) = classify_folder(&dir);
    assert_eq!(node_type, NodeType::ContainerFolder);
}

#[test]
fn test_subfolders_only_is_container() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("ContainerMod");
    fs::create_dir_all(dir.join("SubA")).unwrap();
    fs::create_dir_all(dir.join("SubB")).unwrap();

    let (node_type, _reasons) = classify_folder(&dir);
    assert_eq!(node_type, NodeType::ContainerFolder);
}

#[test]
fn test_mod_ini_with_assets_is_modpack() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("RaidenShogun");
    fs::create_dir(&dir).unwrap();

    // Create valid mod ini
    fs::write(
        dir.join("raiden.ini"),
        "[TextureOverrideRaidenBody]\nhash = d94c8962\nhandling = skip\n",
    )
    .unwrap();
    // Create mod asset
    fs::write(dir.join("RaidenBody.buf"), "fake buffer data").unwrap();

    let (node_type, reasons) = classify_folder(&dir);
    assert_eq!(node_type, NodeType::ModPackRoot);
    assert!(reasons.iter().any(|r| r.contains("Mod ini")));
    assert!(reasons.iter().any(|r| r.contains("mod asset")));
}

#[test]
fn test_mod_ini_without_assets_is_modpack() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("MinimalMod");
    fs::create_dir(&dir).unwrap();

    fs::write(dir.join("mod.ini"), "[ShaderOverrideBody]\nhash = abc123\n").unwrap();

    let (node_type, reasons) = classify_folder(&dir);
    assert_eq!(node_type, NodeType::ModPackRoot);
    assert!(reasons.iter().any(|r| r.contains("Mod ini")));
}

#[test]
fn test_non_mod_ini_is_container() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("ConfigFolder");
    fs::create_dir(&dir).unwrap();

    // INI without mod sections (AC-FG6: false positive protection)
    fs::write(
        dir.join("settings.ini"),
        "[Settings]\nresolution = 1920x1080\n[Constants]\nx = 5\n",
    )
    .unwrap();

    let (node_type, _reasons) = classify_folder(&dir);
    assert_eq!(node_type, NodeType::ContainerFolder);
}

#[test]
fn test_desktop_ini_ignored() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("DesktopIniFolder");
    fs::create_dir(&dir).unwrap();

    // desktop.ini should be skipped completely
    fs::write(
        dir.join("desktop.ini"),
        "[.ShellClassInfo]\nIconResource=C:\\icon.dll,0\n",
    )
    .unwrap();

    let (node_type, _reasons) = classify_folder(&dir);
    assert_eq!(node_type, NodeType::ContainerFolder);
}

#[test]
fn test_variant_container_multiple_child_dirs() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("VariantMod");
    fs::create_dir(&dir).unwrap();

    // Parent has orchestrator ini
    fs::write(
        dir.join("orchestrator.ini"),
        "[TextureOverrideMain]\nhash = abc\n",
    )
    .unwrap();

    // Create 3+ child dirs each with their own mod ini
    for i in 0..3 {
        let child = dir.join(format!("Variant{i}"));
        fs::create_dir(&child).unwrap();
        fs::write(
            child.join("mod.ini"),
            format!("[TextureOverrideV{i}]\nhash = hash{i}\n"),
        )
        .unwrap();
    }

    let (node_type, reasons) = classify_folder(&dir);
    assert_eq!(node_type, NodeType::VariantContainer);
    assert!(reasons
        .iter()
        .any(|r| r.contains("child dirs with mod ini")));
}

#[test]
fn test_resource_section_with_filename_is_modpack() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("ResourceMod");
    fs::create_dir(&dir).unwrap();

    fs::write(
        dir.join("mod.ini"),
        "[ResourceBodyDiffuse]\nfilename = Textures/body_diffuse.dds\n",
    )
    .unwrap();

    let (node_type, reasons) = classify_folder(&dir);
    assert_eq!(node_type, NodeType::ModPackRoot);
    assert!(reasons.iter().any(|r| r.contains("Mod ini")));
    assert!(reasons.iter().any(|r| r.contains("Textures")));
}
