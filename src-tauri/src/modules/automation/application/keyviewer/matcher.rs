//! Catalog matching and geometry-first runtime sentinel selection.
//!
//! Matching proves which catalog identity owns an enabled mod. Sentinel
//! selection is deliberately independent: it only considers typed targets
//! observed in that mod and uses resource-kind tiers, never hash rarity or a
//! mod's `match_priority`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use crate::modules::matching::application::deep_matcher::models::types::{
    RuntimeResourceKind, RuntimeTarget, RuntimeTargetProvenance,
};

/// Kept as a small compatibility contract for the post-apply call site.
/// Runtime eligibility no longer has a score or a sentinel-count knob.
#[derive(Debug, Clone, Default)]
pub struct MatchConfig;

/// A flattened catalog entry used only by KeyViewer.
#[derive(Debug, Clone)]
pub struct KvObjectEntry {
    pub name: String,
    pub object_type: String,
    /// Hashes are identity evidence. They are not directly emitted as observers.
    pub code_hashes: Vec<String>,
    pub skin_hashes: HashMap<String, Vec<String>>,
    pub runtime_targets: Vec<RuntimeTarget>,
    pub tags: Vec<String>,
    pub thumbnail_path: Option<String>,
}

/// Origin retained for preview/diagnostics without exposing filesystem paths
/// in generated overlay text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSentinelSource {
    Catalog {
        variant: String,
        component: Option<String>,
        provenance: RuntimeTargetProvenance,
    },
    Harvest {
        section_name: String,
        file_path: PathBuf,
    },
}

/// Exact observer emitted into the generated 3DMigoto INI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSentinel {
    pub hash: String,
    pub resource_kind: RuntimeResourceKind,
    pub callback_slot: String,
    pub match_first_index: Option<u32>,
    pub source: RuntimeSentinelSource,
}

impl RuntimeSentinel {
    pub fn from_catalog(target: &RuntimeTarget) -> Option<Self> {
        let callback_slot = target.slot.as_ref()?.trim().to_ascii_lowercase();
        (!callback_slot.is_empty() && target.resource_kind != RuntimeResourceKind::Shader).then(
            || Self {
                hash: target.hash.to_ascii_lowercase(),
                resource_kind: target.resource_kind,
                callback_slot,
                match_first_index: target.match_first_index,
                source: RuntimeSentinelSource::Catalog {
                    variant: target.variant.clone(),
                    component: target.component.clone(),
                    provenance: target.provenance.clone(),
                },
            },
        )
    }

    pub fn stable_key(&self) -> (String, RuntimeResourceKind, String, Option<u32>) {
        (
            self.hash.clone(),
            self.resource_kind,
            self.callback_slot.clone(),
            self.match_first_index,
        )
    }
}

/// Result of matching one catalog identity.
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub object_name: String,
    pub object_type: String,
    /// Number of catalog targets found in enabled mods, retained for preview.
    pub score: f32,
    pub matched_hashes: Vec<String>,
    /// Every observer in the selected resource tier is OR-ed into one panel.
    pub sentinels: Vec<RuntimeSentinel>,
    pub confidence: MatchConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchConfidence {
    Excellent,
    High,
    Medium,
    Low,
}

/// Selects all targets in the safest available resource tier.
///
/// Index buffers are not safe without draw context. Texture is a final
/// fallback, intended for face-only/texture-only mods once no geometry target
/// from that mod is present.
pub fn select_geometry_first_sentinels(
    candidates: impl IntoIterator<Item = RuntimeSentinel>,
) -> Vec<RuntimeSentinel> {
    let mut candidates: Vec<_> = candidates
        .into_iter()
        .filter(|target| {
            target.hash.len() == 8 && target.hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .filter(|target| {
            target.resource_kind != RuntimeResourceKind::Shader
                && !(target.resource_kind == RuntimeResourceKind::IndexBuffer
                    && target.match_first_index.is_none())
        })
        .collect();
    candidates.sort_by_key(|candidate| candidate.stable_key());
    candidates.dedup_by(|left, right| left.stable_key() == right.stable_key());

    let Some(tier) = candidates.iter().map(resource_tier).min() else {
        return Vec::new();
    };
    candidates
        .into_iter()
        .filter(|candidate| resource_tier(candidate) == tier)
        .collect()
}

fn resource_tier(target: &RuntimeSentinel) -> u8 {
    match target.resource_kind {
        RuntimeResourceKind::PositionVb => 0,
        RuntimeResourceKind::DrawVb | RuntimeResourceKind::VertexBuffer => 1,
        RuntimeResourceKind::IndexBuffer => 2,
        RuntimeResourceKind::Texture => 3,
        RuntimeResourceKind::Shader => 4,
    }
}

fn catalog_target_owners(entries: &[KvObjectEntry]) -> HashMap<SentinelIdentity, BTreeSet<String>> {
    let mut owners = HashMap::new();
    for entry in entries {
        for target in &entry.runtime_targets {
            let Some(sentinel) = RuntimeSentinel::from_catalog(target) else {
                continue;
            };
            owners
                .entry(SentinelIdentity::from(&sentinel))
                .or_insert_with(BTreeSet::new)
                .insert(entry.name.clone());
        }
    }
    owners
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct SentinelIdentity {
    hash: String,
    resource_kind: RuntimeResourceKind,
    callback_slot: String,
    match_first_index: Option<u32>,
}

impl From<&RuntimeSentinel> for SentinelIdentity {
    fn from(target: &RuntimeSentinel) -> Self {
        Self {
            hash: target.hash.clone(),
            resource_kind: target.resource_kind,
            callback_slot: target.callback_slot.clone(),
            match_first_index: target.match_first_index,
        }
    }
}

/// Match active resource hashes against catalog identities.
///
/// A character needs one observed target to establish ownership. A catalog
/// target shared by multiple character entries is removed unless its full
/// typed identity (including draw context) is unique, so one frame cannot
/// create two contradictory panels.
pub fn match_objects(
    entries: &[KvObjectEntry],
    active_hashes: &HashSet<String>,
    _occurrence_counts: &HashMap<String, usize>,
    _config: &MatchConfig,
) -> Vec<MatchResult> {
    let owners = catalog_target_owners(entries);
    let mut results = Vec::new();

    for entry in entries {
        let mut by_variant: BTreeMap<String, Vec<RuntimeSentinel>> = BTreeMap::new();
        let mut matched_hashes = BTreeSet::new();
        for target in &entry.runtime_targets {
            let hash = target.hash.to_ascii_lowercase();
            if !active_hashes.contains(&hash) {
                continue;
            }
            matched_hashes.insert(hash);
            let Some(sentinel) = RuntimeSentinel::from_catalog(target) else {
                continue;
            };
            if owners
                .get(&SentinelIdentity::from(&sentinel))
                .is_some_and(|names| names.len() > 1)
            {
                continue;
            }
            by_variant
                .entry(target.variant.clone())
                .or_default()
                .push(sentinel);
        }

        let Some((_, sentinels)) = by_variant
            .into_iter()
            .filter_map(|(variant, targets)| {
                let selected = select_geometry_first_sentinels(targets);
                (!selected.is_empty()).then_some((variant, selected))
            })
            .min_by(|(left_variant, left), (right_variant, right)| {
                resource_tier(&left[0])
                    .cmp(&resource_tier(&right[0]))
                    .then_with(|| right.len().cmp(&left.len()))
                    .then_with(|| left_variant.cmp(right_variant))
            })
        else {
            continue;
        };

        let matched_hashes: Vec<_> = matched_hashes.into_iter().collect();
        let score = matched_hashes.len() as f32;
        results.push(MatchResult {
            object_name: entry.name.clone(),
            object_type: entry.object_type.clone(),
            score,
            matched_hashes,
            sentinels,
            confidence: if score >= 4.0 {
                MatchConfidence::Excellent
            } else if score >= 2.0 {
                MatchConfidence::High
            } else {
                MatchConfidence::Medium
            },
        });
    }

    results.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.object_name.cmp(&right.object_name))
    });
    results
}
