//! Unit tests for the hash matcher + sentinel selection.

use std::collections::{HashMap, HashSet};

use crate::services::keyviewer::matcher::{
    find_collision_hashes, match_objects, MatchConfidence, MatchConfig,
};
use crate::services::keyviewer::resource_pack::KvObjectEntry;

fn make_kv_entry(name: &str, hashes: &[&str]) -> KvObjectEntry {
    let code_hashes: Vec<String> = hashes.iter().map(|h| h.to_string()).collect();
    let mut skin_hashes = HashMap::new();
    skin_hashes.insert("Default".to_string(), code_hashes.clone());
    KvObjectEntry {
        name: name.to_string(),
        object_type: "Character".to_string(),
        code_hashes,
        skin_hashes,
        tags: vec![],
        thumbnail_path: None,
    }
}

fn make_active_hashes(hashes: &[&str]) -> HashSet<String> {
    hashes.iter().map(|h| h.to_string()).collect()
}

fn make_occurrences(hashes: &[&str], count: usize) -> HashMap<String, usize> {
    hashes.iter().map(|h| (h.to_string(), count)).collect()
}

// ─── Basic Matching ──────────────────────────────────────────────────────────

#[test]
fn matches_single_object_with_overlapping_hashes() {
    let entries = vec![make_kv_entry("Albedo", &["aabb1111", "aabb2222"])];
    let active = make_active_hashes(&["aabb1111", "aabb2222", "ccdd3333"]);
    let occ = make_occurrences(&["aabb1111", "aabb2222"], 1);
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].object_name, "Albedo");
    assert!(results[0].score >= config.score_threshold);
    assert_eq!(results[0].matched_hashes.len(), 2);
}

#[test]
fn no_match_when_no_hash_overlap() {
    let entries = vec![make_kv_entry("Albedo", &["aabb1111"])];
    let active = make_active_hashes(&["ccdd3333"]);
    let occ = HashMap::new();
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);
    assert!(results.is_empty());
}

#[test]
fn matches_multiple_objects_sorted_by_score() {
    let entries = vec![
        make_kv_entry("Albedo", &["aabb1111"]), // 1 hash overlap
        make_kv_entry("Amber", &["ccdd1111", "ccdd2222", "ccdd3333"]), // 3 hash overlap
    ];
    let active = make_active_hashes(&["aabb1111", "ccdd1111", "ccdd2222", "ccdd3333"]);
    let occ = make_occurrences(&["aabb1111", "ccdd1111", "ccdd2222", "ccdd3333"], 1);
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);

    assert_eq!(results.len(), 2);
    // Amber should rank first (more hashes → higher score)
    assert_eq!(results[0].object_name, "Amber");
    assert_eq!(results[1].object_name, "Albedo");
    assert!(results[0].score > results[1].score);
}

// ─── Score Threshold ─────────────────────────────────────────────────────────

#[test]
fn respects_score_threshold() {
    let entries = vec![make_kv_entry("Albedo", &["aabb1111"])];
    let active = make_active_hashes(&["aabb1111"]);
    let occ = make_occurrences(&["aabb1111"], 1);
    let mut config = MatchConfig::default();
    config.score_threshold = 999.0; // impossible threshold

    let results = match_objects(&entries, &active, &occ, &config);
    assert!(results.is_empty());
}

// ─── Tiebreaking ─────────────────────────────────────────────────────────────

#[test]
fn tiebreaks_by_name_when_scores_equal() {
    // Two entries with exactly the same single hash → same score
    let entries = vec![
        make_kv_entry("Zhongli", &["aabb1111"]),
        make_kv_entry("Albedo", &["aabb1111"]),
    ];
    let active = make_active_hashes(&["aabb1111"]);
    let occ = make_occurrences(&["aabb1111"], 1);
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);

    assert_eq!(results.len(), 2);
    // Same score → alphabetical order: Albedo before Zhongli
    assert_eq!(results[0].object_name, "Albedo");
    assert_eq!(results[1].object_name, "Zhongli");
}

// ─── Confidence Levels ───────────────────────────────────────────────────────

#[test]
fn assigns_correct_confidence_levels() {
    // With default config: threshold=5.0, base=10.0, occurrence_bonus~1.39, rarity_bonus=5.0
    // A single intersecting hash with rarity gives ~16.39 → ~3.3× threshold → High
    let entries = vec![make_kv_entry("Test", &["aabb1111"])];
    let active = make_active_hashes(&["aabb1111"]);
    let occ = make_occurrences(&["aabb1111"], 1);
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].confidence, MatchConfidence::High);
}

#[test]
fn excellent_confidence_with_many_hashes() {
    // 4 unique hashes → ~65+ score → 13× threshold → Excellent
    let hashes = &["aa110000", "bb220000", "cc330000", "dd440000"];
    let entries = vec![make_kv_entry("Test", hashes)];
    let active = make_active_hashes(hashes);
    let occ = make_occurrences(hashes, 1);
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].confidence, MatchConfidence::Excellent);
}

// ─── Occurrence Bonus ────────────────────────────────────────────────────────

#[test]
fn higher_occurrence_gives_higher_score() {
    let entries = vec![make_kv_entry("Test", &["aabb1111"])];
    let active = make_active_hashes(&["aabb1111"]);

    let config = MatchConfig::default();

    let occ_low = make_occurrences(&["aabb1111"], 1);
    let occ_high = make_occurrences(&["aabb1111"], 10);

    let results_low = match_objects(&entries, &active, &occ_low, &config);
    let results_high = match_objects(&entries, &active, &occ_high, &config);

    assert!(results_high[0].score > results_low[0].score);
}

// ─── Rarity Bonus ────────────────────────────────────────────────────────────

#[test]
fn rare_hash_gets_rarity_bonus() {
    // Hash only in 1 object → rare → gets bonus
    let entries = vec![make_kv_entry("Solo", &["unique01"])];
    let active = make_active_hashes(&["unique01"]);
    let occ = make_occurrences(&["unique01"], 1);
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);
    assert_eq!(results.len(), 1);

    // Score should include rarity bonus (5.0 by default)
    // base(10) + occ_bonus(2*ln(2)≈1.39) + rarity(5) ≈ 16.39
    assert!(results[0].score > 15.0);
}

#[test]
fn common_hash_no_rarity_bonus() {
    // Hash shared across 5 objects → not rare
    let entries = vec![
        make_kv_entry("A", &["shared01"]),
        make_kv_entry("B", &["shared01"]),
        make_kv_entry("C", &["shared01"]),
        make_kv_entry("D", &["shared01"]),
        make_kv_entry("E", &["shared01"]),
    ];
    let active = make_active_hashes(&["shared01"]);
    let occ = make_occurrences(&["shared01"], 1);
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);
    // All 5 should match (shared hash)
    assert_eq!(results.len(), 5);
    // Score should NOT include rarity bonus (hash in 5 objects, threshold is 2)
    // base(10) + occ_bonus(2*ln(2)≈1.39) ≈ 11.39
    for r in &results {
        assert!(r.score < 13.0);
    }
}

// ─── Sentinel Selection ──────────────────────────────────────────────────────

#[test]
fn sentinels_prefer_rare_hashes() {
    // "unique01" is only in this entry, "shared01" is in 3 entries
    let entries = vec![
        make_kv_entry("Target", &["unique01", "shared01"]),
        make_kv_entry("Other1", &["shared01"]),
        make_kv_entry("Other2", &["shared01"]),
    ];
    let active = make_active_hashes(&["unique01", "shared01"]);
    let occ = make_occurrences(&["unique01", "shared01"], 1);
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);

    let target = results.iter().find(|r| r.object_name == "Target").unwrap();
    // Sentinel should prefer "unique01" (appears in only 1 object)
    assert!(!target.sentinel_hashes.is_empty());
    assert_eq!(target.sentinel_hashes[0], "unique01");
}

#[test]
fn sentinels_exclude_high_collision_hashes() {
    // "collision01" appears in 4 objects → above collision_threshold(3)
    let entries = vec![
        make_kv_entry("Target", &["collision01", "safe01"]),
        make_kv_entry("Other1", &["collision01"]),
        make_kv_entry("Other2", &["collision01"]),
        make_kv_entry("Other3", &["collision01"]),
    ];
    let active = make_active_hashes(&["collision01", "safe01"]);
    let occ = make_occurrences(&["collision01", "safe01"], 1);
    let mut config = MatchConfig::default();
    config.collision_threshold = 3;

    let results = match_objects(&entries, &active, &occ, &config);

    let target = results.iter().find(|r| r.object_name == "Target").unwrap();
    // "collision01" should be excluded (in 4 objects ≥ threshold 3)
    assert!(!target.sentinel_hashes.contains(&"collision01".to_string()));
    // "safe01" should be selected
    assert!(target.sentinel_hashes.contains(&"safe01".to_string()));
}

#[test]
fn sentinel_count_limited_by_config() {
    let entries = vec![make_kv_entry("Test", &["h1", "h2", "h3", "h4", "h5"])];
    let active = make_active_hashes(&["h1", "h2", "h3", "h4", "h5"]);
    let occ = make_occurrences(&["h1", "h2", "h3", "h4", "h5"], 1);
    let mut config = MatchConfig::default();
    config.sentinel_count = 2;

    let results = match_objects(&entries, &active, &occ, &config);
    assert_eq!(results[0].sentinel_hashes.len(), 2);
}

// ─── Collision Detection ─────────────────────────────────────────────────────

#[test]
fn find_collision_hashes_detects_shared() {
    let entries = vec![
        make_kv_entry("A", &["shared01", "unique_a"]),
        make_kv_entry("B", &["shared01", "unique_b"]),
        make_kv_entry("C", &["shared01", "unique_c"]),
    ];

    let collisions = find_collision_hashes(&entries, 3);
    assert!(collisions.contains("shared01"));
    assert!(!collisions.contains("unique_a"));
    assert!(!collisions.contains("unique_b"));
    assert!(!collisions.contains("unique_c"));
}

#[test]
fn find_collision_hashes_empty_when_no_collisions() {
    let entries = vec![
        make_kv_entry("A", &["unique_a"]),
        make_kv_entry("B", &["unique_b"]),
    ];

    let collisions = find_collision_hashes(&entries, 3);
    assert!(collisions.is_empty());
}

// ─── Edge Cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_entries_returns_empty() {
    let entries: Vec<KvObjectEntry> = vec![];
    let active = make_active_hashes(&["aabb1111"]);
    let occ = HashMap::new();
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);
    assert!(results.is_empty());
}

#[test]
fn empty_active_hashes_returns_empty() {
    let entries = vec![make_kv_entry("Albedo", &["aabb1111"])];
    let active: HashSet<String> = HashSet::new();
    let occ = HashMap::new();
    let config = MatchConfig::default();

    let results = match_objects(&entries, &active, &occ, &config);
    assert!(results.is_empty());
}

#[test]
fn sentinels_empty_when_all_hashes_high_collision() {
    // All matched hashes are in 4+ objects → all excluded
    let entries = vec![
        make_kv_entry("Target", &["c1", "c2"]),
        make_kv_entry("O1", &["c1", "c2"]),
        make_kv_entry("O2", &["c1", "c2"]),
        make_kv_entry("O3", &["c1", "c2"]),
    ];
    let active = make_active_hashes(&["c1", "c2"]);
    let occ = make_occurrences(&["c1", "c2"], 1);
    let mut config = MatchConfig::default();
    config.collision_threshold = 3;

    let results = match_objects(&entries, &active, &occ, &config);
    let target = results.iter().find(|r| r.object_name == "Target").unwrap();
    // All hashes in 4 objects ≥ threshold 3 → empty sentinels
    assert!(target.sentinel_hashes.is_empty());
}
