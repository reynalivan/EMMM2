//! Unit tests for `resource_pack::extract_kv_entries`.

use std::collections::HashMap;

use crate::services::scanner::deep_matcher::models::types::DbEntry;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;

use crate::services::keyviewer::resource_pack::extract_kv_entries;

fn make_entry(name: &str, hash_db: HashMap<String, Vec<String>>) -> DbEntry {
    DbEntry {
        name: name.to_string(),
        tags: vec!["Tag1".to_string()],
        object_type: "Character".to_string(),
        custom_skins: Vec::new(),
        thumbnail_path: Some(format!("thumbnails/{}.png", name.to_lowercase())),
        metadata: None,
        hash_db,
    }
}

fn make_db(entries: Vec<DbEntry>) -> MasterDb {
    MasterDb::new(entries)
}

#[test]
fn extracts_single_default_hash() {
    let mut hash_db = HashMap::new();
    hash_db.insert("Default".to_string(), vec!["df65bb00".to_string()]);

    let db = make_db(vec![make_entry("Albedo", hash_db)]);
    let kv_entries = extract_kv_entries(&db);

    assert_eq!(kv_entries.len(), 1);
    assert_eq!(kv_entries[0].name, "Albedo");
    assert_eq!(kv_entries[0].object_type, "Character");
    assert_eq!(kv_entries[0].code_hashes, vec!["df65bb00"]);
    assert_eq!(kv_entries[0].skin_hashes.len(), 1);
    assert!(kv_entries[0].skin_hashes.contains_key("Default"));
}

#[test]
fn extracts_multiple_skin_hashes() {
    let mut hash_db = HashMap::new();
    hash_db.insert("Default".to_string(), vec!["a2ea4b2d".to_string()]);
    hash_db.insert("100% Outrider".to_string(), vec!["557b2eff".to_string()]);

    let db = make_db(vec![make_entry("Amber", hash_db)]);
    let kv_entries = extract_kv_entries(&db);

    assert_eq!(kv_entries.len(), 1);
    assert_eq!(kv_entries[0].skin_hashes.len(), 2);
    // Flattened and sorted
    assert_eq!(kv_entries[0].code_hashes, vec!["557b2eff", "a2ea4b2d"]);
}

#[test]
fn skips_entries_with_empty_hash_db() {
    let db = make_db(vec![make_entry("Arataki Itto", HashMap::new()), {
        let mut hash_db = HashMap::new();
        hash_db.insert("Default".to_string(), vec!["df65bb00".to_string()]);
        make_entry("Albedo", hash_db)
    }]);

    let kv_entries = extract_kv_entries(&db);
    assert_eq!(kv_entries.len(), 1);
    assert_eq!(kv_entries[0].name, "Albedo");
}

#[test]
fn normalizes_hashes_to_lowercase() {
    let mut hash_db = HashMap::new();
    hash_db.insert("Default".to_string(), vec!["DF65BB00".to_string()]);

    let db = make_db(vec![make_entry("Test", hash_db)]);
    let kv_entries = extract_kv_entries(&db);

    assert_eq!(kv_entries[0].code_hashes, vec!["df65bb00"]);
}

#[test]
fn deduplicates_hashes_across_skins() {
    let mut hash_db = HashMap::new();
    // Same hash appears under two skin names
    hash_db.insert("Default".to_string(), vec!["aabbccdd".to_string()]);
    hash_db.insert("Skin2".to_string(), vec!["aabbccdd".to_string()]);

    let db = make_db(vec![make_entry("Dupe", hash_db)]);
    let kv_entries = extract_kv_entries(&db);

    // Code hashes should be deduped
    assert_eq!(kv_entries[0].code_hashes, vec!["aabbccdd"]);
    // But skin_hashes preserves both
    assert_eq!(kv_entries[0].skin_hashes.len(), 2);
}

#[test]
fn preserves_tags_and_thumbnail() {
    let mut hash_db = HashMap::new();
    hash_db.insert("Default".to_string(), vec!["12345678".to_string()]);

    let db = make_db(vec![make_entry("Foo", hash_db)]);
    let kv_entries = extract_kv_entries(&db);

    assert_eq!(kv_entries[0].tags, vec!["Tag1"]);
    assert_eq!(
        kv_entries[0].thumbnail_path.as_deref(),
        Some("thumbnails/foo.png")
    );
}

#[test]
fn returns_empty_for_all_empty_hash_db() {
    let db = make_db(vec![
        make_entry("A", HashMap::new()),
        make_entry("B", HashMap::new()),
    ]);

    let kv_entries = extract_kv_entries(&db);
    assert!(kv_entries.is_empty());
}
