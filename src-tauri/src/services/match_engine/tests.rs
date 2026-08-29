use super::destination::{resolve_destination_candidates, DestinationContext, ExistingDestination};
use super::inspection::{inspect_source, InspectionRequest};
use super::types::CanonicalIdentity;
use crate::services::import_batch::types::{DestinationKind, StableCategory};

#[test]
fn inspection_strips_disabled_and_limits_content_to_three_levels() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("DISABLED 라이덴32114");
    std::fs::create_dir_all(source.join("data/inner/third/fourth")).unwrap();
    std::fs::write(
        source.join("data/inner/Raiden.ini"),
        "[TextureOverrideRaiden]",
    )
    .unwrap();
    std::fs::write(
        source.join("data/inner/third/fourth/ignored.ini"),
        "ignored",
    )
    .unwrap();
    std::fs::write(source.join("data/readme.txt"), "ignored").unwrap();

    let inspected = inspect_source(&InspectionRequest {
        source_path: source,
        planned_name: None,
        match_extensions: vec!["ini".to_string(), "dds".to_string()],
    })
    .unwrap();

    assert_eq!(inspected.source_name, "라이덴32114");
    assert!(
        inspected.normalized_name.is_ascii() && inspected.normalized_name.starts_with("rai"),
        "unexpected transliteration: {}",
        inspected.normalized_name
    );
    assert!(inspected
        .matching_files
        .iter()
        .any(|path| path.ends_with("Raiden.ini")));
    assert!(!inspected
        .matching_files
        .iter()
        .any(|path| path.ends_with("ignored.ini")));
    assert_eq!(inspected.fingerprint.file_count, 2);
}

#[test]
fn folder_name_resolution_handles_required_examples_even_for_other_category() {
    let existing = vec![
        ExistingDestination::new(
            "ayaka",
            "Ayaka",
            "Ayaka",
            None,
            vec![],
            StableCategory::Character,
        ),
        ExistingDestination::new(
            "raiden",
            "Raiden Shogun",
            "Raiden Shogun",
            None,
            vec!["Raiden", "Shogun"],
            StableCategory::Character,
        ),
        ExistingDestination::new(
            "hutao",
            "Hutao",
            "Hutao",
            None,
            vec!["Hu Tao"],
            StableCategory::Character,
        ),
    ];

    for (source, expected) in [
        ("DISABLED ayaka-12319mods", "ayaka"),
        ("DISABLED raiden32114", "raiden"),
        ("DISABLED shogun32114", "raiden"),
        ("DISABLED aw31hutao123-81", "hutao"),
    ] {
        let suggestions = resolve_destination_candidates(DestinationContext {
            source_name: source,
            category: StableCategory::Other,
            specific_target: None,
            existing: &existing,
            canonical: None,
            mods_root: "C:/Mods",
            enforce_category: false,
        });
        assert_eq!(suggestions[0].object_id.as_deref(), Some(expected));
        assert_eq!(suggestions[0].kind, DestinationKind::ExistingObject);
    }
}

#[test]
fn no_db_match_does_not_invent_an_other_folder() {
    let suggestions = resolve_destination_candidates(DestinationContext {
        source_name: "unknown-mod",
        category: StableCategory::Other,
        specific_target: None,
        existing: &[],
        canonical: None,
        mods_root: "C:/Mods",
        enforce_category: true,
    });
    assert!(suggestions.is_empty());
}

#[test]
fn create_folder_is_offered_only_for_canonical_identity() {
    let taxonomy = CanonicalIdentity {
        entry_key: "weapon".to_string(),
        name: "Weapon".to_string(),
        entry_kind: crate::services::scanner::deep_matcher::EntryKind::Taxonomy,
    };
    let canonical = CanonicalIdentity {
        entry_key: "ayaka".to_string(),
        name: "Ayaka".to_string(),
        entry_kind: crate::services::scanner::deep_matcher::EntryKind::Canonical,
    };

    let taxonomy_result = resolve_destination_candidates(DestinationContext {
        source_name: "weapon skin",
        category: StableCategory::Weapon,
        specific_target: None,
        existing: &[],
        canonical: Some(&taxonomy),
        mods_root: "C:/Mods",
        enforce_category: true,
    });
    assert!(taxonomy_result.is_empty());

    let canonical_result = resolve_destination_candidates(DestinationContext {
        source_name: "ayaka skin",
        category: StableCategory::Character,
        specific_target: None,
        existing: &[],
        canonical: Some(&canonical),
        mods_root: "C:/Mods",
        enforce_category: true,
    });
    assert_eq!(canonical_result[0].kind, DestinationKind::CreateCanonical);
    assert!(canonical_result[0].target_path.ends_with("Ayaka"));
}

#[test]
fn import_destination_matching_excludes_existing_objects_from_another_category() {
    let existing = vec![ExistingDestination::new(
        "weapon-ayaka",
        "Ayaka",
        "Ayaka",
        None,
        vec![],
        StableCategory::Weapon,
    )];

    let suggestions = resolve_destination_candidates(DestinationContext {
        source_name: "DISABLED ayaka skin",
        category: StableCategory::Character,
        specific_target: None,
        existing: &existing,
        canonical: None,
        mods_root: "C:/Mods",
        enforce_category: true,
    });

    assert!(suggestions.is_empty());
}

#[test]
fn specific_target_with_another_category_is_kept_with_an_explicit_warning() {
    let target = ExistingDestination::new(
        "weapon-ayaka",
        "Ayaka",
        "Ayaka",
        None,
        vec![],
        StableCategory::Weapon,
    );

    let suggestions = resolve_destination_candidates(DestinationContext {
        source_name: "DISABLED ayaka skin",
        category: StableCategory::Character,
        specific_target: Some(&target),
        existing: std::slice::from_ref(&target),
        canonical: None,
        mods_root: "C:/Mods",
        enforce_category: true,
    });

    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].kind, DestinationKind::SpecificTarget);
    assert!(suggestions[0]
        .warning
        .as_deref()
        .is_some_and(|warning| warning.contains("differs")));
}
