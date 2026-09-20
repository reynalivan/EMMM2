use crate::modules::duplicates::domain::dup_scan::DupScanGroup;
use crate::modules::workspace::application::scanner::core::walker::{self, ModCandidate};
use crate::shared::errors::ScannerError;
use rayon::prelude::*;

use sqlx::SqlitePool;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::grouping::{build_groups, ScoredPair};
use super::hashing::{hash_snapshot, hash_snapshot_full, HashProfile};
use super::signals::aggregate_signals;
use super::snapshot::{collect_snapshot, ModSnapshot};
use crate::shared::path_key::canonical_path_key_for_path;

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

#[derive(Default)]
struct ExactClusterCompression {
    cluster_by_index: Vec<Option<usize>>,
    spanning_pairs: Vec<(usize, usize)>,
    initial_profiles: HashMap<usize, HashProfile>,
    full_profiles: HashMap<usize, HashProfile>,
}

pub async fn scan_duplicates(
    mods_root: &Path,
    game_id: &str,
    db: &SqlitePool,
    cancel_flag: Arc<AtomicBool>,
) -> Result<DedupScanOutcome, ScannerError> {
    let candidates = walker::scan_mod_folders(mods_root)?;
    scan_duplicates_for_candidates(mods_root, candidates, game_id, db, cancel_flag).await
}

/// Scan a candidate snapshot that was already enumerated for progress reporting.
/// File contents and metadata are still read from disk by the snapshot phase.
pub async fn scan_duplicates_for_candidates(
    mods_root: &Path,
    candidates: Vec<ModCandidate>,
    game_id: &str,
    db: &SqlitePool,
    cancel_flag: Arc<AtomicBool>,
) -> Result<DedupScanOutcome, ScannerError> {
    // One read of the mods table feeds both the scan candidates and the
    // path -> (mod id, is_safe) lookup used when grouping the results.
    let mut conn = db.acquire().await?;
    let mod_rows = crate::modules::library::adapters::sqlite::mods::get_all_mods_id_and_paths_tx(
        &mut conn, game_id,
    )
    .await?;
    drop(conn);

    let total_folders = candidates.len();

    if is_cancelled(&cancel_flag) {
        return Ok(cancelled(total_folders));
    }

    // Keyed by the resolved absolute path, because that is what a snapshot
    // carries. Keying by the stored value made every lookup miss.
    let path_to_mod_id: HashMap<String, (String, bool)> = mod_rows
        .into_iter()
        .map(|(id, folder_path, is_safe)| {
            let absolute = folder_path.resolve(mods_root);
            (canonical_path_key_for_path(&absolute), (id, is_safe))
        })
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

    let exact_compression =
        build_exact_cluster_compression(&snapshots, &path_to_mod_id, &whitelist_pairs);
    let mut candidate_pairs = phase1_candidate_filtering_without_exact_cliques(
        &snapshots,
        &exact_compression.cluster_by_index,
    );
    candidate_pairs.extend(exact_compression.spanning_pairs.iter().copied());
    candidate_pairs.sort_unstable();
    candidate_pairs.dedup();
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
    let mut hash_profiles = exact_compression.initial_profiles;
    let missing_initial_indices: Vec<usize> = pair_indices
        .iter()
        .copied()
        .filter(|index| !hash_profiles.contains_key(index))
        .collect();
    hash_profiles.extend(
        missing_initial_indices
            .par_iter()
            .map(|index| (*index, hash_snapshot(&snapshots[*index])))
            .collect::<HashMap<_, _>>(),
    );

    if is_cancelled(&cancel_flag) {
        return cancelled(total_folders);
    }

    let preliminary_pairs: Vec<ScoredPair> = candidate_pairs
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

    let full_hash_indices: HashSet<usize> = preliminary_pairs
        .iter()
        .filter(|(_, _, score, _, _)| *score == 100)
        .flat_map(|(left, right, _, _, _)| [*left, *right])
        .collect();
    let mut full_hash_profiles = exact_compression.full_profiles;
    let missing_full_indices: Vec<usize> = full_hash_indices
        .iter()
        .copied()
        .filter(|index| !full_hash_profiles.contains_key(index))
        .collect();
    full_hash_profiles.extend(
        missing_full_indices
            .par_iter()
            .map(|index| (*index, hash_snapshot_full(&snapshots[*index])))
            .collect::<HashMap<_, _>>(),
    );
    let scored_pairs: Vec<ScoredPair> = preliminary_pairs
        .into_iter()
        .filter_map(|pair| {
            if pair.2 < 100 {
                return Some(pair);
            }
            let left_hash = full_hash_profiles.get(&pair.0)?;
            let right_hash = full_hash_profiles.get(&pair.1)?;
            let (score, signals, reason) = aggregate_signals(
                &snapshots[pair.0],
                &snapshots[pair.1],
                left_hash,
                right_hash,
            );
            (score >= super::signals::weights::MIN_REPORTED_SCORE)
                .then_some((pair.0, pair.1, score, signals, reason))
        })
        .collect();

    DedupScanOutcome {
        status: DedupScanStatus::Completed,
        groups: build_groups(&snapshots, &scored_pairs, &path_to_mod_id),
        total_folders,
    }
}

/// Compress a complete graph of content-identical candidates into a spanning
/// forest. Exact duplicates are rendered as groups, not one result per pair,
/// so retaining every clique edge only increases CPU and memory use.
fn build_exact_cluster_compression(
    snapshots: &[ModSnapshot],
    path_to_mod_id: &HashMap<String, (String, bool)>,
    whitelist_pairs: &HashSet<(String, String)>,
) -> ExactClusterCompression {
    let mut compression = ExactClusterCompression {
        cluster_by_index: vec![None; snapshots.len()],
        ..Default::default()
    };
    let mod_ids: Vec<Option<&str>> = snapshots
        .iter()
        .map(|snapshot| {
            path_to_mod_id
                .get(&canonical_path_key_for_path(&snapshot.candidate.path))
                .map(|(mod_id, _)| mod_id.as_str())
        })
        .collect();
    let mut next_cluster_id = 0;

    for layout_group in structurally_equivalent_groups(snapshots) {
        let initial_profiles: Vec<(usize, HashProfile)> = layout_group
            .par_iter()
            .map(|index| (*index, hash_snapshot(&snapshots[*index])))
            .collect();
        let initial_match_groups = matching_profile_groups(&initial_profiles);
        compression.initial_profiles.extend(initial_profiles);

        for initial_match_group in initial_match_groups {
            let full_profiles: Vec<(usize, HashProfile)> = initial_match_group
                .par_iter()
                .map(|index| (*index, hash_snapshot_full(&snapshots[*index])))
                .collect();
            let exact_groups = matching_profile_groups(&full_profiles);
            compression.full_profiles.extend(full_profiles);

            for exact_group in exact_groups {
                for (members, edges) in
                    exact_components_after_whitelist(&exact_group, &mod_ids, whitelist_pairs)
                {
                    for index in members {
                        compression.cluster_by_index[index] = Some(next_cluster_id);
                    }
                    compression.spanning_pairs.extend(edges);
                    next_cluster_id += 1;
                }
            }
        }
    }

    compression
}

fn structurally_equivalent_groups(snapshots: &[ModSnapshot]) -> Vec<Vec<usize>> {
    let mut groups_by_fingerprint: HashMap<String, Vec<Vec<usize>>> = HashMap::new();

    for (index, snapshot) in snapshots.iter().enumerate() {
        if snapshot.files.is_empty() {
            continue;
        }
        let groups = groups_by_fingerprint
            .entry(layout_fingerprint(snapshot))
            .or_default();
        if let Some(group) = groups.iter_mut().find(|members| {
            members
                .first()
                .is_some_and(|first| same_file_layout(&snapshots[*first], snapshot))
        }) {
            group.push(index);
        } else {
            groups.push(vec![index]);
        }
    }

    groups_by_fingerprint
        .into_values()
        .flatten()
        .filter(|members| members.len() > 1)
        .collect()
}

fn layout_fingerprint(snapshot: &ModSnapshot) -> String {
    let mut files: Vec<_> = snapshot.files.iter().collect();
    files.sort_unstable_by(|left, right| left.rel_path.cmp(&right.rel_path));

    let mut hasher = blake3::Hasher::new();
    hasher.update(&(files.len() as u64).to_le_bytes());
    for file in files {
        hasher.update(&(file.rel_path.len() as u64).to_le_bytes());
        hasher.update(file.rel_path.as_bytes());
        hasher.update(&file.size_bytes.to_le_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn same_file_layout(left: &ModSnapshot, right: &ModSnapshot) -> bool {
    if left.files.len() != right.files.len() {
        return false;
    }

    let mut left_files: Vec<_> = left.files.iter().collect();
    let mut right_files: Vec<_> = right.files.iter().collect();
    left_files.sort_unstable_by(|first, second| first.rel_path.cmp(&second.rel_path));
    right_files.sort_unstable_by(|first, second| first.rel_path.cmp(&second.rel_path));
    left_files.iter().zip(right_files).all(|(first, second)| {
        first.rel_path == second.rel_path && first.size_bytes == second.size_bytes
    })
}

fn matching_profile_groups(profiles: &[(usize, HashProfile)]) -> Vec<Vec<usize>> {
    let profiles_by_index: HashMap<usize, &HashProfile> = profiles
        .iter()
        .map(|(index, profile)| (*index, profile))
        .collect();
    let mut groups_by_fingerprint: HashMap<String, Vec<Vec<usize>>> = HashMap::new();

    for (index, profile) in profiles {
        if profile.file_hashes.is_empty() {
            continue;
        }
        let groups = groups_by_fingerprint
            .entry(profile_fingerprint(profile))
            .or_default();
        if let Some(group) = groups.iter_mut().find(|members| {
            members.first().is_some_and(|first| {
                profiles_by_index
                    .get(first)
                    .is_some_and(|other| other.file_hashes == profile.file_hashes)
            })
        }) {
            group.push(*index);
        } else {
            groups.push(vec![*index]);
        }
    }

    groups_by_fingerprint
        .into_values()
        .flatten()
        .filter(|members| members.len() > 1)
        .collect()
}

fn profile_fingerprint(profile: &HashProfile) -> String {
    let mut hasher = blake3::Hasher::new();
    for (path, hash) in &profile.file_hashes {
        hasher.update(&(path.len() as u64).to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update(&(hash.len() as u64).to_le_bytes());
        hasher.update(hash.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

type ExactComponent = (Vec<usize>, Vec<(usize, usize)>);

fn exact_components_after_whitelist(
    members: &[usize],
    mod_ids: &[Option<&str>],
    whitelist_pairs: &HashSet<(String, String)>,
) -> Vec<ExactComponent> {
    if whitelist_pairs.is_empty() {
        return vec![(
            members.to_vec(),
            members.windows(2).map(|pair| (pair[0], pair[1])).collect(),
        )];
    }

    let mut unvisited: BTreeSet<usize> = members.iter().copied().collect();
    let mut components = Vec::new();
    while let Some(seed) = unvisited.first().copied() {
        unvisited.remove(&seed);
        let mut queue = VecDeque::from([seed]);
        let mut component_members = vec![seed];
        let mut component_edges = Vec::new();

        while let Some(current) = queue.pop_front() {
            let connected: Vec<usize> = unvisited
                .iter()
                .copied()
                .filter(|candidate| {
                    !pair_is_whitelisted(current, *candidate, mod_ids, whitelist_pairs)
                })
                .collect();
            for candidate in connected {
                unvisited.remove(&candidate);
                component_members.push(candidate);
                component_edges.push((current.min(candidate), current.max(candidate)));
                queue.push_back(candidate);
            }
        }

        components.push((component_members, component_edges));
    }

    components
}

fn pair_is_whitelisted(
    left: usize,
    right: usize,
    mod_ids: &[Option<&str>],
    whitelist_pairs: &HashSet<(String, String)>,
) -> bool {
    let (Some(left_id), Some(right_id)) = (mod_ids[left], mod_ids[right]) else {
        return false;
    };
    whitelist_pairs.contains(&canonical_pair(left_id, right_id))
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
            let left_path = canonical_path_key_for_path(&snapshots[*left_index].candidate.path);
            let right_path = canonical_path_key_for_path(&snapshots[*right_index].candidate.path);

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

async fn fetch_whitelist_pairs(
    db: &SqlitePool,
    game_id: &str,
) -> Result<HashSet<(String, String)>, ScannerError> {
    let rows = crate::modules::duplicates::adapters::sqlite::dedup::get_duplicate_whitelist_pairs(
        db, game_id,
    )
    .await?;

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
#[cfg(test)]
#[cfg(test)]
fn phase1_candidate_filtering(snapshots: &[ModSnapshot]) -> Vec<(usize, usize)> {
    phase1_candidate_filtering_without_exact_cliques(snapshots, &[])
}

fn phase1_candidate_filtering_without_exact_cliques(
    snapshots: &[ModSnapshot],
    exact_cluster_by_index: &[Option<usize>],
) -> Vec<(usize, usize)> {
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
            if exact_cluster_by_index
                .get(left)
                .copied()
                .flatten()
                .is_some_and(|cluster| {
                    exact_cluster_by_index.get(right).copied().flatten() == Some(cluster)
                })
            {
                continue;
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

#[cfg(test)]
#[path = "tests/dedup_scanner_tests.rs"]
mod dedup_scanner_tests;

#[cfg(test)]
#[path = "tests/dedup_phase1_tests.rs"]
mod dedup_phase1_tests;
