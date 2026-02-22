use std::collections::HashSet;

use crate::services::scanner::core::normalizer;

use super::*;
use crate::services::scanner::deep_matcher::CustomSkin;

fn db_entries() -> Vec<DbEntry> {
    vec![
        DbEntry {
            name: "Raiden Shogun".to_string(),
            tags: vec!["Ei".to_string(), "Electro".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::from([(
                "Default".to_string(),
                vec!["D94C8962".to_string(), "0x00000000d94c8962".to_string()],
            )]),
        },
        DbEntry {
            name: "Albedo".to_string(),
            tags: vec!["Kreideprinz".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![CustomSkin {
                name: "Default".to_string(),
                aliases: vec![],
                thumbnail_skin_path: None,
                rarity: None,
            }],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::from([(
                "Default".to_string(),
                vec!["d94c8962".to_string(), "0xC77E380B".to_string()],
            )]),
        },
        DbEntry {
            name: "NoHash".to_string(),
            tags: vec!["Support".to_string()],
            object_type: "Other".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::from([(
                "Default".to_string(),
                vec!["invalid".to_string(), "123".to_string()],
            )]),
        },
    ]
}

fn keywords(entries: &[DbEntry]) -> Vec<(usize, HashSet<String>)> {
    entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let mut tokens = normalizer::preprocess_text(&entry.name);
            for tag in &entry.tags {
                tokens.extend(normalizer::preprocess_text(tag));
            }
            (idx, tokens)
        })
        .collect()
}

// Covers: TC-2.2-05 (index build determinism)
#[test]
fn test_index_build_is_deterministic() {
    let entries = db_entries();
    let keyword_sets = keywords(&entries);

    let first = MatcherIndexes::build(&entries, &keyword_sets);
    let second = MatcherIndexes::build(&entries, &keyword_sets);

    assert_eq!(first, second);
    assert_eq!(first.hash_index.get("d94c8962"), Some(&vec![0, 1]));
    assert_eq!(first.hash_df.get("d94c8962"), Some(&2));
    assert_eq!(first.hash_index.get("c77e380b"), Some(&vec![1]));
    assert!(first.hash_index.get("invalid").is_none());
}

// Covers: TC-2.2-06 (empty index safety)
#[test]
fn test_index_build_empty_db_safe() {
    let indexes = MatcherIndexes::build(&[], &[]);

    assert!(indexes.token_index.is_empty());
    assert!(indexes.hash_index.is_empty());
    assert!(indexes.token_df.is_empty());
    assert!(indexes.hash_df.is_empty());
    assert!(indexes.token_idf("anything", 0).is_finite());
    assert!(indexes.hash_idf("deadbeef", 0).is_finite());
}
