use crate::services::import_batch::types::{
    CanonicalSuggestion, ConfidenceTier, MatchEvidence, StableCategory,
};
use crate::services::scanner::core::walker::{scan_folder_content, ModCandidate};
use crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig;
use crate::services::scanner::deep_matcher::analysis::content::PreparedTokenFilters;
use crate::services::scanner::deep_matcher::models::result_summary::score_to_percentage;
use crate::services::scanner::deep_matcher::{match_folder_phased, MasterDb, Reason};
use std::path::Path;

pub fn match_canonical_objects(
    source_path: &Path,
    planned_name: &str,
    category: StableCategory,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
) -> Vec<CanonicalSuggestion> {
    let filtered = MasterDb::new(
        master_db
            .entries
            .iter()
            .filter(|entry| {
                entry.entry_kind == crate::services::scanner::deep_matcher::EntryKind::Canonical
                    && entry.object_type == category.as_str()
            })
            .cloned()
            .collect(),
    );
    if filtered.entries.is_empty() {
        return Vec::new();
    }
    let candidate = ModCandidate {
        path: source_path.to_path_buf(),
        raw_name: planned_name.to_string(),
        display_name: crate::common::normalizer::normalize_display_name(planned_name).into_owned(),
        is_disabled: crate::common::normalizer::is_disabled_folder(planned_name),
    };
    let content = scan_folder_content(source_path, 3);
    let result = match_folder_phased(
        &candidate,
        &filtered,
        &content,
        ini_filters,
        &AiRerankConfig::default(),
    );
    result
        .candidates_topk
        .iter()
        .map(|candidate| {
            let percentage = score_to_percentage(candidate);
            CanonicalSuggestion {
                entry_key: crate::services::scanner::sync::helpers::canonical_entry_key(
                    &candidate.name,
                ),
                name: candidate.name.clone(),
                matched_alias: alias_reason(&candidate.reasons),
                confidence_percentage: percentage,
                confidence_tier: ConfidenceTier::from_percentage(percentage),
                evidence: candidate
                    .reasons
                    .iter()
                    .take(12)
                    .map(|reason| MatchEvidence {
                        source: "deep_matcher".to_string(),
                        value: format!("{reason:?}"),
                        score: f64::from(candidate.score),
                    })
                    .collect(),
            }
        })
        .collect()
}

fn alias_reason(reasons: &[Reason]) -> Option<String> {
    reasons.iter().find_map(|reason| match reason {
        Reason::AliasStrict { alias } => Some(alias.clone()),
        _ => None,
    })
}
