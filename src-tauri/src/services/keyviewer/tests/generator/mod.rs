//! Unit tests for the file generation pipeline.

use std::collections::HashMap;
use tempfile::TempDir;

use crate::services::ini::document::KeyBinding;
use crate::services::keyviewer::generator::{
    atomic_write, discover_reload_key, generate_keybind_text, generate_keyviewer_ini,
    generate_status_text, write_keybind_files, write_status_file, SourceKeyBinding, StatusFields,
};
use crate::services::keyviewer::matcher::{MatchConfidence, MatchResult};

fn make_keybinding(section: &str, key: Option<&str>, back: Option<&str>) -> KeyBinding {
    KeyBinding {
        section_name: section.to_string(),
        key: key.map(|s| s.to_string()),
        back: back.map(|s| s.to_string()),
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
        sentinel_hashes: sentinels.iter().map(|s| s.to_string()).collect(),
        confidence: MatchConfidence::High,
    }
}

mod atomic_tests;
mod ini_tests;
mod keybind_text_tests;
mod reload_key_tests;
mod status_tests;
