use crate::modules::ingestion::application::import_batch::types::{
    CanonicalSuggestion, ConfidenceTier, ImportMatchStatus, MatchEvidence, StableCategory,
};
use crate::modules::matching::application::deep_matcher::analysis::ai_rerank::AiRerankConfig;
use crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters;
use crate::modules::matching::application::deep_matcher::models::result_summary::score_to_percentage;
use crate::modules::matching::application::deep_matcher::state::signal_cache::SignalCache;
use crate::modules::matching::application::deep_matcher::{
    match_folder_phased_cached, MasterDb, Reason,
};
use crate::modules::workspace::application::scanner::core::walker::{
    scan_folder_content, ModCandidate,
};
use std::path::Path;

pub fn match_canonical_objects(
    source_path: &Path,
    planned_name: &str,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
) -> Vec<CanonicalSuggestion> {
    let canonical_db = prepare_canonical_match_db(master_db);
    match_canonical_objects_with_prepared_db(source_path, planned_name, &canonical_db, ini_filters)
}

/// Builds the immutable canonical-only matcher index once for a classification
/// batch. Callers still scan each source folder at match time, so disk remains
/// the source of truth for suggestions and stale-preview validation.
pub(crate) fn prepare_canonical_match_db(master_db: &MasterDb) -> MasterDb {
    MasterDb::new(
        master_db
            .entries
            .iter()
            .filter(|entry| {
                entry.entry_kind
                    == crate::modules::matching::application::deep_matcher::EntryKind::Canonical
            })
            .cloned()
            .collect(),
    )
}

pub(crate) fn match_canonical_objects_with_prepared_db(
    source_path: &Path,
    planned_name: &str,
    canonical_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
) -> Vec<CanonicalSuggestion> {
    let content = scan_folder_content(source_path, 3);
    match_canonical_objects_with_prepared_content(
        source_path,
        planned_name,
        canonical_db,
        ini_filters,
        &content,
    )
}

pub(crate) fn match_canonical_objects_with_prepared_content(
    source_path: &Path,
    planned_name: &str,
    canonical_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
    content: &crate::modules::workspace::application::scanner::core::walker::FolderContent,
) -> Vec<CanonicalSuggestion> {
    let mut signal_cache = SignalCache::new();
    match_canonical_objects_with_prepared_content_cached(
        source_path,
        planned_name,
        canonical_db,
        ini_filters,
        content,
        &mut signal_cache,
    )
}

/// Match canonical entries against a caller-owned disk snapshot. Keeping the
/// cache request-local preserves fresh disk reads for each request while
/// avoiding duplicate INI signal work between classification passes.
pub(crate) fn match_canonical_objects_with_prepared_content_cached(
    source_path: &Path,
    planned_name: &str,
    canonical_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
    content: &crate::modules::workspace::application::scanner::core::walker::FolderContent,
    signal_cache: &mut SignalCache,
) -> Vec<CanonicalSuggestion> {
    match_canonical_objects_against(
        source_path,
        planned_name,
        canonical_db,
        ini_filters,
        content,
        signal_cache,
    )
}

pub fn match_canonical_objects_for_category(
    source_path: &Path,
    planned_name: &str,
    category: StableCategory,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
) -> Vec<CanonicalSuggestion> {
    let content = scan_folder_content(source_path, 3);
    let mut signal_cache = SignalCache::new();
    match_canonical_objects_for_category_with_prepared_content(
        source_path,
        planned_name,
        category,
        master_db,
        ini_filters,
        &content,
        &mut signal_cache,
    )
}

pub(crate) fn match_canonical_objects_for_category_with_prepared_content(
    source_path: &Path,
    planned_name: &str,
    category: StableCategory,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
    content: &crate::modules::workspace::application::scanner::core::walker::FolderContent,
    signal_cache: &mut SignalCache,
) -> Vec<CanonicalSuggestion> {
    let filtered = MasterDb::new(
        master_db
            .entries
            .iter()
            .filter(|entry| {
                entry.entry_kind
                    == crate::modules::matching::application::deep_matcher::EntryKind::Canonical
                    && entry.object_type == category.as_str()
            })
            .cloned()
            .collect(),
    );
    match_canonical_objects_against(
        source_path,
        planned_name,
        &filtered,
        ini_filters,
        content,
        signal_cache,
    )
}

fn match_canonical_objects_against(
    source_path: &Path,
    planned_name: &str,
    filtered: &MasterDb,
    ini_filters: &PreparedTokenFilters,
    content: &crate::modules::workspace::application::scanner::core::walker::FolderContent,
    signal_cache: &mut SignalCache,
) -> Vec<CanonicalSuggestion> {
    if filtered.entries.is_empty() {
        return Vec::new();
    }
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
    let result = match_folder_phased_cached(
        &candidate,
        &filtered,
        content,
        ini_filters,
        &AiRerankConfig::default(),
        signal_cache,
    );
    let match_status = match result.status {
        crate::modules::matching::application::deep_matcher::MatchStatus::AutoMatched => {
            ImportMatchStatus::AutoMatched
        }
        crate::modules::matching::application::deep_matcher::MatchStatus::NeedsReview => {
            ImportMatchStatus::NeedsReview
        }
        crate::modules::matching::application::deep_matcher::MatchStatus::NoMatch => {
            ImportMatchStatus::NoMatch
        }
    };
    result
        .candidates_topk
        .iter()
        .map(|candidate| {
            let percentage = score_to_percentage(candidate);
            CanonicalSuggestion {
                entry_key: crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key(
                    &candidate.name,
                ),
                name: candidate.name.clone(),
                matched_alias: alias_reason(&candidate.reasons),
                confidence_percentage: percentage,
                confidence_tier: ConfidenceTier::from_percentage(percentage),
                match_status,
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

#[cfg(test)]
mod tests {
    use super::{
        match_canonical_objects_for_category_with_prepared_content, prepare_canonical_match_db,
    };
    use crate::modules::catalog::application::match_engine::classification::classify_source_with_content;
    use crate::modules::ingestion::application::import_batch::types::StableCategory;
    use crate::modules::matching::application::deep_matcher::analysis::content::IniTokenizationConfig;
    use crate::modules::matching::application::deep_matcher::state::signal_cache::SignalCache;
    use crate::modules::matching::application::deep_matcher::MasterDb;
    use crate::modules::workspace::application::scanner::core::walker::scan_folder_content;

    #[test]
    fn prepared_canonical_match_db_excludes_taxonomy_entries() {
        let entries = serde_json::from_value(serde_json::json!([
            {"name": "Ayaka", "object_type": "Character", "entry_kind": "canonical"},
            {"name": "Weapon Taxonomy", "object_type": "Weapon", "entry_kind": "taxonomy"}
        ]))
        .expect("fixture entries should deserialize");
        let prepared = prepare_canonical_match_db(&MasterDb::new(entries));

        assert_eq!(prepared.entries.len(), 1);
        assert_eq!(prepared.entries[0].name, "Ayaka");
    }

    #[test]
    fn prepared_content_reuses_signal_cache_across_import_matching_passes() {
        let root = tempfile::tempdir().expect("temp directory");
        let source = root.path().join("Unsorted skin");
        std::fs::create_dir(&source).expect("source directory");
        std::fs::write(
            source.join("mod.ini"),
            "[TextureOverrideRaiden]\nhash = 12345678\n",
        )
        .expect("source ini");
        let entries = serde_json::from_value(serde_json::json!([
            {
                "name": "Raiden",
                "object_type": "Character",
                "entry_kind": "canonical",
                "hashes": ["12345678"]
            }
        ]))
        .expect("fixture entries");
        let master_db = MasterDb::new(entries);
        let filters = IniTokenizationConfig::default().prepare();
        let content = scan_folder_content(&source, 3);
        let mut signal_cache = SignalCache::new();

        let categories = classify_source_with_content(
            &source,
            "Unsorted skin",
            &master_db,
            &filters,
            &content,
            &mut signal_cache,
        );
        let cached_after_classification = signal_cache.len();
        let canonical = match_canonical_objects_for_category_with_prepared_content(
            &source,
            "Unsorted skin",
            StableCategory::Character,
            &master_db,
            &filters,
            &content,
            &mut signal_cache,
        );

        assert!(!categories.is_empty());
        assert!(!canonical.is_empty());
        assert_eq!(
            signal_cache.len(),
            cached_after_classification,
            "canonical matching must reuse the current item's signal cache"
        );
    }
}
