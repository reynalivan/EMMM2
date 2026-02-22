use std::collections::HashSet;

use crate::services::scanner::deep_matcher::CustomSkin;

use super::*;
use crate::services::scanner::core::normalizer;
use crate::services::scanner::deep_matcher::analysis::indexes::MatcherIndexes;
use crate::services::scanner::deep_matcher::models::types::DbEntry;

fn sample_indexes() -> MatcherIndexes {
    let entries = vec![
        DbEntry {
            name: "Raiden Shogun".to_string(),
            tags: vec!["electro".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::from([(
                "Default".to_string(),
                vec!["aaaaaaaa".to_string()],
            )]),
        },
        DbEntry {
            name: "Jean".to_string(),
            tags: vec!["dandelion".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::from([(
                "Default".to_string(),
                vec!["aaaaaaaa".to_string()],
            )]),
        },
        DbEntry {
            name: "Albedo".to_string(),
            tags: vec!["chalk".to_string()],
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
                vec!["bbbbbbbb".to_string()],
            )]),
        },
        DbEntry {
            name: "Diluc".to_string(),
            tags: vec![],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::new(),
        },
    ];

    let keywords: Vec<(usize, HashSet<String>)> = entries
        .iter()
        .enumerate()
        .map(|(entry_id, entry)| {
            let mut tokens = normalizer::preprocess_text(&entry.name);
            for tag in &entry.tags {
                tokens.extend(normalizer::preprocess_text(tag));
            }
            (entry_id, tokens)
        })
        .collect();

    MatcherIndexes::build(&entries, &keywords)
}

// Covers: TC-2.2-Task9-01
#[test]
fn test_seed_candidates_rarity_first_with_seed_cap() {
    let indexes = sample_indexes();
    let observed_hashes = vec!["aaaaaaaa".to_string(), "bbbbbbbb".to_string()];
    let observed_tokens: HashSet<String> = ["chalk", "raiden", "electro"]
        .into_iter()
        .map(str::to_string)
        .collect();

    let seeded = seed_candidates(&indexes, &observed_hashes, &observed_tokens, 2);

    // bbbbbbbb and chalk both map only to Albedo (entry 2), then rare electro maps to Raiden.
    assert_eq!(seeded, vec![0, 2]);
}

// Covers: TC-2.2-Task9-02
#[test]
fn test_seed_candidates_empty_signals_safe() {
    let indexes = sample_indexes();
    let seeded = seed_candidates(&indexes, &[], &HashSet::new(), 200);
    assert!(seeded.is_empty());
}

#[test]
fn test_replenish_candidates_when_below_min_pool() {
    let indexes = sample_indexes();
    let buckets = ObservedTokenBuckets {
        folder_tokens: ["electro".to_string(), "chalk".to_string()]
            .into_iter()
            .collect(),
        deep_name_tokens: ["raiden".to_string()].into_iter().collect(),
        ini_section_tokens: BTreeSet::new(),
        ini_content_tokens: BTreeSet::new(),
    };

    let replenished = replenish_candidates_if_needed(&indexes, &[2], &buckets, 3, 4);
    assert_eq!(replenished, vec![0, 2]);
}
