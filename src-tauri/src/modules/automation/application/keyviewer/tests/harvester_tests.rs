//! Unit tests for typed KeyViewer harvesting and cache ownership.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

use crate::modules::automation::application::keyviewer::harvester::{
    cached_entry_count_for_root, harvest_mod, harvest_targets_from_ini, invalidate_cached_mods,
    retain_cached_mods, HarvestCapabilities, HarvestedTarget,
};
use crate::modules::matching::application::deep_matcher::models::types::RuntimeResourceKind;

/// Helper to create a temp INI file with given content.
fn write_ini(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut f = fs::File::create(&path).expect("create temp INI");
    f.write_all(content.as_bytes()).expect("write temp INI");
    path
}

fn position_targets(path: &Path) -> Vec<HarvestedTarget> {
    harvest_targets_from_ini(path, &HarvestCapabilities::from_callback_slots(["vb0"])).unwrap()
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
fn batch_invalidation_removes_all_variants_and_preserves_other_roots() {
    let changed_root = TempDir::new().unwrap();
    let other_root = TempDir::new().unwrap();
    let old_path = changed_root.path().join("OldMod");
    let new_path = changed_root.path().join("NewMod");
    let sibling_path = changed_root.path().join("NewModSibling");
    let other_path = other_root.path().join("Mod");
    let capabilities = [
        HarvestCapabilities::from_callback_slots(["vb0"]),
        HarvestCapabilities::from_callback_slots(["ib"]),
    ];
    for path in [&old_path, &new_path, &sibling_path, &other_path] {
        fs::create_dir(path).unwrap();
        fs::write(
            path.join("mod.ini"),
            "[TextureOverrideBody]\nhash = df65bb00\nvb0 = ResourceBody\nib = ResourceIndex\n",
        )
        .unwrap();
        for capability in &capabilities {
            harvest_mod(path, capability).unwrap();
        }
    }
    let sibling = harvest_mod(&sibling_path, &capabilities[0]).unwrap();
    let other = harvest_mod(&other_path, &capabilities[0]).unwrap();
    let lexical_alias = std::path::PathBuf::from(
        old_path
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_uppercase(),
    );

    invalidate_cached_mods([
        old_path.as_path(),
        lexical_alias.as_path(),
        new_path.as_path(),
        new_path.as_path(),
    ]);

    assert_eq!(cached_entry_count_for_root(changed_root.path()), 2);
    assert_eq!(cached_entry_count_for_root(other_root.path()), 2);
    assert!(Arc::ptr_eq(
        &sibling,
        &harvest_mod(&sibling_path, &capabilities[0]).unwrap()
    ));
    assert!(Arc::ptr_eq(
        &other,
        &harvest_mod(&other_path, &capabilities[0]).unwrap()
    ));
    invalidate_cached_mods(std::iter::empty::<&std::path::Path>());
    assert_eq!(cached_entry_count_for_root(changed_root.path()), 2);
    assert_eq!(cached_entry_count_for_root(other_root.path()), 2);
}

#[test]
fn cold_harvest_fingerprints_preserve_encoded_bytes_and_warm_validation() {
    use sha2::{Digest, Sha256};

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("mod.ini");
    let text =
        "[TextureOverrideBody]\nhash = df65bb00\nvb0 = ResourceBody\n[KeyBody]\nkey = CTRL+K\n";
    let bytes: Vec<u8> = [0xff, 0xfe]
        .into_iter()
        .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    fs::write(&path, &bytes).unwrap();
    let capabilities = HarvestCapabilities::from_callback_slots(["vb0"]);
    let first = harvest_mod(dir.path(), &capabilities).unwrap();
    assert_eq!(
        first.ini_fingerprints,
        [format!("{:x}", Sha256::digest(&bytes))]
    );
    assert_eq!(first.targets[0].hash, "df65bb00");
    assert_eq!(first.keybinds.len(), 1);
    assert_eq!(
        crate::modules::automation::application::keyviewer::harvester::harvest_digest_count(&path),
        1
    );
    let second = harvest_mod(dir.path(), &capabilities).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(
        crate::modules::automation::application::keyviewer::harvester::harvest_digest_count(&path),
        2
    );

    let replacement = text.replace("df65bb00", "df65bb01");
    let replacement_bytes: Vec<u8> = [0xff, 0xfe]
        .into_iter()
        .chain(replacement.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    fs::write(&path, &replacement_bytes).unwrap();
    let third = harvest_mod(dir.path(), &capabilities).unwrap();
    assert_eq!(third.targets[0].hash, "df65bb01");
    assert_eq!(third.keybinds.len(), 1);
    assert_eq!(
        crate::modules::automation::application::keyviewer::harvester::harvest_digest_count(&path),
        3
    );
    assert_eq!(
        third.ini_fingerprints,
        [format!("{:x}", Sha256::digest(&replacement_bytes))]
    );
}

#[test]
fn oversized_ini_retains_targets_and_fingerprint_without_editable_keybinds() {
    use sha2::{Digest, Sha256};

    let dir = TempDir::new().unwrap();
    let mut bytes =
        b"[TextureOverrideBody]\nhash = df65bb00\nvb0 = ResourceBody\n[KeyBody]\nkey = K\n;"
            .to_vec();
    bytes.resize(
        crate::modules::library::application::ini::document::MAX_PARSEABLE_INI_BYTES as usize + 1,
        b' ',
    );
    fs::write(dir.path().join("mod.ini"), &bytes).unwrap();
    let capabilities = HarvestCapabilities::from_callback_slots(["vb0"]);
    let first = harvest_mod(dir.path(), &capabilities).unwrap();
    assert_eq!(first.targets[0].hash, "df65bb00");
    assert!(first.keybinds.is_empty());
    assert_eq!(
        first.ini_fingerprints,
        [format!("{:x}", Sha256::digest(&bytes))]
    );
    assert!(Arc::ptr_eq(
        &first,
        &harvest_mod(dir.path(), &capabilities).unwrap()
    ));
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
vb0 = ResourceBody
handling = skip
"#,
    );

    let result = position_targets(&path);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].hash, "df65bb00");
    assert_eq!(result[0].section_name, "TextureOverrideAlbedoBody");
    assert_eq!(result[0].file_path, path);
    assert_eq!(result[0].resource_kind, RuntimeResourceKind::PositionVb);
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
vb0 = ResourceShader
"#,
    );

    let result = position_targets(&path);
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
vb0 = ResourceConstants

[ResourceAlbedo]
hash = 87654321
vb0 = ResourceAlbedo

[TextureOverrideAlbedo]
hash = df65bb00
vb0 = ResourceBody
"#,
    );

    let result = position_targets(&path);
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
vb0 = ResourceNotification

[TextureOverrideUIElement]
hash = 22222222
vb0 = ResourceUI

[TextureOverrideCursorHand]
hash = 33333333
vb0 = ResourceCursor

[ShaderOverrideUISomething]
hash = 44444444
vb0 = ResourceShaderUI

[ShaderOverrideShadowMap]
hash = 55555555
vb0 = ResourceShaderShadow

[TextureOverrideAlbedo]
hash = df65bb00
vb0 = ResourceBody
"#,
    );

    let result = position_targets(&path);
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
vb0 = ResourceHead

[TextureOverrideAlbedoBody]
hash = aabbcc22
vb0 = ResourceBody

[TextureOverrideAlbedoDress]
hash = aabbcc33
vb0 = ResourceDress
"#,
    );

    let result = position_targets(&path);
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
vb0 = ResourcePosition
"#,
    );

    let result = position_targets(&path);
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
    f.write_all(b"[TextureOverrideFoo]\nhash = abcd1234\nvb0 = ResourcePosition\n")
        .unwrap();

    let result = position_targets(&path);
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
vb0 = ResourcePosition
"#,
    );

    let result = position_targets(&path);
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
vb0 = ResourceBody
"#,
    );
    write_ini(
        &dir,
        "extra.ini",
        r#"
[TextureOverrideHead]
hash = 33334444
vb0 = ResourceHead

[TextureOverrideExtra]
hash = 11112222
vb0 = ResourceExtra
"#,
    );

    let capabilities = HarvestCapabilities::from_callback_slots(["vb0"]);
    let result = harvest_mod(dir.path(), &capabilities).unwrap();
    assert_eq!(result.targets.len(), 3);
    assert_eq!(
        result
            .targets
            .iter()
            .filter(|target| target.hash == "11112222")
            .count(),
        2
    );
    assert_eq!(
        result
            .targets
            .iter()
            .filter(|target| target.hash == "33334444")
            .count(),
        1
    );
}

#[test]
fn harvest_from_empty_mod_returns_empty() {
    let dir = TempDir::new().unwrap();
    let capabilities = HarvestCapabilities::from_callback_slots(["vb0"]);
    let result = harvest_mod(dir.path(), &capabilities).unwrap();
    assert!(result.targets.is_empty());
    assert!(result.keybinds.is_empty());
    assert!(result.ini_fingerprints.is_empty());
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
vb0 = ResourceBody

[TEXTUREOVERRIDEALBEDOVS]
hash = aabbccdd
vb0 = ResourceHead
"#,
    );

    let result = position_targets(&path);
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
