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

    let conflicts = detect_conflicts(&[(mod_a.clone(), ini_a), (mod_b.clone(), ini_b)]);

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

    let conflicts = detect_conflicts(&[(mod_dir.clone(), ini)]);

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

    let conflicts = detect_conflicts(&[(mod_a.clone(), ini_a), (mod_b.clone(), ini_b)]);
    assert!(conflicts.is_empty());
}

// Covers: EC-2.05 — Zero-byte INI
#[test]
fn test_empty_ini_file() {
    let dir = TempDir::new().unwrap();
    let mod_dir = dir.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();
    let ini = create_ini(&mod_dir, "empty.ini", "");

    let conflicts = detect_conflicts(&[(mod_dir.clone(), ini)]);
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

    let conflicts = detect_conflicts(&[(mod_a.clone(), ini_a), (mod_b.clone(), ini_b)]);
    // Should be empty because [Constants] is not [TextureOverride...]
    assert!(conflicts.is_empty());
}

// TC-05/TC-43: Duplicate hash — 3 mods share the same hash → all 3 paths merged into ONE ConflictInfo
#[test]
fn test_duplicate_hash_merges_all_mod_paths() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    let mod_c = dir.path().join("ModC");
    fs::create_dir_all(&mod_a).unwrap();
    fs::create_dir_all(&mod_b).unwrap();
    fs::create_dir_all(&mod_c).unwrap();

    let ini_a = create_ini(
        &mod_a,
        "config.ini",
        "[TextureOverrideBody]\nhash = deadbeef\n",
    );
    let ini_b = create_ini(
        &mod_b,
        "config.ini",
        "[TextureOverrideBody]\nhash = deadbeef\n",
    );
    let ini_c = create_ini(
        &mod_c,
        "config.ini",
        "[TextureOverrideBody]\nhash = deadbeef\n",
    );

    let conflicts = detect_conflicts(&[
        (mod_a.clone(), ini_a),
        (mod_b.clone(), ini_b),
        (mod_c.clone(), ini_c),
    ]);

    // Must produce exactly ONE ConflictInfo for the shared hash
    assert_eq!(
        conflicts.len(),
        1,
        "Expected 1 conflict info for hash 'deadbeef'"
    );
    // All 3 distinct mod roots must appear in mod_paths
    assert_eq!(
        conflicts[0].mod_paths.len(),
        3,
        "All 3 mods should be listed in mod_paths"
    );
    assert_eq!(conflicts[0].hash, "deadbeef");
}

// TC-05: Ambiguous hash — same hash referenced in 2 different sections within ONE mod → NOT a conflict
#[test]
fn test_ambiguous_hash_within_same_mod_is_not_a_conflict() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    fs::create_dir_all(&mod_a).unwrap();

    // Two TextureOverride sections inside the SAME mod both use hash = cafebabe
    let ini = create_ini(
        &mod_a,
        "config.ini",
        "[TextureOverrideBody]\nhash = cafebabe\n[TextureOverrideHead]\nhash = cafebabe\n",
    );

    let conflicts = detect_conflicts(&[(mod_a.clone(), ini)]);

    // Same mod duplicating a hash is NOT a cross-mod conflict
    assert!(
        conflicts.is_empty(),
        "Same-mod hash reuse must not raise a conflict"
    );
}

// TC-05: Multiple distinct hashes conflicting between two mods should each produce their own ConflictInfo
#[test]
fn test_multiple_conflicting_hashes_each_produce_a_conflict() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    fs::create_dir_all(&mod_a).unwrap();
    fs::create_dir_all(&mod_b).unwrap();

    // ModA and ModB share 2 different hashes
    let ini_a = create_ini(
        &mod_a,
        "config.ini",
        "[TextureOverrideBody]\nhash = aaaa1111\n[TextureOverrideHead]\nhash = bbbb2222\n",
    );
    let ini_b = create_ini(
        &mod_b,
        "config.ini",
        "[TextureOverrideBody]\nhash = aaaa1111\n[TextureOverrideHead]\nhash = bbbb2222\n",
    );

    let conflicts = detect_conflicts(&[(mod_a.clone(), ini_a), (mod_b.clone(), ini_b)]);

    assert_eq!(
        conflicts.len(),
        2,
        "Each shared hash should produce its own ConflictInfo"
    );
    // Both conflicts must involve mod_a and mod_b
    for c in &conflicts {
        assert_eq!(c.mod_paths.len(), 2);
    }
}
