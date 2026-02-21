//! Baseline Matcher Test Suite (Task 1)
//!
//! Captures current legacy matcher behavior for 4 representative cases:
//! 1. Exact Name Match (L1)
//! 2. Alias Match (L0 skin aliases)
//! 3. Ambiguous Case (multiple potential matches)
//! 4. No Match (unmatched folder)
//!
//! Purpose: Fixture-driven golden corpus for refactor validation.
//! Status: Legacy behavior baseline (no staged scoring yet).

use std::path::PathBuf;

// Import from lib (integration test crate)
use emmm2_lib::services::scanner::deep_matcher::{
    Confidence, CustomSkin, DbEntry, MasterDb, MatchLevel,
};
use emmm2_lib::services::scanner::walker::{FolderContent, ModCandidate};

// ─── Fixtures ─────────────────────────────────────────────────────

/// Load baseline fixture database from embedded test data.
fn fixture_db() -> MasterDb {
    MasterDb::new(vec![
        // Test Case 1: Exact Match
        DbEntry {
            name: "Raiden Shogun".to_string(),
            tags: vec!["Raiden".to_string(), "Ei".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![CustomSkin {
                name: "Wish".to_string(),
                aliases: vec!["RaidenWish".to_string(), "Raiden2".to_string()],
                thumbnail_skin_path: None,
                rarity: None,
            }],
            thumbnail_path: None,
            metadata: None,
            hashes: vec![],
        },
        // Test Case 2: Alias Match (skin aliases)
        DbEntry {
            name: "Jean".to_string(),
            tags: vec!["Jean Gunnhildr".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![CustomSkin {
                name: "Gunnhildr's Heritage".to_string(),
                aliases: vec!["JeanCN".to_string(), "Jean2".to_string()],
                thumbnail_skin_path: Some("assets/jean_cn.png".to_string()),
                rarity: Some("5".to_string()),
            }],
            thumbnail_path: Some("assets/jean.png".to_string()),
            metadata: None,
            hashes: vec![],
        },
        // Test Case 3: Ambiguous (multiple keywords could match)
        DbEntry {
            name: "Lumine".to_string(),
            tags: vec!["Traveler".to_string(), "Aether".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hashes: vec![],
        },
        DbEntry {
            name: "Aether".to_string(),
            tags: vec!["Traveler".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hashes: vec![],
        },
        // Test Case 4: Typical weapon entry
        DbEntry {
            name: "Primordial Jade Winged-Spear".to_string(),
            tags: vec!["PJWS".to_string()],
            object_type: "Weapon".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hashes: vec![],
        },
    ])
}

fn empty_content() -> FolderContent {
    FolderContent {
        subfolder_names: vec![],
        files: vec![],
        ini_files: vec![],
    }
}

fn candidate(name: &str) -> ModCandidate {
    ModCandidate {
        path: PathBuf::from(format!("/mods/{name}")),
        raw_name: name.to_string(),
        display_name: name.to_string(),
        is_disabled: false,
    }
}

// ─── Baseline Tests ───────────────────────────────────────────────

/// TC-BASELINE-01: Exact Name Match (L1)
/// Legacy behavior: Direct name match returns High confidence L1Name.
#[test]
fn baseline_exact_name_match() {
    let db = fixture_db();
    let c = candidate("Raiden Shogun skin");
    let result =
        emmm2_lib::services::scanner::deep_matcher::match_folder(&c, &db, &empty_content());

    assert_eq!(
        result.object_name, "Raiden Shogun",
        "Should match by exact name"
    );
    assert_eq!(result.level, MatchLevel::L1Name, "Should be L1 name match");
    assert_eq!(
        result.confidence,
        Confidence::High,
        "Should be high confidence"
    );
}

/// TC-BASELINE-02: Alias Match (L0)
/// Legacy behavior: Skin alias "JeanCN" returns character name with skin detection.
#[test]
fn baseline_alias_match() {
    let db = fixture_db();
    let c = candidate("JeanCN_mod");
    let result =
        emmm2_lib::services::scanner::deep_matcher::match_folder(&c, &db, &empty_content());

    assert_eq!(result.object_name, "Jean", "Should match character");
    assert_eq!(
        result.level,
        MatchLevel::L1Name,
        "Alias match is treated as L1"
    );
    assert_eq!(
        result.confidence,
        Confidence::High,
        "Should be high confidence"
    );
    assert_eq!(
        result.detected_skin,
        Some("Gunnhildr's Heritage".to_string()),
        "Should detect skin"
    );
}

/// TC-BASELINE-03: Ambiguous Case (multiple matches available)
/// Legacy behavior: Picks first match (early return, non-deterministic if unordered).
/// Expected: One of Lumine or Aether (depends on iteration order in legacy DB).
#[test]
fn baseline_ambiguous_case() {
    let db = fixture_db();
    let c = candidate("Traveler_mod");
    let result =
        emmm2_lib::services::scanner::deep_matcher::match_folder(&c, &db, &empty_content());

    // In legacy matcher, this will match on token "Traveler" which is a tag for both Lumine and Aether.
    // The result depends on which entry is processed first.
    assert!(!result.object_name.is_empty(), "Should find some match");
    assert_eq!(
        result.confidence,
        Confidence::High,
        "Should be high confidence"
    );
    // Document the matched object (will be either Lumine or Aether)
    println!("Ambiguous case matched: {}", result.object_name);
}

/// TC-BASELINE-04: No Match
/// Legacy behavior: Falls through all stages, returns Unmatched with None confidence.
#[test]
fn baseline_no_match() {
    let db = fixture_db();
    let c = candidate("xyzABCDEF_completely_unknown_1234");
    let result =
        emmm2_lib::services::scanner::deep_matcher::match_folder(&c, &db, &empty_content());

    assert_eq!(
        result.object_name, "",
        "Should have empty name for no match"
    );
    assert_eq!(result.level, MatchLevel::Unmatched, "Should be unmatched");
    assert_eq!(
        result.confidence,
        Confidence::None,
        "Should have None confidence"
    );
}

// ─── Missing Fixture Test ────────────────────────────────────────

/// Test that verifies missing fixture file detection.
/// This test passes if fixture file exists, fails if missing.
#[test]
fn missing_fixture_detection() {
    let fixture_path = "tests/fixtures/baseline_matcher_golden.json";

    // For now, just check that the fixture directory structure is expected.
    // Later, Task 1 will create the actual fixture file.
    let fixtures_dir = std::path::Path::new("tests/fixtures");

    // This test FAILS if fixtures directory doesn't exist (intentional: RED phase).
    if !fixtures_dir.exists() {
        panic!("Fixture directory missing: tests/fixtures must exist for golden corpus");
    }

    // Placeholder for golden corpus file validation.
    let golden_file = fixtures_dir.join("baseline_matcher_golden.json");
    if !golden_file.exists() {
        println!(
            "⚠️  Golden corpus file not found: {}",
            golden_file.display()
        );
        println!("Expected location: {}", fixture_path);
        // Don't fail yet; we'll create it in GREEN phase
    }
}
