use std::collections::HashMap;

use super::attach_user_aliases;
use crate::modules::matching::application::deep_matcher::analysis::ai_rerank::AiRerankConfig;
use crate::modules::matching::application::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::modules::matching::application::deep_matcher::match_folder_phased;
use crate::modules::matching::application::deep_matcher::models::types::{
    CustomSkin, DbEntry, MatchStatus,
};
use crate::modules::matching::application::deep_matcher::state::master_db::MasterDb;
use crate::modules::workspace::application::scanner::core::walker::{FolderContent, ModCandidate};
use crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key;

/// "Beelzebul" is a real nickname for Raiden Shogun and shares no substring
/// with the entry name, so nothing but an alias can bridge the two.
const NICKNAME: &str = "Beelzebul";

fn entry(name: &str, aliases: Vec<&str>) -> DbEntry {
    DbEntry {
        name: name.to_string(),
        aliases: vec!["electro".to_string()],
        object_type: "Character".to_string(),
        entry_kind: Default::default(),
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
        runtime_targets: vec![],
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
) -> crate::modules::matching::application::deep_matcher::StagedMatchResult {
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

    let mut entries = vec![entry("Raiden Shogun", vec![])];
    attach_user_aliases(&mut entries, &user_aliases("Raiden Shogun", &[NICKNAME]));
    let db = MasterDb::new(entries);

    let result = match_folder(&db, NICKNAME);
    assert_ne!(result.status, MatchStatus::NoMatch);
    assert_eq!(
        result.best.expect("a candidate").name,
        "Raiden Shogun",
        "the user alias should route the folder to its entry"
    );
}

#[test]
fn mmr_attached_user_alias_enters_the_candidate_index() {
    let mut entries = vec![entry("Ayaka", vec![]), entry("Moonlit", vec![])];
    attach_user_aliases(&mut entries, &user_aliases("Ayaka", &["Moonlit Princess"]));
    let db = MasterDb::new(entries);
    assert!(db.keywords[0].1.contains("princess"));
    assert_eq!(db.indexes.token_index.get("princess"), Some(&vec![0]));
}

#[test]
fn attach_skips_aliases_the_bundled_db_already_has() {
    let mut entries = vec![entry("Raiden Shogun", vec!["raidenshogun"])];
    attach_user_aliases(
        &mut entries,
        // Differing case and padding must still count as "already known".
        &user_aliases("Raiden Shogun", &["  RaidenShogun  ", "ei"]),
    );

    let user_skin = entries[0]
        .custom_skins
        .iter()
        .find(|skin| skin.name == "User")
        .expect("a user skin for the fresh alias");
    assert_eq!(user_skin.aliases, vec!["ei".to_string()]);
}

#[test]
fn attach_adds_nothing_when_every_alias_is_a_duplicate() {
    let mut entries = vec![entry("Raiden Shogun", vec!["raidenshogun"])];
    attach_user_aliases(
        &mut entries,
        &user_aliases("Raiden Shogun", &["raidenshogun"]),
    );

    assert_eq!(entries[0].custom_skins.len(), 1);
    assert_eq!(entries[0].custom_skins[0].name, "Bundled");
}

/// An alias whose entry key matches nothing bundled is dropped rather than
/// creating a phantom entry â€” the matcher can only return bundled entries.
#[test]
fn attach_drops_aliases_for_unknown_entry_keys() {
    let mut entries = vec![entry("Raiden Shogun", vec![])];
    attach_user_aliases(
        &mut entries,
        &user_aliases("Some Deleted Object", &["ghost"]),
    );

    assert!(entries[0].custom_skins.is_empty());
}

#[tokio::test]
async fn invalidation_during_load_retries_instead_of_publishing_stale_entries() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let cache = Arc::new(super::MasterDbCache::default());
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let calls = Arc::new(AtomicUsize::new(0));
    let loading = {
        let cache = Arc::clone(&cache);
        let started = Arc::clone(&started);
        let release = Arc::clone(&release);
        let calls = Arc::clone(&calls);
        tokio::spawn(async move {
            cache
                .get_or_load("gimi", || async {
                    let call = calls.fetch_add(1, Ordering::SeqCst);
                    if call == 0 {
                        started.notify_one();
                        release.notified().await;
                    }
                    Ok(MasterDb::new(vec![entry(
                        if call == 0 { "Old" } else { "New" },
                        vec![],
                    )]))
                })
                .await
                .unwrap()
        })
    };
    started.notified().await;
    cache.clear().await;
    release.notify_one();
    let result = loading.await.unwrap();
    assert_eq!(result.entries[0].name, "New");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let cached = cache
        .get_or_load("gimi", || async { panic!("must reuse the fresh cache") })
        .await
        .unwrap();
    assert!(Arc::ptr_eq(&result, &cached));
}

#[tokio::test]
async fn user_aliases_are_scoped_to_the_requested_game_type() {
    use crate::modules::games::domain::models::GameType;
    use crate::test_utils::{
        insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture,
    };

    let pool = crate::test_utils::init_test_db().await.pool;
    for (id, game_type) in [("gimi-game", GameType::Gimi), ("zzmi-game", GameType::Zzmi)] {
        let object_id = format!("{id}-object");
        let folder_path = format!("/mods/{id}");
        insert_test_game(
            &pool,
            &TestGameFixture {
                id,
                name: id,
                game_type,
                path: "/game",
                mods_path: Some("/mods"),
            },
        )
        .await
        .unwrap();
        insert_test_object(
            &pool,
            &TestObjectFixture {
                id: &object_id,
                game_id: id,
                name: "Shared Entry",
                folder_path: &folder_path,
                object_type: "Character",
            },
        )
        .await
        .unwrap();
    }
    sqlx::query(
        "UPDATE objects
         SET matched_entry_key = 'shared-entry',
             custom_skins = CASE game_id
                 WHEN 'gimi-game' THEN '[{\"name\":\"User\",\"aliases\":[\"gimi-alias\"]}]'
                 ELSE '[{\"name\":\"User\",\"aliases\":[\"zzmi-alias\"]}]'
             END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let aliases = super::load_user_aliases(&pool, GameType::Gimi as i32).await;

    assert_eq!(
        aliases.get("shared-entry"),
        Some(&vec!["gimi-alias".to_string()])
    );
}
