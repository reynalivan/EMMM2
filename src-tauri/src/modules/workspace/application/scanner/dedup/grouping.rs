//! Turning scored pairs into the groups the user sees.
//!
//! Union-find over the pairs, then one group per component. Split out of
//! `scanner`, which was holding both the scan pipeline and this assembly.

use std::collections::HashMap;

use crate::shared::path_key::canonical_path_key_for_path;
use crate::types::dup_scan::{DupScanGroup, DupScanMember, DupScanSignal};

use super::snapshot::ModSnapshot;

/// `(left, right, score, signals, reason)` for one pair that scored above the
/// reporting floor.
pub(super) type ScoredPair = (usize, usize, u8, Vec<DupScanSignal>, String);

pub(super) fn build_groups(
    snapshots: &[ModSnapshot],
    pairs: &[ScoredPair],
    path_to_mod_id: &HashMap<String, (String, bool)>,
) -> Vec<DupScanGroup> {
    let mut parent: Vec<usize> = (0..snapshots.len()).collect();
    for (left, right, score, _, _) in pairs {
        if *score == 100 {
            union(&mut parent, *left, *right);
        }
    }

    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for index in 0..snapshots.len() {
        let root = find(&mut parent, index);
        components.entry(root).or_default().push(index);
    }

    let roots: Vec<usize> = (0..snapshots.len())
        .map(|index| find(&mut parent, index))
        .collect();
    let mut exact_components: Vec<Vec<usize>> = components
        .into_values()
        .filter(|members| members.len() > 1)
        .collect();
    exact_components.sort_unstable_by_key(|members| members[0]);

    let mut group_specs: Vec<(Vec<usize>, Vec<&ScoredPair>)> = exact_components
        .into_iter()
        .map(|members| {
            let root = roots[members[0]];
            let component_pairs = pairs
                .iter()
                .filter(|pair| pair.2 == 100 && roots[pair.0] == root)
                .collect();
            (members, component_pairs)
        })
        .collect();

    group_specs.extend(
        pairs
            .iter()
            .filter(|pair| pair.2 < 100)
            .map(|pair| (vec![pair.0, pair.1], vec![pair])),
    );

    group_specs
        .into_iter()
        .enumerate()
        .map(|(group_index, (members, component_pairs))| {
            build_group(
                group_index,
                snapshots,
                &members,
                &component_pairs,
                path_to_mod_id,
            )
        })
        .collect()
}

fn build_group(
    group_index: usize,
    snapshots: &[ModSnapshot],
    member_indices: &[usize],
    pairs: &[&ScoredPair],
    path_to_mod_id: &HashMap<String, (String, bool)>,
) -> DupScanGroup {
    let confidence = pairs
        .iter()
        .map(|(_, _, score, _, _)| *score)
        .max()
        .unwrap_or(0);
    let reason = pairs
        .iter()
        .max_by_key(|(_, _, score, _, _)| *score)
        .map(|(_, _, _, _, reason)| reason.clone())
        .unwrap_or_else(|| "Low confidence - manual review required".to_string());

    let mut signal_map: HashMap<String, DupScanSignal> = HashMap::new();
    for (_, _, _, signals, _) in pairs {
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

    let members: Vec<DupScanMember> = member_indices
        .iter()
        .map(|member_idx| {
            let snapshot = &snapshots[*member_idx];
            let folder_path = snapshot.candidate.path.to_string_lossy().to_string();
            let folder_key = canonical_path_key_for_path(&snapshot.candidate.path);
            let (mod_id, is_safe) = path_to_mod_id
                .get(&folder_key)
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
    let is_unsafe = members.iter().any(|member| !member.is_safe);

    DupScanGroup {
        group_id: format!("dup-group-{}", group_index + 1),
        confidence_score: confidence,
        match_reason: reason,
        is_unsafe,
        signals,
        members,
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
#[path = "tests/dedup_grouping_tests.rs"]
mod tests;
