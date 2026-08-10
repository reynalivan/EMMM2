//! On-demand candidate scoring for the review modal's dropdown percentages.

use std::path::Path;

use crate::services::scanner::core::walker;
use crate::services::scanner::deep_matcher;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::models::result_summary::score_to_percentage;

/// Computes the percentage score for a specific batch of candidates against a folder.
/// Used for lazy loading dropdown percentages without scoring all DB entries.
pub fn score_candidates_batch(
    folder_path: &str,
    master_db: &deep_matcher::MasterDb,
    candidate_names: Vec<String>,
) -> std::collections::HashMap<String, u8> {
    use std::collections::HashMap;

    let mut results = HashMap::new();
    let path = Path::new(folder_path);

    if !path.exists() {
        return results;
    }

    let raw_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let is_disabled = crate::common::normalizer::is_disabled_folder(&raw_name);

    let candidate = walker::ModCandidate {
        path: path.to_path_buf(),
        display_name: crate::common::normalizer::normalize_display_name(&raw_name).into_owned(),
        raw_name,
        is_disabled,
    };

    let content = walker::scan_folder_content(&candidate.path, 3);
    let ini_filters = IniTokenizationConfig::default().prepare();
    let ai_config =
        crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig::default();
    let mut signal_cache =
        crate::services::scanner::deep_matcher::state::signal_cache::SignalCache::new();

    // Map requested names to entry IDs
    let entry_ids: Vec<usize> = master_db
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| candidate_names.contains(&e.name))
        .map(|(id, _)| id)
        .collect();

    if entry_ids.is_empty() {
        return results;
    }

    // Rather than hardcoding the pipeline logic here, we just run the specialized forced scoring
    // pipeline. It bypasses early exits and prunes to give us exact scores for the requested items.
    let match_result = deep_matcher::score_forced_candidates(
        &candidate,
        master_db,
        &content,
        &ini_filters,
        &ai_config,
        &mut signal_cache,
        &entry_ids,
    );

    // Provide baseline scores (0%) for requested names in case matcher filtered them out
    for name in &candidate_names {
        results.insert(name.clone(), 0);
    }

    // Update with actual scores if they survived to the final candidates list
    for c in &match_result.candidates_all {
        if candidate_names.contains(&c.name) {
            results.insert(c.name.clone(), score_to_percentage(c));
        }
    }

    results
}
