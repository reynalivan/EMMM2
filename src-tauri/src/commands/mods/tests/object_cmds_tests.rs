use super::*;
use crate::services::scanner::deep_matcher::{CustomSkin, DbEntry, MasterDb, MatchStatus};
use tempfile::TempDir;

fn build_entry(name: &str, tags: &[&str], aliases: &[&str]) -> DbEntry {
    let custom_skins = if aliases.is_empty() {
        Vec::new()
    } else {
        vec![CustomSkin {
            name: aliases[0].to_string(),
            aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
            thumbnail_skin_path: None,
            rarity: None,
        }]
    };

    DbEntry {
        name: name.to_string(),
        tags: tags.iter().map(|tag| tag.to_string()).collect(),
        object_type: "Character".to_string(),
        custom_skins,
        thumbnail_path: None,
        metadata: None,
        hash_db: std::collections::HashMap::new(),
    }
}

#[test]
fn test_object_command_auto_matched_uses_staged_status_adapters() {
    let db = MasterDb::new(vec![build_entry(
        "Raiden Shogun",
        &["raiden"],
        &["Raiden Wish"],
    )]);
    let match_result = match_object_with_staged_pipeline(&db, "Raiden Wish");
    let temp = TempDir::new().expect("temp dir");

    let item = build_matched_db_entry_from_staged(temp.path(), &db, &match_result)
        .expect("expected staged auto match payload");

    assert_eq!(match_result.status, MatchStatus::AutoMatched);
    assert_eq!(item.name, "Raiden Shogun");
    assert_eq!(item.match_level, "AutoMatched");
    assert_eq!(item.match_confidence, "High");
    assert_eq!(
        item.match_detail,
        "Auto-matched via exact alias match ('Raiden Wish')"
    );
}

#[test]
fn test_object_command_needs_review_keeps_candidate_with_low_confidence_label() {
    let db = MasterDb::new(vec![
        build_entry("Amber", &["sunset"], &["Sunset"]),
        build_entry("Lisa", &["sunset"], &["Sunset"]),
    ]);
    let match_result = match_object_with_staged_pipeline(&db, "Sunset Pack");
    let temp = TempDir::new().expect("temp dir");

    let item = build_matched_db_entry_from_staged(temp.path(), &db, &match_result)
        .expect("expected review payload");

    assert_eq!(match_result.status, MatchStatus::NeedsReview);
    assert_eq!(item.match_level, "NeedsReview");
    assert_eq!(item.match_confidence, "Low");
    assert_eq!(
        item.match_detail,
        "Ambiguous top matches: Amber vs Lisa"
    );
}

#[test]
fn test_object_command_has_no_active_fuzzy_fallback() {
    let db = MasterDb::new(vec![build_entry("Albedo", &[], &[])]);
    let match_result = match_object_with_staged_pipeline(&db, "Albato");
    let temp = TempDir::new().expect("temp dir");

    assert_eq!(match_result.status, MatchStatus::NoMatch);
    assert!(build_matched_db_entry_from_staged(temp.path(), &db, &match_result).is_none());
}
