use std::collections::HashMap;
use std::path::Path;

use crate::services::scanner::core::walker::FolderContent;
use crate::services::scanner::deep_matcher::analysis::content::{
    collect_deep_signals, FolderSignals, IniTokenizationConfig,
};
use crate::services::scanner::deep_matcher::MatchMode;

/// In-memory cache for `FolderSignals` keyed by `(folder_path, mode)`.
///
/// Avoids re-reading and re-tokenizing INI files when `match_folder_phased`
/// falls through from Quick → Full for the same folder within one scan batch.
#[derive(Debug, Default)]
pub struct SignalCache {
    store: HashMap<(String, u8), FolderSignals>,
}

impl SignalCache {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    /// Get cached signals or compute + cache them.
    pub fn get_or_compute(
        &mut self,
        folder: &Path,
        content: &FolderContent,
        mode: MatchMode,
        ini_config: &IniTokenizationConfig,
    ) -> &FolderSignals {
        let key = (folder.to_string_lossy().to_string(), mode as u8);

        self.store
            .entry(key)
            .or_insert_with_key(|_| collect_deep_signals(folder, content, mode, ini_config))
    }

    /// Number of cached entries (for diagnostics).
    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
