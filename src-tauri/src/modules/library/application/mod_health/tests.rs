use super::service::{analyze_mod_health, create_mod_viewer_launch_receipt};
use super::types::{ModControlKind, ModHealthSeverity, ModHealthSupportLevel};
use crate::modules::games::domain::models::GameType;
use std::fs;

fn relative_paths(entries: &[super::types::ModAssetEntry]) -> Vec<&str> {
    entries
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect()
}

#[test]
fn analyzes_active_and_inactive_assets_without_marking_metadata_or_backups_as_orphaned() {
    let folder = tempfile::tempdir().expect("mod fixture");
    fs::write(
        folder.path().join("mod.ini"),
        "[ResourceBody]\nfilename = body.dds\n",
    )
    .expect("active ini");
    fs::write(
        folder.path().join("DISABLED_alt.ini"),
        "[ResourceAlt]\nfilename = alt.dds\n",
    )
    .expect("inactive ini");
    for file in [
        "body.dds",
        "alt.dds",
        "orphan.buf",
        "preview.png",
        "info.json",
        ".mod_viewer.json",
        "mod.ini.bak",
    ] {
        fs::write(folder.path().join(file), "fixture").expect("asset fixture");
    }

    let report = analyze_mod_health(folder.path(), GameType::GIMI).expect("health report");

    assert_eq!(relative_paths(&report.manifest.referenced), ["body.dds"]);
    assert_eq!(relative_paths(&report.manifest.inactive_only), ["alt.dds"]);
    assert_eq!(relative_paths(&report.manifest.orphan), ["orphan.buf"]);
    assert_eq!(report.manifest.counts.referenced, 1);
    assert_eq!(report.manifest.counts.inactive_only, 1);
    assert_eq!(report.manifest.counts.orphan, 1);
    assert!(report
        .file_manifest
        .iter()
        .any(|entry| entry.relative_path == "body.dds" && !entry.blake3.is_empty()));
}

#[test]
fn reports_ini_structure_and_resource_problems_without_stopping_analysis() {
    let folder = tempfile::tempdir().expect("mod fixture");
    fs::write(
        folder.path().join("broken.ini"),
        "[ResourceMissing]\nfilename = missing.dds\n\n[CommandList]\nif $enabled == 1\nelse\nelse\nendif\nendif\nref ResourceUndeclared\n[Another]\nif $unclosed == 1\n[Malformed\n",
    )
    .expect("broken ini");
    fs::write(
        folder.path().join("unsafe.ini"),
        "[ResourceUnsafe]\nfilename = ../outside.dds\n",
    )
    .expect("unsafe ini");

    let report = analyze_mod_health(folder.path(), GameType::GIMI).expect("health report");
    let codes = report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<Vec<_>>();

    for expected in [
        "ini_raw_fallback",
        "condition_order",
        "condition_unbalanced",
        "resource_missing",
        "resource_undeclared",
        "resource_path_unsafe",
    ] {
        assert!(codes.contains(&expected), "missing {expected}: {codes:?}");
    }
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.severity == ModHealthSeverity::Error && issue.line.is_some()));
}

#[test]
fn extracts_supported_cycle_controls_and_marks_srmi_experimental_without_parsing_efmi_controls() {
    let folder = tempfile::tempdir().expect("mod fixture");
    fs::write(
        folder.path().join("controls.ini"),
        "[KeyWardrobeMenu]\nkey = F1\nback = F2\ntype = cycle\n$shape = 0, 1, 2\ndefault = 1\n\n[Present]\nrun = CommandListUpdate\n\n[PresentCustom]\ncustom = 1\n",
    )
    .expect("controls ini");

    let supported = analyze_mod_health(folder.path(), GameType::GIMI).expect("supported report");
    assert_eq!(supported.support_level, ModHealthSupportLevel::Supported);
    assert!(supported.controls.iter().any(|control| {
        control.kind == ModControlKind::MenuToggle
            && control.key.as_deref() == Some("F1")
            && control.variable.as_deref() == Some("$shape")
            && control.values == ["0", "1", "2"]
            && control.default_value.as_deref() == Some("1")
    }));
    assert!(supported
        .controls
        .iter()
        .any(|control| control.kind == ModControlKind::Present));
    assert!(!supported
        .controls
        .iter()
        .any(|control| control.section == "PresentCustom"));
    assert!(supported.controls.iter().any(|control| {
        control.kind == ModControlKind::ShapeVariable
            && control.variable.as_deref() == Some("$shape")
    }));

    let experimental = analyze_mod_health(folder.path(), GameType::SRMI).expect("srmi report");
    assert_eq!(
        experimental.support_level,
        ModHealthSupportLevel::Experimental
    );
    assert!(!experimental.controls.is_empty());

    let basic = analyze_mod_health(folder.path(), GameType::EFMI).expect("efmi report");
    assert_eq!(basic.support_level, ModHealthSupportLevel::Basic);
    assert!(basic.controls.is_empty());
}

#[test]
fn treats_a_disabled_selected_mod_folder_as_inactive() {
    let parent = tempfile::tempdir().expect("mod fixture");
    let folder = parent.path().join("DISABLED Outfit");
    fs::create_dir(&folder).expect("disabled mod folder");
    fs::write(
        folder.join("mod.ini"),
        "[ResourceBody]\nfilename = body.dds\n\n[KeyOutfit]\ntype = cycle\nkey = F1\n$outfit = 0, 1\n",
    )
    .expect("inactive ini");
    fs::write(folder.join("body.dds"), "fixture").expect("asset fixture");

    let report = analyze_mod_health(&folder, GameType::GIMI).expect("health report");

    assert_eq!(
        relative_paths(&report.manifest.referenced),
        Vec::<&str>::new()
    );
    assert_eq!(relative_paths(&report.manifest.inactive_only), ["body.dds"]);
    assert!(report.controls.is_empty());
}

#[test]
fn reports_oversized_ini_without_reading_its_contents() {
    use crate::modules::library::application::ini::document::MAX_PARSEABLE_INI_BYTES;

    let folder = tempfile::tempdir().expect("mod fixture");
    let file = fs::File::create(folder.path().join("huge.ini")).expect("large ini fixture");
    file.set_len(MAX_PARSEABLE_INI_BYTES + 1)
        .expect("make ini exceed analysis cap");

    let report = analyze_mod_health(folder.path(), GameType::GIMI).expect("health report");

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "ini_too_large"
                && issue.severity == ModHealthSeverity::Warning)
    );
}

#[test]
fn launch_receipt_uses_only_regular_non_symlink_files_and_preserves_requested_folder() {
    let folder = tempfile::tempdir().expect("mod fixture");
    fs::write(folder.path().join("asset with spaces.dds"), "texture").expect("asset fixture");
    fs::create_dir(folder.path().join("nested")).expect("nested directory");
    fs::write(folder.path().join("nested/mod.ini"), "[Constants]").expect("ini fixture");

    let receipt = create_mod_viewer_launch_receipt(
        "game-a".to_string(),
        "Original Folder Spelling".to_string(),
        folder.path(),
    )
    .expect("receipt");

    assert_eq!(receipt.game_id, "game-a");
    assert_eq!(receipt.mod_folder, "Original Folder Spelling");
    assert_eq!(
        receipt
            .file_manifest
            .iter()
            .map(|entry| entry.relative_path.as_str())
            .collect::<Vec<_>>(),
        ["asset with spaces.dds", "nested/mod.ini"]
    );
}
