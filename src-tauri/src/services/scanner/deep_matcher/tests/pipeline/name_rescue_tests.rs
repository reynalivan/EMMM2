use crate::services::scanner::core::walker::{scan_folder_content, ModCandidate};
use crate::services::scanner::deep_matcher::pipeline::full_pipeline::match_folder_full;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{Confidence, DbEntry, MatchStatus, Reason};
use std::path::PathBuf;
use tempfile::TempDir;

fn candidate_for(path: PathBuf, display_name: &str) -> ModCandidate {
    ModCandidate {
        path,
        raw_name: display_name.to_string(),
        display_name: display_name.to_string(),
        is_disabled: false,
    }
}

#[test]
fn test_case_1_ayaka_ini_substring() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("ayaka");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("ayaka.ini"), "").unwrap();

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "ayaka");
    let db = MasterDb::new(vec![DbEntry {
        name: "Kamisato Ayaka".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: Default::default(),
    }]);

    let result = match_folder_full(
        &candidate,
        &db,
        &content,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    assert!(result.status == MatchStatus::AutoMatched || result.status == MatchStatus::NeedsReview);
    let best = result.best.unwrap();
    assert_eq!(best.name, "Kamisato Ayaka");
    assert!(best
        .reasons
        .iter()
        .any(|r| matches!(r, Reason::SubstringName { .. })));
}

#[test]
fn test_case_2_prefix_numbers_ayaka() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("ayaka");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("prefix213ayaka.ini"), "").unwrap();

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "ayaka");
    let db = MasterDb::new(vec![DbEntry {
        name: "Kamisato Ayaka".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: Default::default(),
    }]);

    let result = match_folder_full(
        &candidate,
        &db,
        &content,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    let best = result
        .best
        .expect("Should match Ayaka due to number stripping");
    assert_eq!(best.name, "Kamisato Ayaka");
}

#[test]
fn test_case_3_folder_rescue_exact() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("anyfiwed12");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("kamisato_ayaka.ini"), "").unwrap();

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "anyfiwed12");
    let db = MasterDb::new(vec![DbEntry {
        name: "Kamisato Ayaka".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: Default::default(),
    }]);

    let result = match_folder_full(
        &candidate,
        &db,
        &content,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    assert_eq!(result.status, MatchStatus::AutoMatched);
    let best = result.best.unwrap();
    assert_eq!(best.confidence, Confidence::Excellent);
}

#[test]
fn test_case_4_different_character_ini() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("ayaka");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("raiden.ini"), "").unwrap();

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "ayaka");
    let db = MasterDb::new(vec![
        DbEntry {
            name: "Kamisato Ayaka".to_string(),
            tags: vec![],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: Default::default(),
        },
        DbEntry {
            name: "Raiden Shogun".to_string(),
            tags: vec![],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hash_db: Default::default(),
        },
    ]);

    let result = match_folder_full(
        &candidate,
        &db,
        &content,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    let best = result.best.unwrap();
    assert_eq!(
        best.name, "Raiden Shogun",
        "Should prioritize raiden.ini over ayaka folder name"
    );
}

#[test]
fn test_case_5_folder_rescue_last_resort() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("kamisatoa");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("aldwdaw.ini"), "").unwrap();

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "kamisatoa");
    let db = MasterDb::new(vec![DbEntry {
        name: "Kamisato Ayaka".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: Default::default(),
    }]);

    let result = match_folder_full(
        &candidate,
        &db,
        &content,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    assert_eq!(result.status, MatchStatus::NeedsReview);
    let best = result.best.unwrap();
    assert_eq!(best.name, "Kamisato Ayaka");
    assert!(best
        .reasons
        .iter()
        .any(|r| matches!(r, Reason::FolderNameRescue { .. })));
}

/// Short terms (< 3 chars) should NOT produce a substring match.
#[test]
fn test_case_6_short_term_rejection() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("ai");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("ai.ini"), "").unwrap();

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "ai");
    let db = MasterDb::new(vec![DbEntry {
        name: "Kamisato Ayaka".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: Default::default(),
    }]);

    let result = match_folder_full(
        &candidate,
        &db,
        &content,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    // "ai" is < 3 chars so should not match via substring
    assert_eq!(result.status, MatchStatus::NoMatch);
}

/// Skipwords (mod, skin, v, etc.) should be stripped during normalization.
#[test]
fn test_case_7_skipword_stripping() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("skin_ayaka_mod_v2");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("ayaka.ini"), "").unwrap();

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "skin_ayaka_mod_v2");
    let db = MasterDb::new(vec![DbEntry {
        name: "Kamisato Ayaka".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: Default::default(),
    }]);

    let result = match_folder_full(
        &candidate,
        &db,
        &content,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    let best = result.best.expect("Should match after skipword stripping");
    assert_eq!(best.name, "Kamisato Ayaka");
}

/// Cross-word-boundary matching: `kamisatoayaka.ini` should match `Kamisato Ayaka`
/// even without separator between the words (GAP 2 fix validation).
/// Folder uses `ayaka` token to ensure the entry is seeded into the candidate pool.
#[test]
fn test_case_8_cross_word_boundary() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("ayaka_data");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("kamisatoayaka.ini"), "").unwrap();

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "ayaka_data");
    let db = MasterDb::new(vec![DbEntry {
        name: "Kamisato Ayaka".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: Default::default(),
    }]);

    let result = match_folder_full(
        &candidate,
        &db,
        &content,
        &Default::default(),
        &Default::default(),
        &Default::default(),
    );
    let best = result.best.expect("Should match via condensed substring");
    assert_eq!(best.name, "Kamisato Ayaka");
    assert!(best
        .reasons
        .iter()
        .any(|r| matches!(r, Reason::SubstringName { .. })));
}
