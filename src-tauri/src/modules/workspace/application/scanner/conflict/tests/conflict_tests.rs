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
        "[TextureOverrideBody]\nhash = abc12345\n",
    );
    let ini_b = create_ini(
        &mod_b,
        "config.ini",
        "[TextureOverrideBody]\nhash = abc12345\n",
    );

    let conflicts = detect_conflicts(&[(mod_a.clone(), ini_a), (mod_b.clone(), ini_b)]);

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].hash, "abc12345");
    assert_eq!(conflicts[0].mod_paths.len(), 2);
    assert_eq!(conflicts[0].kind, ConflictKind::ResourceHash);
    assert_eq!(conflicts[0].certainty, ConflictCertainty::Potential);
    assert_eq!(conflicts[0].evidence.len(), 2);
}

#[test]
fn records_override_evidence_and_excludes_disjoint_first_indices() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    fs::create_dir(&mod_a).unwrap();
    fs::create_dir(&mod_b).unwrap();

    let ini_a = create_ini(
        &mod_a,
        "a.ini",
        "namespace = Alice\n[TextureOverrideBody]\nhash = abcdef12\ncondition = $active\nmatch_priority = 7\nmatch_first_index = 0\n",
    );
    let ini_b = create_ini(
        &mod_b,
        "b.ini",
        "[TextureOverrideBody]\nhash = abcdef12\nmatch_first_index = 1\n",
    );

    assert!(detect_conflicts(&[(mod_a.clone(), ini_a.clone()), (mod_b.clone(), ini_b)]).is_empty());

    fs::write(
        &ini_a,
        "namespace = Alice\n[TextureOverrideBody]\nhash = abcdef12\ncondition = $active\nmatch_priority = 7\nmatch_first_index = 1\n",
    )
    .unwrap();
    let conflicts = detect_conflicts(&[(mod_a, ini_a), (mod_b, dir.path().join("ModB/b.ini"))]);
    let evidence = &conflicts[0].evidence[0];
    assert_eq!(evidence.namespace.as_deref(), Some("Alice"));
    assert_eq!(evidence.condition.as_deref(), Some("$active"));
    assert_eq!(evidence.priority, Some(7));
    assert_eq!(evidence.match_first_index, Some(1));
    assert!(conflicts[0].has_conditional_evidence);
    assert_eq!(conflicts[0].certainty, ConflictCertainty::Potential);
}

#[test]
fn detects_same_stage_shaderfixes_replacements() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    fs::create_dir_all(mod_a.join("ShaderFixes")).unwrap();
    fs::create_dir_all(mod_b.join("ShaderFixes")).unwrap();
    let filename = "0123456789abcdef-ps_replace.txt";
    fs::write(mod_a.join("ShaderFixes").join(filename), "shader a").unwrap();
    fs::write(mod_b.join("ShaderFixes").join(filename), "shader b").unwrap();
    let ini_a = create_ini(&mod_a, "a.ini", "[Constants]");
    let ini_b = create_ini(&mod_b, "b.ini", "[Constants]");

    let conflicts = detect_conflicts(&[(mod_a, ini_a), (mod_b, ini_b)]);

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].kind, ConflictKind::ShaderReplacement);
    assert_eq!(conflicts[0].evidence[0].shader_stage.as_deref(), Some("ps"));
}

#[test]
fn unindexed_evidence_overlaps_indexed_evidence_from_every_other_mod() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    let mod_c = dir.path().join("ModC");
    for mod_root in [&mod_a, &mod_b, &mod_c] {
        fs::create_dir(mod_root).unwrap();
    }

    let ini_a = create_ini(
        &mod_a,
        "a.ini",
        "[TextureOverrideBody]\nhash = abcdef12\nmatch_first_index = 0\n",
    );
    let ini_b = create_ini(
        &mod_b,
        "b.ini",
        "[TextureOverrideBody]\nhash = abcdef12\nmatch_first_index = 1\n",
    );
    let ini_c = create_ini(&mod_c, "c.ini", "[TextureOverrideBody]\nhash = abcdef12\n");

    let conflicts = detect_conflicts(&[(mod_a, ini_a), (mod_b, ini_b), (mod_c, ini_c)]);

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].mod_paths.len(), 3);
    assert_eq!(conflicts[0].evidence.len(), 3);
}

#[test]
fn combined_runtime_scan_detects_ini_and_shader_conflicts_once_per_root() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    for mod_root in [&mod_a, &mod_b] {
        fs::create_dir_all(mod_root.join("ShaderFixes")).unwrap();
        fs::create_dir_all(mod_root.join("DISABLED ignored")).unwrap();
        create_ini(
            mod_root,
            "config.ini",
            "[TextureOverrideBody]\nhash = abcdef12\n",
        );
        create_ini(
            &mod_root.join("DISABLED ignored"),
            "ignored.ini",
            "[TextureOverrideBody]\nhash = deadbeef\n",
        );
        fs::write(
            mod_root.join("ShaderFixes/0123456789abcdef-ps_replace.txt"),
            "shader",
        )
        .unwrap();
    }

    let conflicts = detect_runtime_conflicts(&[mod_a, mod_b]);

    assert_eq!(conflicts.len(), 2);
    assert!(conflicts.iter().any(|conflict| {
        conflict.kind == ConflictKind::ResourceHash && conflict.hash == "abcdef12"
    }));
    assert!(conflicts.iter().any(|conflict| {
        conflict.kind == ConflictKind::ShaderReplacement && conflict.hash == "0123456789abcdef"
    }));
    assert!(conflicts.iter().all(|conflict| conflict.hash != "deadbeef"));
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
        "[TextureOverrideBody]\nhash = abc12345\n[TextureOverrideHead]\nhash = abc12345\n",
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
        "[TextureOverrideBody]\nhash = abc12345\n",
    );
    let ini_b = create_ini(
        &mod_b,
        "config.ini",
        "[TextureOverrideBody]\nhash = def45678\n",
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

    let ini_a = create_ini(&mod_a, "config.ini", "[Constants]\nhash = abc12345\n");
    let ini_b = create_ini(&mod_b, "config.ini", "[Constants]\nhash = abc12345\n");

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

#[test]
fn rejects_hash_lengths_that_do_not_match_the_override_namespace() {
    let dir = TempDir::new().unwrap();
    let mod_a = dir.path().join("ModA");
    let mod_b = dir.path().join("ModB");
    fs::create_dir(&mod_a).unwrap();
    fs::create_dir(&mod_b).unwrap();

    let content = concat!(
        "[TextureOverrideInvalid]\nhash = abc123\n",
        "[ShaderOverrideInvalid]\nhash = abcdef12\n"
    );
    let ini_a = create_ini(&mod_a, "a.ini", content);
    let ini_b = create_ini(&mod_b, "b.ini", content);

    assert!(
        detect_conflicts(&[(mod_a, ini_a), (mod_b, ini_b)]).is_empty(),
        "TextureOverride requires 8 hex digits and ShaderOverride requires 16"
    );
}
