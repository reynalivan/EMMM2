use std::path::PathBuf;

use tempfile::TempDir;

use crate::services::scanner::core::walker::{scan_folder_content, ModCandidate};

use crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;

use super::match_folder_quick;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{CustomSkin, DbEntry, MatchStatus, Reason};

fn candidate_for(path: PathBuf, display_name: &str) -> ModCandidate {
    ModCandidate {
        path,
        raw_name: display_name.to_string(),
        display_name: display_name.to_string(),
        is_disabled: false,
    }
}

fn quick_test_db() -> MasterDb {
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
                vec!["d94c8962".to_string()],
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
    ])
}

// Covers: TC-2.2-Task11-01
#[test]
fn test_quick_hash_stage_accepts_early_with_unique_hash() {
    let temp = TempDir::new().expect("temp dir");
    let folder = temp.path().join("mystery_mod");
    std::fs::create_dir_all(&folder).expect("create folder");
    std::fs::write(
        folder.join("mod.ini"),
        "[TextureOverrideRaiden]\nhash = d94c8962\n",
    )
    .expect("write ini");

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "Unknown Mystery Pack");
    let db = quick_test_db();

    let result = match_folder_quick(
        &candidate,
        &db,
        &content,
        &IniTokenizationConfig::default(),
        &AiRerankConfig::default(),
    );

    assert_eq!(result.status, MatchStatus::AutoMatched);
    let best = result.best.expect("best candidate");
    assert_eq!(best.name, "Raiden Shogun");
    assert!(best
        .reasons
        .iter()
        .any(|reason| matches!(reason, Reason::HashOverlap { .. })));
}

// Covers: TC-2.2-Task11-02
#[test]
fn test_quick_direct_name_support_only_does_not_auto_match() {
    let temp = TempDir::new().expect("temp dir");
    let folder = temp.path().join("raiden");
    std::fs::create_dir_all(&folder).expect("create folder");

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "raiden");
    let db = MasterDb::new(vec![DbEntry {
        name: "Raiden".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: std::collections::HashMap::new(),
    }]);

    let result = match_folder_quick(
        &candidate,
        &db,
        &content,
        &IniTokenizationConfig::default(),
        &AiRerankConfig::default(),
    );

    assert_ne!(result.status, MatchStatus::AutoMatched);
    let best = result.best.expect("best candidate present");
    assert!(best
        .reasons
        .iter()
        .any(|reason| matches!(reason, Reason::DirectNameSupport { .. })));
}

// Covers: TC-2.2-Task15-04
#[test]
fn test_quick_pipeline_has_no_fuzzy_fallback_for_near_name_only() {
    let temp = TempDir::new().expect("temp dir");
    let folder = temp.path().join("albato");
    std::fs::create_dir_all(&folder).expect("create folder");

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "Albato");
    let db = MasterDb::new(vec![DbEntry {
        name: "Albedo".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: std::collections::HashMap::new(),
    }]);

    let result = match_folder_quick(
        &candidate,
        &db,
        &content,
        &IniTokenizationConfig::default(),
        &AiRerankConfig::default(),
    );

    assert_eq!(result.status, MatchStatus::NoMatch);
    assert!(result.best.is_none());
    assert!(result.candidates_topk.is_empty());
}
