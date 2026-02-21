use std::path::PathBuf;

use tempfile::TempDir;

use crate::services::scanner::walker::{scan_folder_content, ModCandidate};

use super::super::content::IniTokenizationConfig;
use super::super::content::FULL_MAX_INI_FILES;
use super::super::types::{CustomSkin, DbEntry, MatchStatus, Reason};
use super::super::MasterDb;
use super::match_folder_full;

fn candidate_for(path: PathBuf, display_name: &str) -> ModCandidate {
    ModCandidate {
        path,
        raw_name: display_name.to_string(),
        display_name: display_name.to_string(),
        is_disabled: false,
    }
}

// Covers: TC-2.2-Task12-01
#[test]
fn test_full_alias_recheck_rescues_match_after_deep_ini_collection() {
    let temp = TempDir::new().expect("temp dir");
    let folder = temp.path().join("target_mystery_mod_pack");
    std::fs::create_dir_all(&folder).expect("create folder");
    std::fs::write(
        folder.join("mod.ini"),
        "[TextureOverrideX]\nfilename = Textures/radiantshadow/body_diffuse.dds\n",
    )
    .expect("write ini");

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "Mystery Pack");
    let db = MasterDb::new(vec![
        DbEntry {
            name: "Target Hero".to_string(),
            tags: vec![],
            object_type: "Character".to_string(),
            custom_skins: vec![CustomSkin {
                name: "Radiant Shadow".to_string(),
                aliases: vec!["radiantshadow".to_string()],
                thumbnail_skin_path: None,
                rarity: None,
            }],
            thumbnail_path: None,
            metadata: None,
            hashes: vec![],
        },
        DbEntry {
            name: "Control Hero".to_string(),
            tags: vec![],
            object_type: "Character".to_string(),
            custom_skins: vec![],
            thumbnail_path: None,
            metadata: None,
            hashes: vec![],
        },
    ]);

    let result = match_folder_full(&candidate, &db, &content, &IniTokenizationConfig::default());

    assert_eq!(result.status, MatchStatus::AutoMatched);
    let best = result.best.expect("best candidate");
    assert_eq!(best.name, "Target Hero");
    assert!(best
        .reasons
        .iter()
        .any(|reason| matches!(reason, Reason::AliasStrict { .. })));
}

// Covers: TC-2.2-Task12-02
#[test]
fn test_full_budget_overflow_keeps_partial_signals_and_continues_matching() {
    let temp = TempDir::new().expect("temp dir");
    let folder = temp.path().join("full_budget_pipeline_mod");
    std::fs::create_dir_all(&folder).expect("create folder");

    let payload = "x".repeat(220 * 1024);
    for idx in 1..=12 {
        let ini = format!(
            "[TextureOverrideBudget{idx}]\nfilename = Characters/arlecchino/body_diffuse.dds\n;{payload}\n"
        );
        std::fs::write(folder.join(format!("{idx:02}.ini")), ini).expect("write ini");
    }
    std::fs::write(folder.join("arlecchino_mesh.buf"), "dummy").expect("write mesh");

    let content = scan_folder_content(&folder, 3);
    let candidate = candidate_for(folder.clone(), "Budget Stress Folder");
    let db = MasterDb::new(vec![DbEntry {
        name: "Arlecchino".to_string(),
        tags: vec![],
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hashes: vec![],
    }]);

    let result = match_folder_full(&candidate, &db, &content, &IniTokenizationConfig::default());

    assert!(matches!(
        result.status,
        MatchStatus::AutoMatched | MatchStatus::NeedsReview | MatchStatus::NoMatch
    ));
    assert!(result.evidence.scanned_ini_files < FULL_MAX_INI_FILES);
    assert_eq!(result.evidence.scanned_ini_files, 5);
    assert!(result.evidence.scanned_name_items > 0);
}

// Covers: TC-2.2-Task15-05
#[test]
fn test_full_pipeline_has_no_fuzzy_fallback_for_near_name_only() {
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
        hashes: vec![],
    }]);

    let result = match_folder_full(&candidate, &db, &content, &IniTokenizationConfig::default());

    assert_eq!(result.status, MatchStatus::NoMatch);
    assert!(result.best.is_none());
    assert!(result.candidates_topk.is_empty());
}
