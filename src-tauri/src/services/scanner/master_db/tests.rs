use super::*;

fn db(json: &str) -> MasterDb {
    MasterDb::from_json(json).expect("valid test database")
}

#[test]
fn deserializes_legacy_entries_as_canonical() {
    let parsed = db(
        r#"[{"name":"Weapon","object_type":"Weapon"},{"name":"Jean","object_type":"Character"}]"#,
    );

    assert_eq!(parsed.entries[0].entry_kind, EntryKind::Canonical);
    assert_eq!(parsed.entries[1].entry_kind, EntryKind::Canonical);
}

#[test]
fn deserializes_explicit_taxonomy_entries_without_guessing_from_the_name() {
    let parsed = db(r#"[{"name":"Weapon","object_type":"Weapon","entry_kind":"taxonomy"}]"#);

    assert_eq!(parsed.entries[0].entry_kind, EntryKind::Taxonomy);
}

#[test]
fn search_filters_nonempty_queries_and_includes_custom_skin_aliases() {
    let parsed = db(r#"[
            {"name":"Silver Wolf","object_type":"Character","custom_skins":[{"name":"Punklorde","aliases":["hacker"]}]},
            {"name":"Hacker Weapon","object_type":"Weapon"}
        ]"#);
    let results = search_master_db_service(
        &parsed,
        std::path::Path::new("resources"),
        "hacker",
        Some("Character"),
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].item.name, "Silver Wolf");
}
