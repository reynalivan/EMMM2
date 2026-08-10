use std::collections::HashMap;

use super::attach_user_aliases;
use crate::services::scanner::core::walker::{FolderContent, ModCandidate};
use crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::match_folder_phased;
use crate::services::scanner::deep_matcher::models::types::{CustomSkin, DbEntry, MatchStatus};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::sync::helpers::canonical_entry_key;

/// "Beelzebul" is a real nickname for Raiden Shogun and shares no substring
/// with the entry name, so nothing but an alias can bridge the two.
const NICKNAME: &str = "Beelzebul";

fn entry(name: &str, aliases: Vec<&str>) -> DbEntry {
    DbEntry {
        name: name.to_string(),
        tags: vec!["electro".to_string()],
        object_type: "Character".to_string(),
        custom_skins: if aliases.is_empty() {
            vec![]
        } else {
            vec![CustomSkin {
                name: "Bundled".to_string(),
                aliases: aliases.into_iter().map(str::to_string).collect(),
                thumbnail_skin_path: None,
                rarity: None,
            }]
        },
        thumbnail_path: None,
        metadata: None,
        hash_db: HashMap::new(),
    }
}

fn user_aliases(name: &str, aliases: &[&str]) -> HashMap<String, Vec<String>> {
    HashMap::from([(
        canonical_entry_key(name),
        aliases.iter().map(|a| a.to_string()).collect(),
    )])
}

fn match_folder(
    db: &MasterDb,
    folder: &str,
) -> crate::services::scanner::deep_matcher::StagedMatchResult {
    let candidate = ModCandidate {
        path: format!("mods/{folder}").into(),
        raw_name: folder.to_string(),
        display_name: folder.to_string(),
        is_disabled: false,
    };
    let content = FolderContent {
        subfolder_names: vec![],
        files: vec![],
        ini_files: vec![],
    };
    match_folder_phased(
        &candidate,
        db,
        &content,
        &IniTokenizationConfig::default().prepare(),
        &AiRerankConfig::default(),
    )
}

/// The point of the whole change: an alias the user typed decides a match that
/// would otherwise be a NoMatch. Asserts both halves so the test fails loudly
/// if the nickname ever starts matching for some unrelated reason.
#[test]
fn user_alias_turns_a_no_match_into_a_match() {
    let baseline = MasterDb::new(vec![entry("Raiden Shogun", vec![])]);
    assert_eq!(
        match_folder(&baseline, NICKNAME).status,
        MatchStatus::NoMatch,
        "without the user alias this folder must not match"
    );

    let mut db = MasterDb::new(vec![entry("Raiden Shogun", vec![])]);
    attach_user_aliases(&mut db, &user_aliases("Raiden Shogun", &[NICKNAME]));

    let result = match_folder(&db, NICKNAME);
    assert_ne!(result.status, MatchStatus::NoMatch);
    assert_eq!(
        result.best.expect("a candidate").name,
        "Raiden Shogun",
        "the user alias should route the folder to its entry"
    );
}

#[test]
fn attach_skips_aliases_the_bundled_db_already_has() {
    let mut db = MasterDb::new(vec![entry("Raiden Shogun", vec!["raidenshogun"])]);
    attach_user_aliases(
        &mut db,
        // Differing case and padding must still count as "already known".
        &user_aliases("Raiden Shogun", &["  RaidenShogun  ", "ei"]),
    );

    let user_skin = db.entries[0]
        .custom_skins
        .iter()
        .find(|skin| skin.name == "User")
        .expect("a user skin for the fresh alias");
    assert_eq!(user_skin.aliases, vec!["ei".to_string()]);
}

#[test]
fn attach_adds_nothing_when_every_alias_is_a_duplicate() {
    let mut db = MasterDb::new(vec![entry("Raiden Shogun", vec!["raidenshogun"])]);
    attach_user_aliases(&mut db, &user_aliases("Raiden Shogun", &["raidenshogun"]));

    assert_eq!(db.entries[0].custom_skins.len(), 1);
    assert_eq!(db.entries[0].custom_skins[0].name, "Bundled");
}

/// An alias whose entry key matches nothing bundled is dropped rather than
/// creating a phantom entry — the matcher can only return bundled entries.
#[test]
fn attach_drops_aliases_for_unknown_entry_keys() {
    let mut db = MasterDb::new(vec![entry("Raiden Shogun", vec![])]);
    attach_user_aliases(&mut db, &user_aliases("Some Deleted Object", &["ghost"]));

    assert!(db.entries[0].custom_skins.is_empty());
}
