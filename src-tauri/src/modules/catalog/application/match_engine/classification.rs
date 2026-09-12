use crate::modules::ingestion::application::import_batch::types::{
    CategorySuggestion, ConfidenceTier, MatchEvidence, StableCategory,
};
use crate::modules::matching::application::deep_matcher::analysis::ai_rerank::AiRerankConfig;
use crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters;
use crate::modules::matching::application::deep_matcher::models::result_summary::score_to_percentage;
use crate::modules::matching::application::deep_matcher::StagedMatchResult;
use crate::modules::matching::application::deep_matcher::{match_folder_phased, MasterDb};
use crate::modules::workspace::application::scanner::core::walker::{
    scan_folder_content, ModCandidate,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

pub fn category_suggestions(result: &StagedMatchResult) -> Vec<CategorySuggestion> {
    let mut scores = BTreeMap::<String, u8>::new();
    for candidate in &result.candidates_all {
        let percentage = score_to_percentage(candidate);
        scores
            .entry(candidate.object_type.clone())
            .and_modify(|score| *score = (*score).max(percentage))
            .or_insert(percentage);
    }
    let mut suggestions = scores
        .into_iter()
        .filter_map(|(category, score)| {
            let category = StableCategory::from_str(&category).ok()?;
            Some(CategorySuggestion {
                category,
                sub_category: None,
                confidence_percentage: score,
                confidence_tier: ConfidenceTier::from_percentage(score),
                evidence: vec![MatchEvidence {
                    source: "deep_matcher".to_string(),
                    value: category.as_str().to_string(),
                    score: f64::from(score),
                }],
                metadata: serde_json::json!({}),
            })
        })
        .collect::<Vec<_>>();
    suggestions.sort_by(|left, right| {
        right
            .confidence_percentage
            .cmp(&left.confidence_percentage)
            .then_with(|| left.category.as_str().cmp(right.category.as_str()))
    });
    suggestions
}

pub fn classify_source(
    source_path: &Path,
    planned_name: &str,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
) -> Vec<CategorySuggestion> {
    let candidate = ModCandidate {
        path: source_path.to_path_buf(),
        raw_name: planned_name.to_string(),
        display_name: crate::modules::workspace::domain::normalizer::normalize_display_name(
            planned_name,
        )
        .into_owned(),
        is_disabled: crate::modules::workspace::domain::normalizer::is_disabled_folder(
            planned_name,
        ),
    };
    let content = scan_folder_content(source_path, 3);
    let result = match_folder_phased(
        &candidate,
        master_db,
        &content,
        ini_filters,
        &AiRerankConfig::default(),
    );
    category_suggestions(&result)
}
