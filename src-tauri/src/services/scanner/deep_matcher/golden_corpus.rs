//! Golden corpus test harness for matcher threshold tuning.
//!
//! Provides a minimal fixture-driven test system that asserts expected status
//! and best_entry for deterministic matcher behavior validation.

use std::path::PathBuf;

use tempfile::TempDir;

use crate::services::scanner::core::walker::{scan_folder_content, ModCandidate};

use crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;

use super::{match_folder_full, match_folder_quick};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::MatchStatus;

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
    let result_quick = match_folder_quick(
        &candidate,
        db,
        &content,
        &IniTokenizationConfig::default(),
        &AiRerankConfig::default(),
    );
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
    let result_full = match_folder_full(
        &candidate,
        db,
        &content,
        &IniTokenizationConfig::default(),
        &AiRerankConfig::default(),
        &Default::default(),
    );
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
#[path = "tests/golden_corpus_tests.rs"]
mod tests;
