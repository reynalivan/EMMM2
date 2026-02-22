use crate::services::scanner::core::walker::{self, ModCandidate};
use crate::types::dup_scan::{DupScanGroup, DupScanMember, DupScanSignal};
use rayon::prelude::*;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::signals::{aggregate_signals, collect_snapshot, hash_snapshot, ModSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupScanStatus {
    Completed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct DedupScanOutcome {
    pub status: DedupScanStatus,
    pub groups: Vec<DupScanGroup>,
    pub total_folders: usize,
}

type ScoredPair = (usize, usize, u8, Vec<DupScanSignal>, String);

pub async fn scan_duplicates(
    mods_root: &Path,
    game_id: &str,
    db: &SqlitePool,
    cancel_flag: Arc<AtomicBool>,
) -> Result<DedupScanOutcome, String> {
    let candidates = walker::scan_mod_folders(mods_root)?;
    let total_folders = candidates.len();

    if is_cancelled(&cancel_flag) {
        return Ok(cancelled(total_folders));
    }

    let path_to_mod_id = fetch_mod_id_map(db, game_id).await?;
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
    .await
    .map_err(|error| format!("Duplicate scan worker failed: {error}"))?;

    Ok(outcome)
}

fn run_pipeline_blocking(
    candidates: Vec<ModCandidate>,
    cancel_flag: Arc<AtomicBool>,
    total_folders: usize,
    path_to_mod_id: HashMap<String, String>,
    whitelist_pairs: HashSet<(String, String)>,
) -> DedupScanOutcome {
    let snapshots: Vec<ModSnapshot> = candidates
        .iter()
        .filter_map(|candidate| collect_snapshot(candidate).ok())
        .collect();

    if is_cancelled(&cancel_flag) {
        return cancelled(total_folders);
    }

    let candidate_pairs = phase1_candidate_filtering(&snapshots);
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
            if score < 45 {
                return None;
            }
            Some((left, right, score, signals, reason))
        })
        .collect();

    DedupScanOutcome {
        status: DedupScanStatus::Completed,
        groups: build_groups(&snapshots, &scored_pairs),
        total_folders,
    }
}

fn apply_whitelist_filter(
    candidate_pairs: Vec<(usize, usize)>,
    snapshots: &[ModSnapshot],
    path_to_mod_id: &HashMap<String, String>,
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

            let Some(left_id) = path_to_mod_id.get(&left_path) else {
                return true;
            };
            let Some(right_id) = path_to_mod_id.get(&right_path) else {
                return true;
            };

            let key = canonical_pair(left_id, right_id);
            !whitelist_pairs.contains(&key)
        })
        .collect()
}

async fn fetch_mod_id_map(
    db: &SqlitePool,
    game_id: &str,
) -> Result<HashMap<String, String>, String> {
    let rows = sqlx::query("SELECT id, folder_path FROM mods WHERE game_id = ?")
        .bind(game_id)
        .fetch_all(db)
        .await
        .map_err(|error| format!("Failed to fetch mod mapping for duplicate scan: {error}"))?;

    let mut mapping = HashMap::new();
    for row in rows {
        let mod_id: String = row
            .try_get("id")
            .map_err(|error| format!("Invalid mods.id value during duplicate scan: {error}"))?;
        let folder_path: String = row.try_get("folder_path").map_err(|error| {
            format!("Invalid mods.folder_path value during duplicate scan: {error}")
        })?;
        mapping.insert(folder_path, mod_id);
    }

    Ok(mapping)
}

async fn fetch_whitelist_pairs(
    db: &SqlitePool,
    game_id: &str,
) -> Result<HashSet<(String, String)>, String> {
    let rows =
        sqlx::query("SELECT folder_a_id, folder_b_id FROM duplicate_whitelist WHERE game_id = ?")
            .bind(game_id)
            .fetch_all(db)
            .await
            .map_err(|error| format!("Failed to fetch duplicate whitelist pairs: {error}"))?;

    let mut pairs = HashSet::new();
    for row in rows {
        let folder_a_id: String = row
            .try_get("folder_a_id")
            .map_err(|error| format!("Invalid duplicate_whitelist.folder_a_id value: {error}"))?;
        let folder_b_id: String = row
            .try_get("folder_b_id")
            .map_err(|error| format!("Invalid duplicate_whitelist.folder_b_id value: {error}"))?;
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

fn phase1_candidate_filtering(snapshots: &[ModSnapshot]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for left in 0..snapshots.len() {
        for right in (left + 1)..snapshots.len() {
            let first = &snapshots[left];
            let second = &snapshots[right];
            if first.files.is_empty() || second.files.is_empty() {
                continue;
            }
            if first.files.len().abs_diff(second.files.len()) > 4 {
                continue;
            }
            if ratio(first.total_size_bytes, second.total_size_bytes) < 0.70 {
                continue;
            }
            pairs.push((left, right));
        }
    }
    pairs
}

fn build_groups(snapshots: &[ModSnapshot], pairs: &[ScoredPair]) -> Vec<DupScanGroup> {
    let mut parent: Vec<usize> = (0..snapshots.len()).collect();
    for (left, right, _, _, _) in pairs {
        union(&mut parent, *left, *right);
    }

    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for index in 0..snapshots.len() {
        let root = find(&mut parent, index);
        components.entry(root).or_default().push(index);
    }

    components
        .into_values()
        .filter(|members| members.len() > 1)
        .enumerate()
        .map(|(group_index, members)| {
            let component_pairs: Vec<_> = pairs
                .iter()
                .filter(|(left, right, _, _, _)| members.contains(left) && members.contains(right))
                .collect();
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
            for (_, _, _, signals, _) in &component_pairs {
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

            let members = members
                .iter()
                .map(|member_idx| {
                    let snapshot = &snapshots[*member_idx];
                    DupScanMember {
                        mod_id: None,
                        folder_path: snapshot.candidate.path.to_string_lossy().to_string(),
                        display_name: snapshot.candidate.display_name.clone(),
                        total_size_bytes: snapshot.total_size_bytes,
                        file_count: snapshot.files.len() as u64,
                        confidence_score: confidence,
                        signals: signals.clone(),
                    }
                })
                .collect();

            DupScanGroup {
                group_id: format!("dup-group-{}", group_index + 1),
                confidence_score: confidence,
                match_reason: reason,
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

fn ratio(left: u64, right: u64) -> f64 {
    let max = left.max(right);
    if max == 0 {
        return 0.0;
    }
    left.min(right) as f64 / max as f64
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
