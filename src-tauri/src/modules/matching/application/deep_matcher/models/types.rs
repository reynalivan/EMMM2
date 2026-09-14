//! Domain types for the staged matcher pipeline.
//!
//! Contains: MatchMode, MatchStatus, Candidate, Evidence, Reason,
//! StagedMatchResult, ScoreState, Confidence, CustomSkin, DbEntry.

use serde::{Deserialize, Serialize};

// ==================== STAGED TYPES (NEW) ====================

/// Matching mode for staged matcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum MatchMode {
    /// Fast mode: minimal INI scan, shallow recursion.
    Quick,
    /// Accurate mode: deep INI scan, recursive content analysis.
    FullScoring,
}

impl std::fmt::Display for MatchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchMode::Quick => write!(f, "Quick"),
            MatchMode::FullScoring => write!(f, "FullScoring"),
        }
    }
}

/// Match status indicating whether candidate needs review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum MatchStatus {
    /// Accepted automatically with high confidence.
    AutoMatched,
    /// Requires manual review (top candidates returned).
    NeedsReview,
    /// No viable match found.
    NoMatch,
}

impl std::fmt::Display for MatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchStatus::AutoMatched => write!(f, "AutoMatched"),
            MatchStatus::NeedsReview => write!(f, "NeedsReview"),
            MatchStatus::NoMatch => write!(f, "NoMatch"),
        }
    }
}

/// A single candidate with score and structured reasons.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Candidate {
    /// Stable identifier for the database entry.
    #[specta(type = f64)]
    pub entry_id: usize,
    /// Display name.
    pub name: String,
    /// Object type (e.g., "Character", "Weapon").
    pub object_type: String,
    /// Aggregate score.
    pub score: f32,
    /// Confidence level for this candidate.
    pub confidence: Confidence,
    /// Structured reasons explaining the score.
    pub reasons: Vec<Reason>,
}

/// Evidence collected during matching process.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Evidence {
    /// Unique hashes found and matched (sorted).
    pub matched_hashes: Vec<String>,
    /// Unique tokens matched (sorted).
    pub matched_tokens: Vec<String>,
    /// INI section headers matched (sorted).
    pub matched_sections: Vec<String>,
    /// Number of INI files scanned.
    #[specta(type = f64)]
    pub scanned_ini_files: usize,
    /// Number of name items (subfolders + file stems) scanned.
    #[specta(type = f64)]
    pub scanned_name_items: usize,
}

impl Evidence {
    pub fn new() -> Self {
        Self {
            matched_hashes: Vec::new(),
            matched_tokens: Vec::new(),
            matched_sections: Vec::new(),
            scanned_ini_files: 0,
            scanned_name_items: 0,
        }
    }
}

impl Default for Evidence {
    fn default() -> Self {
        Self::new()
    }
}

/// Structured reason explaining why a candidate scored.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum Reason {
    /// Hash overlap evidence.
    HashOverlap { overlap: u32, unique_overlap: u32 },
    /// Strict alias match (all alias tokens present).
    AliasStrict { alias: String },
    /// Direct name token match (supporting only, not primary).
    DirectNameSupport { token: String },
    /// Token overlap ratio.
    TokenOverlap { ratio: f32 },
    /// Deep name token match (subfolders/file stems).
    DeepNameToken { token: String },
    /// INI section header token match.
    IniSectionToken { token: String },
    /// INI content token match (key names + path-like values).
    IniContentToken { token: String },
    /// AI re-ranking score (optional, future).
    AiRerank { ai_score: f32 },
    /// Penalizes conflicting strong tokens that point away from candidate.
    NegativeEvidence { foreign_strong_hits: u32 },
    /// Substring match in file stem / subfolder name (early stage F3).
    SubstringName {
        matched_term: String,
        source: String,
    },
    /// Last-resort root folder name substring match (F9).
    FolderNameRescue { matched_term: String },
    /// Last-resort approximate folder-name match (F9). Never primary evidence.
    FuzzyName {
        matched_term: String,
        similarity: f32,
    },
}

/// Result contract for staged matcher (new implementation).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct StagedMatchResult {
    /// Match status after staged evaluation.
    pub status: MatchStatus,
    /// Best candidate (if any).
    pub best: Option<Candidate>,
    /// Top-k candidates for review.
    pub candidates_topk: Vec<Candidate>,
    /// All evaluated candidates before truncation (useful for UI batch rendering).
    pub candidates_all: Vec<Candidate>,
    /// Evidence collected.
    pub evidence: Evidence,
}

impl StagedMatchResult {
    pub fn no_match() -> Self {
        Self {
            status: MatchStatus::NoMatch,
            best: None,
            candidates_topk: Vec::new(),
            candidates_all: Vec::new(),
            evidence: Evidence::new(),
        }
    }
}

/// Internal state for tracking score during pipeline execution.
#[derive(Debug, Clone)]
pub struct ScoreState {
    pub score: f32,
    pub reasons: Vec<Reason>,
    pub overlap: u32,
    pub unique_overlap: u32,
    pub max_confidence: Confidence,
}

impl ScoreState {
    pub fn new() -> Self {
        Self {
            score: 0.0,
            reasons: Vec::new(),
            overlap: 0,
            unique_overlap: 0,
            max_confidence: Confidence::None,
        }
    }
}

impl Default for ScoreState {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== DETERMINISTIC ORDERING ====================

/// Sort candidates deterministically: score desc → name asc → entry_id asc.
pub fn sort_candidates_deterministic(candidates: &mut [Candidate]) {
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.entry_id.cmp(&b.entry_id))
    });
}

// ==================== CAPS & LIMITS ====================

/// Maximum number of reasons to store per candidate (prevents unbounded growth).
pub const MAX_REASONS_PER_CANDIDATE: usize = 12;

/// Maximum number of evidence items (hashes, tokens, sections) to store.
pub const MAX_EVIDENCE_HASHES: usize = 50;
pub const MAX_EVIDENCE_TOKENS: usize = 50;
pub const MAX_EVIDENCE_SECTIONS: usize = 50;

// ==================== SHARED TYPES ====================

/// Matching confidence level.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
pub enum Confidence {
    None,
    Low,
    Medium,
    High,
    Excellent,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::Excellent => write!(f, "Excellent"),
            Confidence::High => write!(f, "High"),
            Confidence::Medium => write!(f, "Medium"),
            Confidence::Low => write!(f, "Low"),
            Confidence::None => write!(f, "None"),
        }
    }
}

/// A named skin/outfit with aliases.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CustomSkin {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub thumbnail_skin_path: Option<String>,
    #[serde(default)]
    pub rarity: Option<String>,
}

/// Controls whether a MasterDB entry can be selected as a concrete object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type, Default)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    #[default]
    Canonical,
    Taxonomy,
}

/// Identifies the 3DMigoto resource that a runtime target observes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeResourceKind {
    PositionVb,
    DrawVb,
    VertexBuffer,
    IndexBuffer,
    Texture,
    Shader,
}

/// Immutable catalog provenance for a runtime target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RuntimeTargetProvenance {
    pub source_repo: String,
    pub commit: String,
    pub path: String,
}

/// A catalog-declared 3DMigoto target used by runtime features.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RuntimeTarget {
    pub variant: String,
    #[serde(default)]
    pub component: Option<String>,
    pub resource_kind: RuntimeResourceKind,
    pub hash: String,
    #[serde(default)]
    pub slot: Option<String>,
    /// Optional draw discriminator for index-buffer targets. This remains
    /// absent for resources whose hash alone identifies the runtime callback.
    #[serde(default)]
    pub match_first_index: Option<u32>,
    pub provenance: RuntimeTargetProvenance,
}

impl RuntimeTarget {
    /// Validates the catalog contract without changing the authored target.
    pub fn validate(&self) -> Result<(), String> {
        validate_required("variant", &self.variant)?;
        validate_optional("component", self.component.as_deref())?;
        validate_optional("slot", self.slot.as_deref())?;
        validate_required("provenance.source_repo", &self.provenance.source_repo)?;
        validate_required("provenance.commit", &self.provenance.commit)?;
        validate_required("provenance.path", &self.provenance.path)?;

        let expected_length = if self.resource_kind == RuntimeResourceKind::Shader {
            16
        } else {
            8
        };
        if self.hash.len() != expected_length
            || !self.hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            let kind = if self.resource_kind == RuntimeResourceKind::Shader {
                "shader"
            } else {
                "resource"
            };
            return Err(format!(
                "{kind} hash must contain exactly {expected_length} hexadecimal characters"
            ));
        }

        Ok(())
    }

    pub fn is_resource_target(&self) -> bool {
        self.resource_kind != RuntimeResourceKind::Shader
    }
}

fn validate_required(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn validate_optional(label: &str, value: Option<&str>) -> Result<(), String> {
    if let Some(value) = value {
        validate_required(label, value)?;
    }
    Ok(())
}

/// A single DB entry from Master DB.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DbEntry {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub object_type: String,
    /// Whether this is a matchable object or a generic taxonomy placeholder.
    #[serde(default)]
    pub entry_kind: EntryKind,
    #[serde(default)]
    pub custom_skins: Vec<CustomSkin>,
    #[serde(default)]
    pub thumbnail_path: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Optional hashes for this entry (default: empty dictionary).
    /// Maps a skin/variant name to its list of hashes. Invalid hashes are ignored.
    #[serde(default)]
    pub hash_db: std::collections::HashMap<String, Vec<String>>,
    /// Optional catalog-declared runtime targets. Legacy hash_db is not runtime eligibility.
    #[serde(default)]
    pub runtime_targets: Vec<RuntimeTarget>,
}
