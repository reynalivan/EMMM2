//! Domain types for the staged matcher pipeline.
//!
//! Contains: MatchMode, MatchStatus, Candidate, Evidence, Reason,
//! StagedMatchResult, ScoreState, Confidence, CustomSkin, DbEntry.

use serde::{Deserialize, Serialize};

// ==================== STAGED TYPES (NEW) ====================

/// Matching mode for staged matcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    /// Stable identifier for the database entry.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Unique hashes found and matched (sorted).
    pub matched_hashes: Vec<String>,
    /// Unique tokens matched (sorted).
    pub matched_tokens: Vec<String>,
    /// INI section headers matched (sorted).
    pub matched_sections: Vec<String>,
    /// Number of INI files scanned.
    pub scanned_ini_files: usize,
    /// Number of name items (subfolders + file stems) scanned.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// Result contract for staged matcher (new implementation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedMatchResult {
    /// Match status after staged evaluation.
    pub status: MatchStatus,
    /// Best candidate (if any).
    pub best: Option<Candidate>,
    /// Top-k candidates for review.
    pub candidates_topk: Vec<Candidate>,
    /// Evidence collected.
    pub evidence: Evidence,
}

impl StagedMatchResult {
    pub fn no_match() -> Self {
        Self {
            status: MatchStatus::NoMatch,
            best: None,
            candidates_topk: Vec::new(),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSkin {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub thumbnail_skin_path: Option<String>,
    #[serde(default)]
    pub rarity: Option<String>,
}

/// A single DB entry from Master DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbEntry {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub object_type: String,
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
}
