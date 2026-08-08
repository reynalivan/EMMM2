use crate::domain::errors::ScannerError;
use crate::services::scanner::core::walker::ModCandidate;
use crate::types::dup_scan::{DupScanGroup, DupScanMember, DupScanSignal};
use rayon::prelude::*;

use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::signals::{aggregate_signals, collect_snapshot, hash_snapshot, ModSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DedupScanStatus {
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DedupScanOutcome {
    pub status: DedupScanStatus,
    pub groups: Vec<DupScanGroup>,
    #[specta(type = f64)]
    pub total_folders: usize,
}

type ScoredPair = (usize, usize, u8, Vec<DupScanSignal>, String);

pub async fn scan_duplicates(
    mods_root: &Path,
    game_id: &str,
    db: &SqlitePool,
    cancel_flag: Arc<AtomicBool>,
) -> Result<DedupScanOutcome, ScannerError> {
    // One read of the mods table feeds both the scan candidates and the
    // path -> (mod id, is_safe) lookup used when grouping the results.
    let mut conn = db.acquire().await?;
    let mod_rows = crate::repo::mod_repo::get_all_mods_id_and_paths_tx(&mut conn, game_id).await?;
    drop(conn);

    let candidates = build_candidates(&mod_rows, mods_root);
    let total_folders = candidates.len();

    if is_cancelled(&cancel_flag) {
        return Ok(cancelled(total_folders));
    }

    let path_to_mod_id: HashMap<String, (String, bool)> = mod_rows
        .into_iter()
        .map(|(id, folder_path, is_safe)| (folder_path, (id, is_safe)))
        .collect();
    let whitelist_pairs = fetch_whitelist_pairs(db, game_id).await?;

    let cancel_for_blocking = Arc::clone(&cancel_flag);
    let outcome = tokio::task::spawn_blocking(move || {
        run_pipeline_blocking(
            candidates,
            cancel_for_blocking,
            total_folders,
            path_to_mod_id,
            whitelist_pairs,
        )
    })
    .await?;

    Ok(outcome)
}

fn run_pipeline_blocking(
    candidates: Vec<ModCandidate>,
    cancel_flag: Arc<AtomicBool>,
    total_folders: usize,
    path_to_mod_id: HashMap<String, (String, bool)>,
    whitelist_pairs: HashSet<(String, String)>,
) -> DedupScanOutcome {
    // Each snapshot is an independent folder walk + per-file stat + INI read.
    // The hashing phase below is already parallel; this is the heavier half.
    let snapshots: Vec<ModSnapshot> = candidates
        .par_iter()
        .filter_map(|candidate| collect_snapshot(candidate).ok())
        .collect();

    if is_cancelled(&cancel_flag) {
        return cancelled(total_folders);
    }

    let candidate_pairs = phase1_candidate_filtering(&snapshots);
    let candidate_pairs = apply_modpack_filter(candidate_pairs, &snapshots);
    let candidate_pairs = apply_whitelist_filter(
        candidate_pairs,
        &snapshots,
        &path_to_mod_id,
        &whitelist_pairs,
    );
    if is_cancelled(&cancel_flag) {
        return cancelled(total_folders);
    }

    let pair_indices: HashSet<usize> = candidate_pairs.iter().flat_map(|(a, b)| [*a, *b]).collect();
    let hash_profiles: HashMap<usize, _> = pair_indices
        .par_iter()
        .map(|index| (*index, hash_snapshot(&snapshots[*index])))
        .collect();

    if is_cancelled(&cancel_flag) {
        return cancelled(total_folders);
    }

    let scored_pairs: Vec<ScoredPair> = candidate_pairs
        .into_iter()
        .filter_map(|(left, right)| {
            let left_hash = hash_profiles.get(&left)?;
            let right_hash = hash_profiles.get(&right)?;
            let (score, signals, reason) =
                aggregate_signals(&snapshots[left], &snapshots[right], left_hash, right_hash);
            if score < super::signals::weights::MIN_REPORTED_SCORE {
                return None;
            }
            Some((left, right, score, signals, reason))
        })
        .collect();

    DedupScanOutcome {
        status: DedupScanStatus::Completed,
        groups: build_groups(&snapshots, &scored_pairs, &path_to_mod_id),
        total_folders,
    }
}

fn apply_modpack_filter(
    candidate_pairs: Vec<(usize, usize)>,
    snapshots: &[ModSnapshot],
) -> Vec<(usize, usize)> {
    let mut variant_container_cache: HashMap<std::path::PathBuf, bool> = HashMap::new();

    candidate_pairs
        .into_iter()
        .filter(|(left_index, right_index)| {
            let left_path = &snapshots[*left_index].candidate.path;
            let right_path = &snapshots[*right_index].candidate.path;

            let left_parent = left_path.parent();
            let right_parent = right_path.parent();

            // If they share exactly the same parent directory
            if let (Some(lp), Some(rp)) = (left_parent, right_parent) {
                if lp == rp {
                    // Check if this parent is a VariantContainer
                    let parent_path = lp.to_path_buf();

                    let is_variant_container = *variant_container_cache
                        .entry(parent_path.clone())
                        .or_insert_with(|| {
                            let (node_type, _, _) =
                                crate::common::classifier::classify_folder(&parent_path);
                            node_type == crate::common::classifier::NodeType::VariantContainer
                                || node_type == crate::common::classifier::NodeType::ModPackRoot
                                || node_type == crate::common::classifier::NodeType::FlatModRoot
                        });

                    if is_variant_container {
                        // They are variants in the same modpack, DO NOT match them as duplicates
                        return false;
                    }
                }
            }

            true
        })
        .collect()
}

fn apply_whitelist_filter(
    candidate_pairs: Vec<(usize, usize)>,
    snapshots: &[ModSnapshot],
    path_to_mod_id: &HashMap<String, (String, bool)>,
    whitelist_pairs: &HashSet<(String, String)>,
) -> Vec<(usize, usize)> {
    candidate_pairs
        .into_iter()
        .filter(|(left_index, right_index)| {
            let left_path = snapshots[*left_index]
                .candidate
                .path
                .to_string_lossy()
                .to_string();
            let right_path = snapshots[*right_index]
                .candidate
                .path
                .to_string_lossy()
                .to_string();

            let Some((left_id, _)) = path_to_mod_id.get(&left_path) else {
                return true;
            };
            let Some((right_id, _)) = path_to_mod_id.get(&right_path) else {
                return true;
            };

            let key = canonical_pair(left_id, right_id);
            !whitelist_pairs.contains(&key)
        })
        .collect()
}

fn build_candidates(mod_rows: &[(String, String, bool)], mods_root: &Path) -> Vec<ModCandidate> {
    let mut candidates = Vec::new();

    for (_id, folder_path, _is_safe) in mod_rows {
        let path = Path::new(folder_path);

        // Skip paths that no longer physically exist (`is_dir` is false for those too).
        if !path.is_dir() {
            continue;
        }

        // Also skip if it's identical to mods_root
        if path == mods_root {
            continue;
        }

        let raw_name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        let is_disabled = crate::common::normalizer::is_disabled_folder(&raw_name);
        let display_name =
            crate::common::normalizer::normalize_display_name(&raw_name).into_owned();

        candidates.push(ModCandidate {
            path: path.to_path_buf(),
            raw_name,
            display_name,
            is_disabled,
        });
    }

    candidates
}

async fn fetch_whitelist_pairs(
    db: &SqlitePool,
    game_id: &str,
) -> Result<HashSet<(String, String)>, ScannerError> {
    let rows = crate::repo::dedup_repo::get_duplicate_whitelist_pairs(db, game_id).await?;

    let mut pairs = HashSet::new();
    for (folder_a_id, folder_b_id) in rows {
        pairs.insert(canonical_pair(&folder_a_id, &folder_b_id));
    }

    Ok(pairs)
}

fn canonical_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

/// The pairs cheap enough to be worth hashing and scoring.
///
/// Both surviving predicates are one-dimensional range tests, so the pair set
/// can be *enumerated* rather than filtered out of all N(N-1)/2 combinations —
/// which is ~50M serial iterations at the 10k-mod design target, sitting
/// between two `par_iter` phases.
///
/// Sorting by file count is what makes the window exact: the count bound is
/// the tighter of the two and it is integral, so once the delta exceeds the
/// window every later snapshot is out of range and the run ends. The size
/// ratio is not monotonic along that order, so it stays a test inside the
/// window rather than a second bound.
fn phase1_candidate_filtering(snapshots: &[ModSnapshot]) -> Vec<(usize, usize)> {
    use super::signals::weights as w;

    // A folder with no files can never pair; dropping it here keeps it out of
    // every window below.
    let mut by_file_count: Vec<usize> = (0..snapshots.len())
        .filter(|index| !snapshots[*index].files.is_empty())
        .collect();
    by_file_count.sort_unstable_by_key(|index| snapshots[*index].files.len());

    let mut pairs = Vec::new();
    for (position, &left) in by_file_count.iter().enumerate() {
        let left_count = snapshots[left].files.len();
        for &right in &by_file_count[position + 1..] {
            // Ascending order: once the window closes it stays closed.
            if snapshots[right].files.len() - left_count > w::CANDIDATE_FILE_COUNT_WINDOW {
                break;
            }
            let ratio = super::size_ratio(
                snapshots[left].total_size_bytes,
                snapshots[right].total_size_bytes,
            );
            if ratio < w::CANDIDATE_MIN_SIZE_RATIO {
                continue;
            }
            pairs.push((left.min(right), left.max(right)));
        }
    }

    // Emit in the same order the exhaustive loop did. `build_groups` breaks
    // score ties by position, so preserving the order keeps this a pure
    // speedup instead of a silent change to the reason text a group reports.
    // Only the degenerate case (every mod the same size and file count) makes
    // this sort large, and there the pair hashing dwarfs it.
    pairs.sort_unstable();
    pairs
}

fn build_groups(
    snapshots: &[ModSnapshot],
    pairs: &[ScoredPair],
    path_to_mod_id: &HashMap<String, (String, bool)>,
) -> Vec<DupScanGroup> {
    let mut parent: Vec<usize> = (0..snapshots.len()).collect();
    for (left, right, _, _, _) in pairs {
        union(&mut parent, *left, *right);
    }

    // Bucket the pairs by component once. Re-scanning every pair per component
    // and testing `members.contains` made assembly
    // O(#components x #pairs x |members|); the union-find roots the loop above
    // just computed answer the same question in one pass. Both endpoints of a
    // pair share a root by construction, so keying on the left one is enough.
    let mut pairs_by_root: HashMap<usize, Vec<&ScoredPair>> = HashMap::new();
    for pair in pairs {
        let root = find(&mut parent, pair.0);
        pairs_by_root.entry(root).or_default().push(pair);
    }

    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for index in 0..snapshots.len() {
        let root = find(&mut parent, index);
        components.entry(root).or_default().push(index);
    }

    components
        .into_iter()
        .filter(|(_, members)| members.len() > 1)
        .enumerate()
        .map(|(group_index, (root, members))| {
            let component_pairs: &[&ScoredPair] = pairs_by_root
                .get(&root)
                .map_or(&[], |bucket| bucket.as_slice());
            let confidence = component_pairs
                .iter()
                .map(|(_, _, score, _, _)| *score)
                .max()
                .unwrap_or(0);
            let reason = component_pairs
                .iter()
                .max_by_key(|(_, _, score, _, _)| *score)
                .map(|(_, _, _, _, reason)| reason.clone())
                .unwrap_or_else(|| "Low confidence - manual review required".to_string());

            let mut signal_map: HashMap<String, DupScanSignal> = HashMap::new();
            for (_, _, _, signals, _) in component_pairs {
                for signal in signals {
                    signal_map
                        .entry(signal.key.clone())
                        .and_modify(|current| {
                            if signal.score > current.score {
                                *current = signal.clone();
                            }
                        })
                        .or_insert_with(|| signal.clone());
                }
            }
            let signals: Vec<DupScanSignal> = signal_map.into_values().collect();

            let members: Vec<DupScanMember> = members
                .iter()
                .map(|member_idx| {
                    let snapshot = &snapshots[*member_idx];
                    let folder_path = snapshot.candidate.path.to_string_lossy().to_string();
                    let (mod_id, is_safe) = path_to_mod_id
                        .get(&folder_path)
                        .map(|(id, safe)| (Some(id.clone()), *safe))
                        .unwrap_or((None, true));

                    DupScanMember {
                        mod_id,
                        version: None,
                        folder_path,
                        display_name: snapshot.candidate.display_name.clone(),
                        total_size_bytes: snapshot.total_size_bytes,
                        file_count: snapshot.files.len() as u64,
                        is_safe,
                        confidence_score: confidence,
                        signals: signals.clone(),
                    }
                })
                .collect();

            let is_unsafe = members.iter().any(|m: &DupScanMember| !m.is_safe);

            DupScanGroup {
                group_id: format!("dup-group-{}", group_index + 1),
                confidence_score: confidence,
                match_reason: reason,
                is_unsafe,
                signals,
                members,
            }
        })
        .collect()
}

fn is_cancelled(cancel_flag: &AtomicBool) -> bool {
    cancel_flag.load(Ordering::Relaxed)
}

fn cancelled(total_folders: usize) -> DedupScanOutcome {
    DedupScanOutcome {
        status: DedupScanStatus::Cancelled,
        groups: Vec::new(),
        total_folders,
    }
}

fn find(parent: &mut [usize], index: usize) -> usize {
    if parent[index] == index {
        return index;
    }
    let root = find(parent, parent[index]);
    parent[index] = root;
    root
}

fn union(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find(parent, left);
    let right_root = find(parent, right);
    if left_root != right_root {
        parent[right_root] = left_root;
    }
}

#[cfg(test)]
#[path = "tests/dedup_scanner_tests.rs"]
mod dedup_scanner_tests;

#[cfg(test)]
#[path = "tests/dedup_phase1_tests.rs"]
mod dedup_phase1_tests;
