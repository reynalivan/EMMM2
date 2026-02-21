//! Golden corpus test harness for matcher threshold tuning.
//!
//! Provides a minimal fixture-driven test system that asserts expected status
//! and best_entry for deterministic matcher behavior validation.

use std::path::PathBuf;

use tempfile::TempDir;

use crate::services::scanner::walker::{scan_folder_content, ModCandidate};

use super::content::IniTokenizationConfig;
use super::types::{DbEntry, MatchStatus};
use super::MasterDb;
use super::{match_folder_full, match_folder_quick};

/// Single golden corpus test case.
pub struct GoldenCase {
    pub name: &'static str,
    pub folder_name: &'static str,
    pub ini_content: Option<&'static str>,
    pub subfolders: Vec<&'static str>,
    pub expected_status_quick: MatchStatus,
    pub expected_best_name_quick: Option<&'static str>,
    pub expected_status_full: MatchStatus,
    pub expected_best_name_full: Option<&'static str>,
}

fn candidate_for(path: PathBuf, display_name: &str) -> ModCandidate {
    ModCandidate {
        path,
        raw_name: display_name.to_string(),
        display_name: display_name.to_string(),
        is_disabled: false,
    }
}

/// Execute a single golden corpus test case.
pub fn run_golden_case(case: &GoldenCase, db: &MasterDb) {
    let temp = TempDir::new().expect("temp dir");
    let folder = temp.path().join(case.folder_name);
    std::fs::create_dir_all(&folder).expect("create folder");

    for subfolder in &case.subfolders {
        std::fs::create_dir_all(folder.join(subfolder)).expect("create subfolder");
    }

    if let Some(ini) = case.ini_content {
        std::fs::write(folder.join("mod.ini"), ini).expect("write ini");
    }

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), case.folder_name);

    // Test Quick mode
    let result_quick =
        match_folder_quick(&candidate, db, &content, &IniTokenizationConfig::default());
    assert_eq!(
        result_quick.status, case.expected_status_quick,
        "Golden case '{}' Quick: expected status {:?}, got {:?}",
        case.name, case.expected_status_quick, result_quick.status
    );
    if let Some(expected_name) = case.expected_best_name_quick {
        assert!(
            result_quick.best.is_some(),
            "Golden case '{}' Quick: expected best candidate",
            case.name
        );
        let best = result_quick.best.unwrap();
        assert_eq!(
            best.name, expected_name,
            "Golden case '{}' Quick: expected best '{}', got '{}'",
            case.name, expected_name, best.name
        );
    } else {
        assert!(
            result_quick.best.is_none(),
            "Golden case '{}' Quick: expected no best candidate",
            case.name
        );
    }

    // Test Full mode
    let result_full =
        match_folder_full(&candidate, db, &content, &IniTokenizationConfig::default());
    assert_eq!(
        result_full.status, case.expected_status_full,
        "Golden case '{}' Full: expected status {:?}, got {:?}",
        case.name, case.expected_status_full, result_full.status
    );
    if let Some(expected_name) = case.expected_best_name_full {
        assert!(
            result_full.best.is_some(),
            "Golden case '{}' Full: expected best candidate",
            case.name
        );
        let best = result_full.best.unwrap();
        assert_eq!(
            best.name, expected_name,
            "Golden case '{}' Full: expected best '{}', got '{}'",
            case.name, expected_name, best.name
        );
    } else {
        assert!(
            result_full.best.is_none(),
            "Golden case '{}' Full: expected no best candidate",
            case.name
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::CustomSkin;
    use super::*;

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
                hashes: vec!["d94c8962".to_string(), "deadbeef".to_string()],
            },
            DbEntry {
                name: "Ayaka".to_string(),
                tags: vec!["cryo".to_string()],
                object_type: "Character".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
                hashes: vec![],
            },
            DbEntry {
                name: "Nahida".to_string(),
                tags: vec!["dendro".to_string()],
                object_type: "Character".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
                hashes: vec!["aaaa1111".to_string()],
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
}
