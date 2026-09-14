//! Regression tests for geometry-first KeyViewer sentinel selection.

use std::collections::{HashMap, HashSet};

use crate::modules::automation::application::keyviewer::matcher::{
    match_objects, KvObjectEntry, MatchConfig,
};
use crate::modules::matching::application::deep_matcher::models::types::{
    RuntimeResourceKind, RuntimeTarget, RuntimeTargetProvenance,
};

fn target(hash: &str, resource_kind: RuntimeResourceKind, slot: Option<&str>) -> RuntimeTarget {
    RuntimeTarget {
        variant: "Default".to_string(),
        component: None,
        resource_kind,
        hash: hash.to_string(),
        slot: slot.map(str::to_string),
        match_first_index: None,
        provenance: RuntimeTargetProvenance {
            source_repo: "fixture".to_string(),
            commit: "abc123".to_string(),
            path: "hash.json".to_string(),
        },
    }
}

fn entry(name: &str, runtime_targets: Vec<RuntimeTarget>) -> KvObjectEntry {
    let code_hashes = runtime_targets
        .iter()
        .map(|target| target.hash.to_ascii_lowercase())
        .collect();
    KvObjectEntry {
        name: name.to_string(),
        object_type: "Character".to_string(),
        code_hashes,
        skin_hashes: HashMap::new(),
        runtime_targets,
        tags: Vec::new(),
        thumbnail_path: None,
    }
}

fn active(hashes: &[&str]) -> HashSet<String> {
    hashes.iter().map(|hash| hash.to_string()).collect()
}

#[test]
fn arlecchino_prefers_position_over_ib_and_face_texture_regardless_of_order() {
    let entries = vec![entry(
        "Arlecchino",
        vec![
            target("a44625da", RuntimeResourceKind::Texture, Some("ps-t0")),
            target("e811d2a1", RuntimeResourceKind::IndexBuffer, Some("ib")),
            target("6895f405", RuntimeResourceKind::PositionVb, Some("vb0")),
        ],
    )];

    let results = match_objects(
        &entries,
        &active(&["a44625da", "e811d2a1", "6895f405"]),
        &HashMap::new(),
        &MatchConfig::default(),
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].sentinels.len(), 1);
    assert_eq!(results[0].sentinels[0].hash, "6895f405");
    assert_eq!(
        results[0].sentinels[0].resource_kind,
        RuntimeResourceKind::PositionVb
    );
}

#[test]
fn boss_keeps_all_position_targets_as_one_or_group() {
    let entries = vec![entry(
        "Arlecchino Boss",
        vec![
            target("55c132a8", RuntimeResourceKind::PositionVb, Some("vb0")),
            target("725e36bd", RuntimeResourceKind::PositionVb, Some("vb0")),
            target("e811d2a1", RuntimeResourceKind::IndexBuffer, Some("ib")),
        ],
    )];

    let results = match_objects(
        &entries,
        &active(&["55c132a8", "725e36bd", "e811d2a1"]),
        &HashMap::new(),
        &MatchConfig::default(),
    );

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0]
            .sentinels
            .iter()
            .map(|sentinel| sentinel.hash.as_str())
            .collect::<Vec<_>>(),
        vec!["55c132a8", "725e36bd"]
    );
}

#[test]
fn face_only_mod_uses_texture_when_no_geometry_hash_is_present() {
    let entries = vec![entry(
        "Arlecchino",
        vec![
            target("6895f405", RuntimeResourceKind::PositionVb, Some("vb0")),
            target("a44625da", RuntimeResourceKind::Texture, Some("ps-t0")),
        ],
    )];

    let results = match_objects(
        &entries,
        &active(&["a44625da"]),
        &HashMap::new(),
        &MatchConfig::default(),
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].sentinels[0].hash, "a44625da");
    assert_eq!(
        results[0].sentinels[0].resource_kind,
        RuntimeResourceKind::Texture
    );
}

#[test]
fn index_buffer_without_draw_context_is_not_a_runtime_sentinel() {
    let entries = vec![entry(
        "Index Only",
        vec![target(
            "e811d2a1",
            RuntimeResourceKind::IndexBuffer,
            Some("ib"),
        )],
    )];

    let results = match_objects(
        &entries,
        &active(&["e811d2a1"]),
        &HashMap::new(),
        &MatchConfig::default(),
    );

    assert!(results.is_empty());
}

#[test]
fn shared_geometry_hash_does_not_choose_an_arbitrary_character_panel() {
    let entries = vec![
        entry(
            "Character A",
            vec![target(
                "shared00",
                RuntimeResourceKind::PositionVb,
                Some("vb0"),
            )],
        ),
        entry(
            "Character B",
            vec![target(
                "shared00",
                RuntimeResourceKind::PositionVb,
                Some("vb0"),
            )],
        ),
    ];

    let results = match_objects(
        &entries,
        &active(&["shared00"]),
        &HashMap::new(),
        &MatchConfig::default(),
    );

    assert!(results.is_empty());
}

#[test]
fn shader_targets_are_never_selected_for_keyviewer() {
    let entries = vec![entry(
        "Shader Only",
        vec![target(
            "12345678abcdef00",
            RuntimeResourceKind::Shader,
            None,
        )],
    )];

    let results = match_objects(
        &entries,
        &active(&["12345678abcdef00"]),
        &HashMap::new(),
        &MatchConfig::default(),
    );

    assert!(results.is_empty());
}
