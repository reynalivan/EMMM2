use super::*;
use std::fs;
use tempfile::TempDir;

fn create_ini(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

// Covers: TC-2.4-01 — Shader conflict detection
#[test]
fn test_detect_conflict() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    fs::create_dir(&mod_a).unwrap();
    fs::create_dir(&mod_b).unwrap();

    let ini_a = create_ini(
        &mod_a,
        "config.ini",
        "[TextureOverrideBody]\nhash = abc123\n",
    );
    let ini_b = create_ini(
        &mod_b,
        "config.ini",
        "[TextureOverrideBody]\nhash = abc123\n",
    );

    let conflicts = detect_conflicts(&[ini_a, ini_b]);

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].hash, "abc123");
    assert_eq!(conflicts[0].mod_paths.len(), 2);
}

// No conflict when same hash is in same mod
#[test]
fn test_no_conflict_same_mod() {
    let dir = TempDir::new().unwrap();
    let mod_dir = dir.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();

    let ini = create_ini(
        &mod_dir,
        "config.ini",
        "[TextureOverrideBody]\nhash = abc123\n[TextureOverrideHead]\nhash = abc123\n",
    );

    let conflicts = detect_conflicts(&[ini]);

    assert!(conflicts.is_empty());
}

// No conflict when hashes differ
#[test]
fn test_no_conflict_different_hashes() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    fs::create_dir(&mod_a).unwrap();
    fs::create_dir(&mod_b).unwrap();

    let ini_a = create_ini(
        &mod_a,
        "config.ini",
        "[TextureOverrideBody]\nhash = abc123\n",
    );
    let ini_b = create_ini(
        &mod_b,
        "config.ini",
        "[TextureOverrideBody]\nhash = def456\n",
    );

    let conflicts = detect_conflicts(&[ini_a, ini_b]);
    assert!(conflicts.is_empty());
}

// Covers: EC-2.05 — Zero-byte INI
#[test]
fn test_empty_ini_file() {
    let dir = TempDir::new().unwrap();
    let ini = create_ini(dir.path(), "empty.ini", "");

    let conflicts = detect_conflicts(&[ini]);
    assert!(conflicts.is_empty());
}

#[test]
fn test_non_texture_override_section_ignored() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    fs::create_dir(&mod_a).unwrap();
    fs::create_dir(&mod_b).unwrap();

    let ini_a = create_ini(&mod_a, "config.ini", "[Constants]\nhash = abc123\n");
    let ini_b = create_ini(&mod_b, "config.ini", "[Constants]\nhash = abc123\n");

    let conflicts = detect_conflicts(&[ini_a, ini_b]);
    // Should be empty because [Constants] is not [TextureOverride...]
    assert!(conflicts.is_empty());
}
