//! Unit tests for the file generation pipeline.

use std::collections::HashMap;
use tempfile::TempDir;

use crate::modules::automation::application::keyviewer::generator::{
    atomic_write, create_staging_directory, discover_reload_key, generate_keybind_text,
    generate_keyviewer_ini as generate_keyviewer_ini_for_game, generate_status_text,
    replace_directory, resolve_d3dx_ini_path, write_keybind_files, write_status_file,
    SourceKeyBinding, StatusFields,
};
use crate::modules::automation::application::keyviewer::matcher::{
    MatchConfidence, MatchResult, RuntimeSentinel, RuntimeSentinelSource,
};
use crate::modules::library::application::ini::document::KeyBinding;
use crate::modules::matching::application::deep_matcher::models::types::RuntimeResourceKind;

fn make_keybinding(section: &str, key: Option<&str>, back: Option<&str>) -> KeyBinding {
    KeyBinding {
        section_name: section.to_string(),
        key: key.map(|s| s.to_string()),
        back: back.map(|s| s.to_string()),
        binding_type: None,
        condition: None,
        key_line_idx: None,
        back_line_idx: None,
    }
}

fn make_match_result(name: &str, sentinels: &[&str]) -> MatchResult {
    MatchResult {
        object_name: name.to_string(),
        object_type: "Character".to_string(),
        score: 50.0,
        matched_hashes: sentinels.iter().map(|s| s.to_string()).collect(),
        sentinels: sentinels
            .iter()
            .map(|hash| RuntimeSentinel {
                hash: (*hash).to_string(),
                resource_kind: RuntimeResourceKind::PositionVb,
                callback_slot: "vb0".to_string(),
                match_first_index: None,
                source: RuntimeSentinelSource::Harvest {
                    section_name: "TextureOverrideFixturePosition".to_string(),
                    file_path: std::path::PathBuf::from("fixture.ini"),
                },
            })
            .collect(),
        confidence: MatchConfidence::High,
    }
}

fn generate_keyviewer_ini(matches: &[MatchResult], toggle_key: &str, _legacy_path: &str) -> String {
    generate_keyviewer_ini_for_game(
        matches,
        toggle_key,
        crate::modules::games::domain::models::GameType::GIMI,
    )
    .unwrap()
}

mod atomic_tests;
mod ini_tests;
mod keybind_text_tests;
mod reload_key_tests;
mod status_tests;
