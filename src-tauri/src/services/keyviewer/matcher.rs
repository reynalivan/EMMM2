//! Hash matcher — scores active-mod hashes against MasterDb entries and selects sentinels.
//!
//! **Algorithm:**
//! 1. For each `KvObjectEntry`: compute intersection `I = active_hashes ∩ known_hashes`
//! 2. Score each entry: base per-hash + occurrence bonus + rarity bonus
//! 3. Pick best match if `score ≥ threshold`; tiebreak: score desc → name asc
//! 4. Select sentinel hashes: top K from intersection, excluding high-collision
//! 5. High-collision: hash appears in ≥`collision_threshold` objects

use std::collections::{HashMap, HashSet};

use super::resource_pack::KvObjectEntry;

// ─── Configuration ───────────────────────────────────────────────────────────

/// Configuration for the matching + sentinel pipeline.
#[derive(Debug, Clone)]
pub struct MatchConfig {
    /// Minimum score to accept a match (below → no match).
    pub score_threshold: f32,
    /// Base score awarded per intersecting hash.
    pub base_per_hash: f32,
    /// Bonus multiplied by `ln(1 + occurrence_count)` for each hash.
    pub occurrence_bonus_factor: f32,
    /// Bonus for rare hashes (appear in ≤ `rarity_max_objects` entries).
    pub rarity_bonus: f32,
    /// Hashes appearing in this many entries or fewer are considered "rare".
    pub rarity_max_objects: usize,
    /// Number of sentinel hashes to select per matched object.
    pub sentinel_count: usize,
    /// A hash appearing in ≥ this many objects is considered high-collision.
    pub collision_threshold: usize,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            score_threshold: 5.0,
            base_per_hash: 10.0,
            occurrence_bonus_factor: 2.0,
            rarity_bonus: 5.0,
            rarity_max_objects: 2,
            sentinel_count: 3,
            collision_threshold: 3,
        }
    }
}

// ─── Result Types ────────────────────────────────────────────────────────────

/// Result of matching active hashes against a single KvObjectEntry.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Name of the matched object (e.g. "Albedo").
    pub object_name: String,
    /// Object type (e.g. "Character").
    pub object_type: String,
    /// Aggregate match score.
    pub score: f32,
    /// Hashes in the intersection (active ∩ known).
    pub matched_hashes: Vec<String>,
    /// Selected sentinel hashes for runtime detection.
    pub sentinel_hashes: Vec<String>,
    /// Confidence level based on score relative to threshold.
    pub confidence: MatchConfidence,
}

/// Confidence levels for matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchConfidence {
    /// Score ≥ 4× threshold.
    Excellent,
    /// Score ≥ 2× threshold.
    High,
    /// Score ≥ threshold.
    Medium,
    /// Score < threshold (used internally, never returned as a final match).
    Low,
}

// ─── Core Matching ───────────────────────────────────────────────────────────

/// Build a reverse index: hash → set of object names that contain it.
fn build_hash_object_index(entries: &[KvObjectEntry]) -> HashMap<String, HashSet<String>> {
    let mut index: HashMap<String, HashSet<String>> = HashMap::new();
    for entry in entries {
        for hash in &entry.code_hashes {
            index
                .entry(hash.clone())
                .or_default()
                .insert(entry.name.clone());
        }
    }
    index
}

/// Score a single entry against the active hashes.
///
/// Returns `None` if the intersection is empty (no possible match).
fn score_entry(
    entry: &KvObjectEntry,
    active_hashes: &HashSet<String>,
    occurrence_counts: &HashMap<String, usize>,
    hash_object_index: &HashMap<String, HashSet<String>>,
    config: &MatchConfig,
) -> Option<(f32, Vec<String>)> {
    let intersection: Vec<String> = entry
        .code_hashes
        .iter()
        .filter(|h| active_hashes.contains(h.as_str()))
        .cloned()
        .collect();

    if intersection.is_empty() {
        return None;
    }

    let mut score: f32 = 0.0;

    for hash in &intersection {
        // Base score per hash
        score += config.base_per_hash;

        // Occurrence bonus: more occurrences → slight boost (log-scaled)
        let occ = occurrence_counts.get(hash).copied().unwrap_or(1) as f32;
        score += config.occurrence_bonus_factor * (1.0 + occ).ln();

        // Rarity bonus: hash appears in few objects → strong signal
        let objects_with_hash = hash_object_index.get(hash).map(|s| s.len()).unwrap_or(0);
        if objects_with_hash <= config.rarity_max_objects {
            score += config.rarity_bonus;
        }
    }

    Some((score, intersection))
}

/// Match active mod hashes against all KvObjectEntries and return ranked results.
///
/// # Arguments
/// - `entries`: All `KvObjectEntry` from the resource pack (via `extract_kv_entries`)
/// - `active_hashes`: Set of hashes harvested from currently-enabled mods
/// - `occurrence_counts`: How many times each hash appears across all INI files
/// - `config`: Matching configuration
///
/// # Returns
/// Sorted list of `MatchResult` (best first), filtered by `score_threshold`.
pub fn match_objects(
    entries: &[KvObjectEntry],
    active_hashes: &HashSet<String>,
    occurrence_counts: &HashMap<String, usize>,
    config: &MatchConfig,
) -> Vec<MatchResult> {
    let hash_object_index = build_hash_object_index(entries);

    let mut results: Vec<MatchResult> = entries
        .iter()
        .filter_map(|entry| {
            let (score, matched_hashes) = score_entry(
                entry,
                active_hashes,
                occurrence_counts,
                &hash_object_index,
                config,
            )?;

            if score < config.score_threshold {
                return None;
            }

            let sentinel_hashes = select_sentinels(&matched_hashes, &hash_object_index, config);

            let confidence = if score >= config.score_threshold * 4.0 {
                MatchConfidence::Excellent
            } else if score >= config.score_threshold * 2.0 {
                MatchConfidence::High
            } else {
                MatchConfidence::Medium
            };

            Some(MatchResult {
                object_name: entry.name.clone(),
                object_type: entry.object_type.clone(),
                score,
                matched_hashes,
                sentinel_hashes,
                confidence,
            })
        })
        .collect();

    // Sort: score desc → name asc (deterministic)
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.object_name.cmp(&b.object_name))
    });

    results
}

// ─── Sentinel Selection ──────────────────────────────────────────────────────

/// Select sentinel hashes from the matched intersection.
///
/// Prefers hashes that appear in fewer objects (higher rarity = better sentinel).
/// Excludes high-collision hashes (appear in ≥ `collision_threshold` objects).
/// Returns up to `sentinel_count` hashes.
fn select_sentinels(
    matched_hashes: &[String],
    hash_object_index: &HashMap<String, HashSet<String>>,
    config: &MatchConfig,
) -> Vec<String> {
    // Score each hash by rarity (fewer objects = better sentinel)
    let mut scored: Vec<(&String, usize)> = matched_hashes
        .iter()
        .map(|h| {
            let count = hash_object_index.get(h).map(|s| s.len()).unwrap_or(1);
            (h, count)
        })
        .collect();

    // Filter out high-collision hashes
    scored.retain(|(_, count)| *count < config.collision_threshold);

    // Sort by object count ascending (rarest first), then hash value for stability
    scored.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(b.0)));

    scored
        .into_iter()
        .take(config.sentinel_count)
        .map(|(h, _)| h.clone())
        .collect()
}

/// Identify high-collision hashes across all entries.
///
/// Returns hashes that appear in ≥ `collision_threshold` different objects.
/// These should be avoided as sentinels since they can't uniquely identify objects.
pub fn find_collision_hashes(
    entries: &[KvObjectEntry],
    collision_threshold: usize,
) -> HashSet<String> {
    let index = build_hash_object_index(entries);
    index
        .into_iter()
        .filter(|(_, objects)| objects.len() >= collision_threshold)
        .map(|(hash, _)| hash)
        .collect()
}
