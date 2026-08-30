use crate::modules::ingestion::application::import_batch::types::{MatchEvidence, SourceFingerprint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalIdentity {
    pub entry_key: String,
    pub name: String,
    pub entry_kind: crate::modules::matching::application::deep_matcher::EntryKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SourceInspection {
    pub source_path: String,
    pub source_name: String,
    pub normalized_name: String,
    pub nested_names: Vec<String>,
    pub matching_files: Vec<String>,
    pub ini_sections: Vec<String>,
    pub evidence: Vec<MatchEvidence>,
    pub fingerprint: SourceFingerprint,
}
