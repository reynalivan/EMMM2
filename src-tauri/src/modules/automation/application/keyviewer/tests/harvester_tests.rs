//! Unit tests for the hash harvester.

use std::fs;
use std::io::Write;
use std::sync::Arc;
use tempfile::TempDir;

use crate::modules::automation::application::keyviewer::harvester::{
    cached_entry_count_for_root, harvest_hashes_from_ini, harvest_hashes_from_mod, harvest_mod,
    harvest_targets_from_ini, retain_cached_mods, HarvestCapabilities,
};
use crate::modules::matching::application::deep_matcher::models::types::RuntimeResourceKind;

/// Helper to create a temp INI file with given content.
fn write_ini(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut f = fs::File::create(&path).expect("create temp INI");
    f.write_all(content.as_bytes()).expect("write temp INI");
    path
}

#[test]
fn retaining_active_mods_prunes_stale_capability_snapshots() {
    let root = TempDir::new().unwrap();
    let mod_path = root.path().join("Mod");
    fs::create_dir(&mod_path).unwrap();
    fs::write(
        mod_path.join("mod.ini"),
        "[TextureOverrideBody]\nhash = df65bb00\nvb0 = ResourceBody\nib = ResourceIndex\n",
    )
    .unwrap();
    let vb0 = HarvestCapabilities::from_callback_slots(["vb0"]);
    let ib = HarvestCapabilities::from_callback_slots(["ib"]);

    harvest_mod(&mod_path, &vb0).unwrap();
    harvest_mod(&mod_path, &ib).unwrap();
    assert_eq!(cached_entry_count_for_root(root.path()), 2);

    retain_cached_mods(root.path(), &ib, [mod_path.as_path()]);
    assert_eq!(cached_entry_count_for_root(root.path()), 1);
}

#[test]
fn cached_harvest_reuses_the_same_allocation() {
    let dir = TempDir::new().unwrap();
    write_ini(
        &dir,
        "position.ini",
        "[TextureOverridePosition]\nhash = 6895f405\nvb0 = ResourcePosition\n",
    );
    let capabilities = HarvestCapabilities::from_callback_slots(["vb0"]);

    let first = harvest_mod(dir.path(), &capabilities).unwrap();
    let second = harvest_mod(dir.path(), &capabilities).unwrap();

    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn extracts_hash_from_texture_override() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideAlbedoBody]
hash = df65bb00
handling = skip
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "df65bb00");
    assert_eq!(result[0].section_name, "TextureOverrideAlbedoBody");
}

#[test]
fn ignores_shader_override_hashes_even_when_they_look_32_bit() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[ShaderOverrideAlbedoVS]
hash = AABBCCDD
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert!(result.is_empty());
}

#[test]
fn skips_non_override_sections() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[Constants]
hash = 12345678

[ResourceAlbedo]
hash = 87654321

[TextureOverrideAlbedo]
hash = df65bb00
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "df65bb00");
}

#[test]
fn skips_denylisted_sections() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideNotificationBar]
hash = 11111111

[TextureOverrideUIElement]
hash = 22222222

[TextureOverrideCursorHand]
hash = 33333333

[ShaderOverrideUISomething]
hash = 44444444

[ShaderOverrideShadowMap]
hash = 55555555

[TextureOverrideAlbedo]
hash = df65bb00
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "df65bb00");
}

#[test]
fn extracts_multiple_hashes_in_one_file() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideAlbedoHead]
hash = aabbcc11

[TextureOverrideAlbedoBody]
hash = aabbcc22

[TextureOverrideAlbedoDress]
hash = aabbcc33
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 3);
    let hashes: Vec<&str> = result.iter().map(|h| h.hash.as_str()).collect();
    assert!(hashes.contains(&"aabbcc11"));
    assert!(hashes.contains(&"aabbcc22"));
    assert!(hashes.contains(&"aabbcc33"));
}

#[test]
fn handles_inline_comments() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideFoo]
hash = df65bb00 ; position buffer
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "df65bb00");
}

#[test]
fn handles_bom_utf8() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bom.ini");
    let mut f = fs::File::create(&path).unwrap();
    // Write UTF-8 BOM + content
    f.write_all(&[0xEF, 0xBB, 0xBF]).unwrap();
    f.write_all(b"[TextureOverrideFoo]\nhash = abcd1234\n")
        .unwrap();

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "abcd1234");
}

#[test]
fn ignores_invalid_hash_lengths() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[TextureOverrideFoo]
hash = abc
hash = abcdef1234
hash = 12345678
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "12345678");
}

#[test]
fn harvest_from_mod_aggregates_across_files() {
    let dir = TempDir::new().unwrap();
    write_ini(
        &dir,
        "merged.ini",
        r#"
[TextureOverrideBody]
hash = 11112222
"#,
    );
    write_ini(
        &dir,
        "extra.ini",
        r#"
[TextureOverrideHead]
hash = 33334444

[TextureOverrideExtra]
hash = 11112222
"#,
    );

    let result = harvest_hashes_from_mod(dir.path()).unwrap();
    // "11112222" appears twice (in two files) → grouped
    assert_eq!(result.get("11112222").map(|v| v.len()), Some(2));
    // "33334444" appears once
    assert_eq!(result.get("33334444").map(|v| v.len()), Some(1));
}

#[test]
fn harvest_from_empty_mod_returns_empty() {
    let dir = TempDir::new().unwrap();
    let result = harvest_hashes_from_mod(dir.path()).unwrap();
    assert!(result.is_empty());
}

#[test]
fn handles_case_insensitive_section_names() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "test.ini",
        r#"
[textureoverridealbedobody]
hash = df65bb00

[TEXTUREOVERRIDEALBEDOVS]
hash = aabbccdd
"#,
    );

    let result = harvest_hashes_from_ini(&path).unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn harvests_slot_and_draw_context_instead_of_only_the_raw_hash() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "geometry.ini",
        r#"
[TextureOverrideArlecchinoPosition]
hash = 6895f405
vb0 = ResourceArlecchinoPosition

[TextureOverrideArlecchinoBody]
hash = e811d2a1
ib = ResourceArlecchinoIB
match_first_index = 40179

[TextureOverrideArlecchinoFaceDiffuse]
hash = a44625da
ps-t0 = ResourceArlecchinoFaceDiffuse
"#,
    );

    let targets = harvest_targets_from_ini(
        &path,
        &HarvestCapabilities::from_callback_slots(["vb0", "ib", "ps-t0"]),
    )
    .unwrap();

    assert_eq!(targets.len(), 3);
    assert_eq!(targets[0].resource_kind, RuntimeResourceKind::PositionVb);
    assert_eq!(targets[0].callback_slot, "vb0");
    assert_eq!(targets[1].resource_kind, RuntimeResourceKind::IndexBuffer);
    assert_eq!(targets[1].match_first_index, Some(40179));
    assert_eq!(targets[2].resource_kind, RuntimeResourceKind::Texture);
    assert_eq!(targets[2].callback_slot, "ps-t0");
}

#[test]
fn accepts_position_section_as_a_vb0_fallback_only_when_the_profile_supports_it() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "legacy.ini",
        r#"
[TextureOverrideArlecchinoPosition]
hash = 6895f405
"#,
    );

    let supported =
        harvest_targets_from_ini(&path, &HarvestCapabilities::from_callback_slots(["vb0"]))
            .unwrap();
    assert_eq!(supported.len(), 1);
    assert_eq!(supported[0].resource_kind, RuntimeResourceKind::PositionVb);
    assert_eq!(supported[0].callback_slot, "vb0");

    let unsupported = harvest_targets_from_ini(&path, &HarvestCapabilities::default()).unwrap();
    assert!(unsupported.is_empty());
}

#[test]
fn ignores_resource_hashes_without_a_slot_or_supported_position_fallback() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "malformed.ini",
        r#"
[TextureOverrideAccessory]
hash = aabbccdd
vbzero = ResourceBroken
ps-t = ResourceAlsoBroken
"#,
    );

    let targets =
        harvest_targets_from_ini(&path, &HarvestCapabilities::from_callback_slots(["vb0"]))
            .unwrap();
    assert!(targets.is_empty());
}

#[test]
fn records_every_effective_ini_in_the_generation_fingerprint() {
    let dir = TempDir::new().unwrap();
    write_ini(
        &dir,
        "position.ini",
        "[TextureOverrideArlecchinoPosition]\nhash = 6895f405\nvb0 = ResourcePosition\n",
    );
    write_ini(&dir, "keys.ini", "[KeyArlecchino]\nkey = CTRL+[\n");

    let harvest = harvest_mod(
        dir.path(),
        &HarvestCapabilities::from_callback_slots(["vb0"]),
    )
    .expect("mod harvest should succeed");

    assert_eq!(harvest.ini_fingerprints.len(), 2);
    assert!(harvest
        .ini_fingerprints
        .iter()
        .all(|value| value.len() == 64));
}

#[test]
fn cached_harvest_reparses_equal_length_ini_when_timestamp_is_preserved() {
    let dir = TempDir::new().unwrap();
    let path = write_ini(
        &dir,
        "position.ini",
        "[TextureOverrideArlecchinoPosition]\nhash = 6895f405\nvb0 = ResourcePosition\n",
    );
    let capabilities = HarvestCapabilities::from_callback_slots(["vb0"]);
    let original_modified = std::fs::metadata(&path).unwrap().modified().unwrap();

    let first = harvest_mod(dir.path(), &capabilities).unwrap();
    assert_eq!(first.targets[0].hash, "6895f405");

    std::fs::write(
        &path,
        "[TextureOverrideArlecchinoPosition]\nhash = a1b2c3d4\nvb0 = ResourcePosition\n",
    )
    .unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(original_modified))
        .unwrap();

    let second = harvest_mod(dir.path(), &capabilities).unwrap();
    assert_eq!(second.targets[0].hash, "a1b2c3d4");
    assert_ne!(first.ini_fingerprints, second.ini_fingerprints);
}
