//! Turning scored pairs into the groups the user sees.
//!
//! Union-find over the pairs, then one group per component. Split out of
//! `scanner`, which was holding both the scan pipeline and this assembly.

use std::collections::HashMap;

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
