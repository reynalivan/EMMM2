use super::*;
use crate::services::scanner::deep_matcher::{CustomSkin, DbEntry, MatchStatus};

fn test_db() -> MasterDb {
    MasterDb::new(vec![
        DbEntry {
            name: "Raiden Shogun".to_string(),
            tags: vec!["raiden".to_string(), "electro".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![CustomSkin {
                name: "Wish".to_string(),
                aliases: vec!["raidenwish".to_string()],
                thumbnail_skin_path: None,
                rarity: None,
            }],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::from([(
                "Default".to_string(),
                vec!["d94c8962".to_string(), "deadbeef".to_string()],
            )]),
        },
        DbEntry {
            name: "Ayaka".to_string(),
            tags: vec!["cryo".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::new(),
        },
        DbEntry {
            name: "Nahida".to_string(),
            tags: vec!["dendro".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::from([(
                "Default".to_string(),
                vec!["aaaa1111".to_string()],
            )]),
        },
    ])
}

// Covers: TC-2.2-Task19-09 (Golden corpus: unique hash auto-match)
#[test]
fn test_golden_unique_hash_auto_match() {
    let case = GoldenCase {
        name: "unique_hash_auto",
        folder_name: "MyMod",
        ini_content: Some("[TextureOverride]\nhash = deadbeef\n"),
        subfolders: vec![],
        expected_status_quick: MatchStatus::AutoMatched,
        expected_best_name_quick: Some("Raiden Shogun"),
        expected_status_full: MatchStatus::AutoMatched,
        expected_best_name_full: Some("Raiden Shogun"),
    };
    run_golden_case(&case, &test_db());
}

#[test]
#[should_panic(expected = "Quick: expected status")]
fn test_golden_mismatch_detection_status() {
    // Intentional mismatch: expect NeedsReview but will get AutoMatched (strong hash signal)
    let case = GoldenCase {
        name: "mismatch_demo_status",
        folder_name: "StrongHashMod",
        ini_content: Some("[TextureOverride]\nhash = deadbeef\n"),
        subfolders: vec![],
        expected_status_quick: MatchStatus::NeedsReview, // Wrong: will be AutoMatched
        expected_best_name_quick: Some("Raiden Shogun"),
        expected_status_full: MatchStatus::AutoMatched,
        expected_best_name_full: Some("Raiden Shogun"),
    };
    run_golden_case(&case, &test_db());
}

#[test]
#[should_panic(expected = "Quick: expected status")]
fn test_golden_mismatch_detection_no_match_expected_auto() {
    // Intentional mismatch: expect AutoMatched but insufficient signals = NoMatch
    let case = GoldenCase {
        name: "mismatch_demo_insufficient",
        folder_name: "WeakMod",
        ini_content: Some("[TextureOverride]\nsomething = value\n"), // No hash
        subfolders: vec![],
        expected_status_quick: MatchStatus::AutoMatched, // Wrong: will be NoMatch
        expected_best_name_quick: Some("Raiden Shogun"),
        expected_status_full: MatchStatus::NoMatch,
        expected_best_name_full: None,
    };
    run_golden_case(&case, &test_db());
}

// Covers: TC-2.2-Task19-10 (Golden corpus: deep scan fallback when no hashes)
#[test]
fn test_golden_deep_scan_fallback_no_hash() {
    let case = GoldenCase {
        name: "deep_scan_fallback",
        folder_name: "ayaka_cryo_pack",
        ini_content: Some("[TextureOverrideAyaka]\nfilename = ayaka_body.dds\n"),
        subfolders: vec!["ayaka"],
        expected_status_quick: MatchStatus::AutoMatched,
        expected_best_name_quick: Some("Ayaka"),
        expected_status_full: MatchStatus::AutoMatched,
        expected_best_name_full: Some("Ayaka"),
    };
    run_golden_case(&case, &test_db());
}

// Covers: TC-2.2-Task19-11 (Golden corpus: direct name only does not auto-match)
#[test]
fn test_golden_direct_name_only_no_auto() {
    let case = GoldenCase {
        name: "direct_name_only",
        folder_name: "nahida",
        ini_content: None,
        subfolders: vec![],
        expected_status_quick: MatchStatus::NeedsReview,
        expected_best_name_quick: Some("Nahida"),
        expected_status_full: MatchStatus::NeedsReview,
        expected_best_name_full: Some("Nahida"),
    };
    run_golden_case(&case, &test_db());
}

// Covers: TC-2.2-Task19-12 (Golden corpus: no match when insufficient signals)
#[test]
fn test_golden_no_match_insufficient_signals() {
    let case = GoldenCase {
        name: "no_match_case",
        folder_name: "unknown_xyz_pack",
        ini_content: Some("[TextureOverride]\nfilename = xyz.dds\n"),
        subfolders: vec![],
        expected_status_quick: MatchStatus::NoMatch,
        expected_best_name_quick: None,
        expected_status_full: MatchStatus::NoMatch,
        expected_best_name_full: None,
    };
    run_golden_case(&case, &test_db());
}
