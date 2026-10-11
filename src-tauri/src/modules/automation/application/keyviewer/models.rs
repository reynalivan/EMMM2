use crate::modules::library::application::ini::document::KeyBinding;

use super::matcher::MatchResult;

#[derive(Debug, Clone)]
pub struct SourceKeyBinding {
    pub mod_name: String,
    pub keybinds: Vec<KeyBinding>,
}

/// Observer metadata and its bindings stay together through generation.
#[derive(Debug, Clone)]
pub struct KeyViewerPanel {
    pub match_result: MatchResult,
    pub sources: Vec<SourceKeyBinding>,
}
