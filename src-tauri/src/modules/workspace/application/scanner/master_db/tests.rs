use super::*;

fn db(json: &str) -> MasterDb {
    MasterDb::from_json(json).expect("valid test database")
}

#[test]
fn deserializes_entries_as_canonical() {
    let parsed = db(
        r#"{"entries":[{"name":"Weapon","object_type":"Weapon"},{"name":"Jean","object_type":"Character"}]}"#,
    );

    assert_eq!(parsed.entries[0].entry_kind, EntryKind::Canonical);
    assert_eq!(parsed.entries[1].entry_kind, EntryKind::Canonical);
}

#[test]
fn deserializes_explicit_taxonomy_entries_without_guessing_from_the_name() {
    let parsed =
        db(r#"{"entries":[{"name":"Weapon","object_type":"Weapon","entry_kind":"taxonomy"}]}"#);

    assert_eq!(parsed.entries[0].entry_kind, EntryKind::Taxonomy);
}

#[test]
fn search_filters_nonempty_queries_and_includes_custom_skin_aliases() {
    let parsed = db(r#"{
        "entries": [
            {"name":"Silver Wolf","object_type":"Character","custom_skins":[{"name":"Punklorde","aliases":["hacker"]}]},
            {"name":"Hacker Weapon","object_type":"Weapon"}
        ]
    }"#);
    let results = search_master_db_service(
        &parsed,
        std::path::Path::new("resources"),
        "hacker",
        Some("Character"),
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].item.name, "Silver Wolf");
}

#[test]
fn missing_catalog_pack_loads_an_empty_database() {
    let app_data_dir = tempfile::tempdir().expect("temporary app-data directory");

    let entries = load_master_db_entries(app_data_dir.path(), 1)
        .expect("an absent optional catalog pack must not block matching");

    assert!(entries.is_empty());
}

#[test]
fn keyviewer_reports_a_missing_catalog_instead_of_an_empty_success() {
    let app_data_dir = tempfile::tempdir().expect("temporary app-data directory");

    let error = load_catalog_keyviewer_entries(app_data_dir.path(), 0)
        .expect_err("runtime detection needs an installed catalog pack");

    assert!(error.to_string().contains("not installed"));
}

fn install_catalog(app_data_dir: &std::path::Path, catalog: &str) {
    use sha2::Digest;

    let pack_dir = app_data_dir.join("asset-pack");
    std::fs::create_dir_all(&pack_dir).expect("create catalog pack directory");
    std::fs::write(pack_dir.join("gimi.json"), catalog).expect("write catalog");
    let checksum = format!("{:x}", sha2::Sha256::digest(catalog.as_bytes()));
    let manifest = serde_json::json!({
        "id": "fixture-catalog",
        "version": "2026.09.13",
        "author": "EMMM test",
        "source": "https://example.invalid/catalog",
        "catalogs": {
            "gimi": {
                "path": "gimi.json",
                "sha256": checksum,
            }
        }
    });
    std::fs::write(
        pack_dir.join("manifest.json"),
        serde_json::to_vec(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");
}

#[test]
fn keyviewer_loader_excludes_legacy_and_shader_only_targets() {
    let app_data_dir = tempfile::tempdir().expect("temporary app-data directory");
    install_catalog(
        app_data_dir.path(),
        r#"{
            "entries": [
                {
                    "name": "Legacy",
                    "object_type": "Character",
                    "hash_db": {"Default": ["deadbeef"]}
                },
                {
                    "name": "Runtime Character",
                    "aliases": ["runtime"],
                    "object_type": "Character",
                    "runtime_targets": [
                        {
                            "variant": "default",
                            "component": "body",
                            "resource_kind": "position_vb",
                            "hash": "A1B2C3D4",
                            "slot": "0",
                            "provenance": {
                                "source_repo": "github.com/example/catalog",
                                "commit": "abc123",
                                "path": "gimi/runtime-character.json"
                            }
                        },
                        {
                            "variant": "default",
                            "resource_kind": "shader",
                            "hash": "12345678ABCDEF00",
                            "provenance": {
                                "source_repo": "github.com/example/catalog",
                                "commit": "abc123",
                                "path": "gimi/runtime-character.json"
                            }
                        }
                    ]
                }
            ]
        }"#,
    );

    let master_entries =
        load_master_db_entries(app_data_dir.path(), 0).expect("load matching catalog entries");
    assert_eq!(master_entries.len(), 2);
    assert_eq!(
        master_entries[0].hash_db.get("Default"),
        Some(&vec!["deadbeef".to_string()])
    );

    let entries =
        load_catalog_keyviewer_entries(app_data_dir.path(), 0).expect("load valid runtime targets");

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.name, "Runtime Character");
    assert_eq!(entry.aliases, vec!["runtime".to_string()]);
    assert_eq!(entry.object_type, "Character");
    assert_eq!(entry.catalog_id, "fixture-catalog");
    assert_eq!(entry.catalog_version, "2026.09.13");
    assert_eq!(entry.runtime_targets.len(), 1);
    let target = &entry.runtime_targets[0];
    assert_eq!(target.hash, "A1B2C3D4");
    assert_eq!(target.variant, "default");
    assert_eq!(target.component.as_deref(), Some("body"));
    assert_eq!(target.slot.as_deref(), Some("0"));
    assert_eq!(target.provenance.source_repo, "github.com/example/catalog");
    assert_eq!(target.provenance.commit, "abc123");
    assert_eq!(target.provenance.path, "gimi/runtime-character.json");
}

#[test]
fn keyviewer_loader_rejects_invalid_runtime_target_hashes() {
    let app_data_dir = tempfile::tempdir().expect("temporary app-data directory");
    install_catalog(
        app_data_dir.path(),
        r#"{
            "entries": [{
                "name": "Invalid",
                "object_type": "Character",
                "runtime_targets": [{
                    "variant": "default",
                    "resource_kind": "texture",
                    "hash": "1234567890abcdef",
                    "provenance": {
                        "source_repo": "github.com/example/catalog",
                        "commit": "abc123",
                        "path": "gimi/invalid.json"
                    }
                }]
            }]
        }"#,
    );

    let error = load_catalog_keyviewer_entries(app_data_dir.path(), 0)
        .expect_err("16-hex resource hashes must be rejected");

    assert!(error.to_string().contains("catalog runtime target"));
    assert!(error.to_string().contains("exactly 8 hexadecimal"));
}
