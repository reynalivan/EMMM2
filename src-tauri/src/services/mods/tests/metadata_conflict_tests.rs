//! Conflict detection reads paths the way the database actually stores them.
//!
//! `mods.folder_path` is written relative to the mods root by disk reconcile,
//! which runs on startup and after every filesystem change. A reader that
//! treats the column as absolute finds nothing and reports no conflicts --
//! silently, because "no conflicts" is a perfectly ordinary answer.

use super::conflicts_for_enabled_paths;
use crate::domain::mod_path::ModFolderPath;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Two enabled mods overriding the same hash: a real conflict to find.
fn two_conflicting_mods(root: &Path) {
    for name in ["ModA", "ModB"] {
        let dir = root.join(name);
        fs::create_dir_all(&dir).expect("mod dir");
        fs::write(
            dir.join("mod.ini"),
            "[TextureOverrideShared]\nhash = abcdef12\nrun = CommandListShared\n",
        )
        .expect("mod ini");
    }
}

#[test]
fn conflicts_are_found_when_paths_are_stored_relative() {
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path();
    two_conflicting_mods(mods_root);

    // Exactly what disk reconcile writes: a path relative to the mods root.
    let stored = vec![
        ModFolderPath::from_stored("ModA"),
        ModFolderPath::from_stored("ModB"),
    ];

    let conflicts = conflicts_for_enabled_paths(mods_root, &stored);

    assert!(
        !conflicts.is_empty(),
        "relative folder_path values must still resolve to the mod folder"
    );
}

#[test]
fn conflicts_are_found_when_paths_are_stored_absolute() {
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path();
    two_conflicting_mods(mods_root);

    // What the scanner commit writes today. Both conventions are in the table,
    // so the reader has to cope with either.
    let stored = vec![
        ModFolderPath::from_stored(mods_root.join("ModA").to_string_lossy().to_string()),
        ModFolderPath::from_stored(mods_root.join("ModB").to_string_lossy().to_string()),
    ];

    let conflicts = conflicts_for_enabled_paths(mods_root, &stored);

    assert!(
        !conflicts.is_empty(),
        "absolute folder_path values must keep working"
    );
}

#[test]
fn a_missing_folder_is_skipped_rather_than_guessed_at() {
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path();
    two_conflicting_mods(mods_root);

    let stored = vec![
        ModFolderPath::from_stored("ModA"),
        ModFolderPath::from_stored("Deleted On Disk"),
        ModFolderPath::from_stored("ModB"),
    ];

    // The stale row contributes nothing; the two real mods still conflict.
    let conflicts = conflicts_for_enabled_paths(mods_root, &stored);
    assert!(!conflicts.is_empty());
}

#[test]
fn a_nested_ini_still_counts_as_a_conflict() {
    // Deliberate asymmetry: the conflict scan recurses, the KeyViewer harvest
    // reads top-level INIs only. 3DMigoto loads nested INIs, so they can
    // genuinely collide -- but the overlay stays conservative about which
    // keybinds it claims are live. Pinned so the two do not get "unified" by
    // someone who reads it as an oversight.
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path();

    let top = mods_root.join("ModA");
    fs::create_dir_all(&top).expect("mod dir");
    fs::write(
        top.join("mod.ini"),
        "[TextureOverrideShared]\nhash = abcdef12\n",
    )
    .expect("mod ini");

    let nested = mods_root.join("ModB").join("variants").join("blue");
    fs::create_dir_all(&nested).expect("nested dir");
    fs::write(
        nested.join("blue.ini"),
        "[TextureOverrideShared]\nhash = abcdef12\n",
    )
    .expect("nested ini");

    let stored = vec![
        ModFolderPath::from_stored("ModA"),
        ModFolderPath::from_stored("ModB"),
    ];

    assert!(
        !conflicts_for_enabled_paths(mods_root, &stored).is_empty(),
        "an INI nested inside an enabled mod still collides"
    );
}

#[test]
fn unrelated_mods_do_not_conflict() {
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path();
    for (name, hash) in [("ModA", "aaaa1111"), ("ModB", "bbbb2222")] {
        let dir = mods_root.join(name);
        fs::create_dir_all(&dir).expect("mod dir");
        fs::write(
            dir.join("mod.ini"),
            format!("[TextureOverride{name}]\nhash = {hash}\n"),
        )
        .expect("mod ini");
    }

    let stored = vec![
        ModFolderPath::from_stored("ModA"),
        ModFolderPath::from_stored("ModB"),
    ];

    assert!(
        conflicts_for_enabled_paths(mods_root, &stored).is_empty(),
        "different hashes are not a conflict"
    );
}

#[test]
fn shaderfixes_only_mods_are_included() {
    let temp = TempDir::new().expect("tempdir");
    let mods_root = temp.path();
    let filename = "0123456789abcdef-ps_replace.txt";
    for name in ["ModA", "ModB"] {
        let shader_fixes = mods_root.join(name).join("ShaderFixes");
        fs::create_dir_all(&shader_fixes).expect("shader fixes");
        fs::write(shader_fixes.join(filename), name).expect("replacement");
    }
    let stored = vec![
        ModFolderPath::from_stored("ModA"),
        ModFolderPath::from_stored("ModB"),
    ];

    let conflicts = conflicts_for_enabled_paths(mods_root, &stored);

    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].kind,
        crate::services::scanner::conflict::ConflictKind::ShaderReplacement
    );
}
