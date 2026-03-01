use crate::services::scanner::core::walker::{FolderContent, ModCandidate};
use crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::models::types::{CustomSkin, DbEntry, MatchStatus};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{match_folder_phased, Confidence};

fn build_test_db() -> MasterDb {
    MasterDb::new(vec![
        DbEntry {
            name: "Raiden Shogun".to_string(),
            tags: vec!["electro".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![CustomSkin {
                name: "Wish".to_string(),
                aliases: vec!["raidenwish".to_string(), "raidenshogun".to_string()],
                thumbnail_skin_path: None,
                rarity: None,
            }],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::new(),
        },
        DbEntry {
            name: "Hu Tao".to_string(),
            tags: vec!["pyro".to_string(), "polearm".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![CustomSkin {
                name: "Default".to_string(),
                aliases: vec!["hutao".to_string(), "director".to_string()],
                thumbnail_skin_path: None,
                rarity: None,
            }],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::new(),
        },
        DbEntry {
            name: "Traveler (Anemo)".to_string(),
            tags: vec!["anemo".to_string(), "sword".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![
                CustomSkin {
                    name: "Lumine".to_string(),
                    aliases: vec!["lumine".to_string()],
                    thumbnail_skin_path: None,
                    rarity: None,
                },
                CustomSkin {
                    name: "Aether".to_string(),
                    aliases: vec!["aether".to_string()],
                    thumbnail_skin_path: None,
                    rarity: None,
                },
            ],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::new(),
        },
        DbEntry {
            name: "Traveler (Geo)".to_string(),
            tags: vec!["geo".to_string(), "sword".to_string()],
            object_type: "Character".to_string(),
            custom_skins: vec![
                CustomSkin {
                    name: "Lumine".to_string(),
                    aliases: vec!["lumine".to_string()],
                    thumbnail_skin_path: None,
                    rarity: None,
                },
                CustomSkin {
                    name: "Aether".to_string(),
                    aliases: vec!["aether".to_string()],
                    thumbnail_skin_path: None,
                    rarity: None,
                },
            ],
            thumbnail_path: None,
            metadata: None,
            hash_db: std::collections::HashMap::new(),
        },
    ])
}

// Covers: TC-09-05 (Dictionary Match performance/accuracy)
// Ensures "dirty" folder names containing an alias are correctly mapped to standard catalog targets securely and cleanly.
#[test]
fn test_alias_resolution_dirty_input_name() {
    let db = build_test_db();
    let ini_config = IniTokenizationConfig::default();
    let ai_config = AiRerankConfig::default();

    let candidate = ModCandidate {
        path: "mods/RaidenShogun_Mod_v2_By_Modder_[Busty]".into(),
        raw_name: "RaidenShogun_Mod_v2_By_Modder_[Busty]".to_string(), // Dirty input
        display_name: "RaidenShogun_Mod_v2_By_Modder_[Busty]".to_string(),
        is_disabled: false,
    };

    let content = FolderContent {
        subfolder_names: vec!["raidenshogun".to_string()], // Typical dirty subfolder string
        files: vec![],
        ini_files: vec![], // No INI or hashes to force alias substring rescue mapping
    };

    let result = match_folder_phased(&candidate, &db, &content, &ini_config, &ai_config);

    // Should rescue via substring/alias matching
    assert!(
        result.status == MatchStatus::NeedsReview || result.status == MatchStatus::AutoMatched,
        "Expected match, got {:?}",
        result.status
    );
    let best = result.best.expect("Should have a best candidate");
    assert_eq!(best.name, "Raiden Shogun");

    // Confidence should be at least Medium from substring/alias logic
    assert!(
        best.confidence == Confidence::Medium
            || best.confidence == Confidence::High
            || best.confidence == Confidence::Excellent,
        "Expected at least Medium confidence via alias resolution, got {:?}",
        best.confidence
    );
}

// Covers: TC-09-08 (Stopword parsing stripping)
// Validates stop words are implicitly handled and don't ruin alias detection.
#[test]
fn test_alias_resolution_stopword_parsing_strip() {
    let db = build_test_db();
    let ini_config = IniTokenizationConfig::default();
    let ai_config = AiRerankConfig::default();

    let candidate = ModCandidate {
        path: "mods/Hu Tao v2 By Modder [Busty]".into(),
        raw_name: "Hu Tao v2 By Modder [Busty]".to_string(), // Folder contains stopwords
        display_name: "Hu Tao v2 By Modder [Busty]".to_string(),
        is_disabled: false,
    };

    let content = FolderContent {
        subfolder_names: vec![],
        files: vec![],
        ini_files: vec![],
    };

    let result = match_folder_phased(&candidate, &db, &content, &ini_config, &ai_config);

    // Root folder rescue ("Hu Tao" stripped of stopwords) -> Should map to Hu Tao
    assert_ne!(result.status, MatchStatus::NoMatch, "Should not be NoMatch");

    let best = result.best.expect("Should have a best candidate");
    assert_eq!(best.name, "Hu Tao");
}

// Covers: TC-09-07 (Aliasing Conflict Traps)
// "Traveler" string shouldn't mechanically map to "Lumine" directly without review if ambiguous.
#[test]
fn test_alias_resolution_conflict_traps() {
    let db = build_test_db();
    let ini_config = IniTokenizationConfig::default();
    let ai_config = AiRerankConfig::default();

    let candidate = ModCandidate {
        path: "mods/Traveler".into(),
        raw_name: "Traveler".to_string(),
        display_name: "Traveler".to_string(),
        is_disabled: false,
    };

    let content = FolderContent {
        subfolder_names: vec!["lumine".to_string(), "aether".to_string()], // Conflict tokens
        files: vec![],
        ini_files: vec![],
    };

    let result = match_folder_phased(&candidate, &db, &content, &ini_config, &ai_config);

    // It should map to Traveler (Anemo) with NeedsReview
    assert_eq!(result.status, MatchStatus::NeedsReview);
    let best = result.best.expect("Should have best candidate");
    assert_eq!(best.name, "Traveler (Anemo)");
}
