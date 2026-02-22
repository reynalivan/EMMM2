use super::*;
use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::models::types::Confidence;
use crate::services::scanner::deep_matcher::models::types::DbEntry;
use crate::services::scanner::deep_matcher::{Candidate, MasterDb};
use std::collections::{HashMap, HashSet};

fn mock_db_with_entry(entry_id: usize, entry: DbEntry) -> MasterDb {
    let mut entries = Vec::new();

    // Add entry
    while entries.len() <= entry_id {
        entries.push(DbEntry {
            name: "Dummy".to_string(),
            tags: vec![],
            object_type: "Other".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: HashMap::new(),
        });
    }
    entries[entry_id] = entry.clone();

    MasterDb::new(entries)
}

fn base_candidate(entry_id: usize) -> Candidate {
    Candidate {
        entry_id,
        name: "Test".to_string(),
        object_type: "Character".to_string(),
        score: 0.0,
        confidence: Confidence::None,
        reasons: vec![],
    }
}

#[test]
fn test_gb_exact_mod_name_bonus() {
    let entry = DbEntry {
        name: "Zibai Lunar Qilin".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: HashMap::new(),
    };
    let db = mock_db_with_entry(0, entry);
    let candidate = base_candidate(0);
    let signals = FolderSignals::default();

    // config without gb_mod_name
    let mut config = MechanicalRerankConfig::default();
    let score_without = compute_points(&candidate, &signals, &db, &config);

    // config WITH gb_mod_name
    config.gb_mod_name = Some("[Mod] Zibai Lunar Qilin".to_string()); // Normalize will strip prefix
    let score_with = compute_points(&candidate, &signals, &db, &config);

    assert!(
        score_with > score_without,
        "Expected significant bonus for gb_mod_name match"
    );
    assert_eq!(score_with - score_without, PT_GB_EXACT_MOD_NAME);
}

#[test]
fn test_gb_category_mismatch_penalty() {
    let entry = DbEntry {
        name: "Skyward Harp Replacement".to_string(),
        tags: vec![],
        object_type: "Weapon".to_string(), // It's a Weapon
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: HashMap::new(),
    };
    let db = mock_db_with_entry(0, entry);
    let candidate = base_candidate(0);
    let signals = FolderSignals::default();

    // config where GB says it's a "Skins", but we know it's a "Weapon"
    let mut config = MechanicalRerankConfig::default();
    config.gb_root_category = Some("Skins".to_string());

    let score = compute_points(&candidate, &signals, &db, &config);

    assert!(
        score < 0.0,
        "Expected penalty for category mismatch to make score negative"
    );
    assert_eq!(score, PENALTY_GB_CATEGORY_MISMATCH); // Base score is 0, minus penalty (-15)
}

#[test]
fn test_gb_description_keywords_bonus() {
    let entry = DbEntry {
        name: "Raiden Shogun Outfit".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: HashMap::new(),
    };
    let db = mock_db_with_entry(0, entry);
    let candidate = base_candidate(0);
    let signals = FolderSignals::default();

    let mut config = MechanicalRerankConfig::default();
    let score_no_desc = compute_points(&candidate, &signals, &db, &config);

    // Add keywords: 'raiden' and 'shogun' match the name/tokens
    config.gb_description_keywords = vec![
        "raiden".to_string(),
        "shogun".to_string(),
        "outfit".to_string(),
    ];
    let score_desc = compute_points(&candidate, &signals, &db, &config);

    assert!(score_desc > score_no_desc);
    assert_eq!(score_desc, 6.0); // 3 keywords * 2 points each = 6 points
}
