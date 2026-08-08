//! Terminal-node rules: which folder in a nested path counts as "the mod",
//! and the per-object totals that follow from it.
//!
//! This lived in `repo::object_repo::counts` and walked the disk from inside
//! the data-access layer, once per row per ancestor with no memo. Resolving a
//! terminal reads directories and parses INI headers, so it belongs here.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::common::classifier::{classify_folder, NodeType};
use crate::common::normalizer::is_disabled_folder;
use crate::common::path_key::{canonical_name_key, folder_path_key, resolve_collection_path};
use crate::domain::models::ItemStatus;
use crate::domain::objects::ObjectSummary;
use crate::repo::object_repo::{ObjectCountCandidate, TerminalDescriptor};

/// Memoized `classify_folder`.
///
/// Sibling mods share ancestors, so the unmemoized walk re-read the same
/// directory (and its INI files) once per sibling per level.
#[derive(Default)]
pub(crate) struct ClassificationCache {
    seen: HashMap<PathBuf, Option<NodeType>>,
}

impl ClassificationCache {
    fn classify(&mut self, path: Option<&Path>) -> Option<NodeType> {
        let path = path?;
        if let Some(cached) = self.seen.get(path) {
            return *cached;
        }
        let resolved = classify_terminal_type(Some(path));
        self.seen.insert(path.to_path_buf(), resolved);
        resolved
    }
}

pub(crate) fn build_terminal_counts(
    objects: &[ObjectSummary],
    candidates: &[ObjectCountCandidate],
    mods_path: Option<&str>,
) -> HashMap<String, (i64, i64, Option<String>)> {
    let object_lookup: HashMap<&str, &ObjectSummary> = objects
        .iter()
        .map(|object| (object.id.as_str(), object))
        .collect();
    let mut totals_by_object: HashMap<String, HashSet<String>> = HashMap::new();
    let mut enabled_by_object: HashMap<String, HashSet<String>> = HashMap::new();
    let mut active_paths_by_object: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut cache = ClassificationCache::default();

    for candidate in candidates {
        let Some(object) = object_lookup.get(candidate.object_id.as_str()) else {
            continue;
        };
        let Some(descriptor) =
            resolve_terminal_descriptor(object, candidate, mods_path, &mut cache)
        else {
            continue;
        };

        let terminal_key = folder_path_key(&descriptor.display_path, mods_path);
        totals_by_object
            .entry(candidate.object_id.clone())
            .or_default()
            .insert(terminal_key.clone());

        if candidate.status != ItemStatus::Enabled {
            continue;
        }
        if has_disabled_ancestor(&descriptor.display_segments) {
            continue;
        }

        enabled_by_object
            .entry(candidate.object_id.clone())
            .or_default()
            .insert(terminal_key.clone());
        active_paths_by_object
            .entry(candidate.object_id.clone())
            .or_default()
            .entry(terminal_key)
            .or_insert(descriptor.display_path);
    }

    let mut counts = HashMap::new();
    for object in objects {
        let total = totals_by_object
            .get(&object.id)
            .map(|entries| entries.len() as i64)
            .unwrap_or(0);
        let enabled = enabled_by_object
            .get(&object.id)
            .map(|entries| entries.len() as i64)
            .unwrap_or(0);
        let active_paths = active_paths_by_object.get(&object.id).map(|entries| {
            let mut values: Vec<String> = entries.values().cloned().collect();
            values.sort_by_key(|value| canonical_name_key(value));
            values.join("|")
        });
        counts.insert(object.id.clone(), (total, enabled, active_paths));
    }

    counts
}

pub(crate) fn resolve_terminal_descriptor(
    object: &ObjectSummary,
    candidate: &ObjectCountCandidate,
    mods_path: Option<&str>,
    cache: &mut ClassificationCache,
) -> Option<TerminalDescriptor> {
    let relative_segments = relative_segments_for_path(
        &object.folder_path,
        &object.name,
        &candidate.folder_path,
        &candidate.actual_name,
    );
    if relative_segments.is_empty() {
        return None;
    }

    let terminal_path = resolve_collection_path(&candidate.folder_path, mods_path);
    let candidate_paths = cumulative_candidate_paths(&terminal_path, relative_segments.len());
    for candidate_path in candidate_paths {
        let Some(_node_type) = cache.classify(candidate_path.as_deref()) else {
            continue;
        };
        let display_path = candidate_path
            .as_ref()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| candidate.folder_path.clone());
        let display_segments = relative_segments_for_path(
            &object.folder_path,
            &object.name,
            &display_path,
            &candidate.actual_name,
        );
        if display_segments.is_empty() {
            continue;
        }

        return Some(TerminalDescriptor {
            display_path,
            display_segments,
        });
    }

    let node_type = cache.classify(terminal_path.as_deref())?;
    let display_path = terminal_path
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| candidate.folder_path.clone());
    let display_segments = relative_segments_for_path(
        &object.folder_path,
        &object.name,
        &display_path,
        &candidate.actual_name,
    );
    if display_segments.is_empty() {
        return None;
    }

    if !matches!(
        node_type,
        NodeType::FlatModRoot | NodeType::ModPackRoot | NodeType::VariantContainer
    ) {
        return None;
    }

    Some(TerminalDescriptor {
        display_path,
        display_segments,
    })
}

pub(crate) fn classify_terminal_type(path: Option<&Path>) -> Option<NodeType> {
    let target = path?;
    let (node_type, _reasons, _warnings) = classify_folder(target);
    if matches!(
        node_type,
        NodeType::FlatModRoot | NodeType::ModPackRoot | NodeType::VariantContainer
    ) {
        return Some(node_type);
    }
    None
}

pub(crate) fn has_disabled_ancestor(segments: &[String]) -> bool {
    segments
        .iter()
        .take(segments.len().saturating_sub(1))
        .any(|segment| is_disabled_folder(segment))
}

pub(crate) fn cumulative_candidate_paths(
    path: &Option<PathBuf>,
    segment_count: usize,
) -> Vec<Option<PathBuf>> {
    let Some(full_path) = path.clone() else {
        return Vec::new();
    };

    let mut current = full_path;
    let mut reversed = Vec::with_capacity(segment_count);
    reversed.push(Some(current.clone()));
    for _ in 1..segment_count {
        let Some(parent) = current.parent() else {
            reversed.push(None);
            continue;
        };
        let parent_path = parent.to_path_buf();
        reversed.push(Some(parent_path.clone()));
        current = parent_path;
    }
    reversed.reverse();
    reversed
}

pub(crate) fn relative_segments_for_path(
    object_folder_path: &str,
    object_name: &str,
    path: &str,
    fallback_name: &str,
) -> Vec<String> {
    let path_segments = split_segments(path);
    let anchors = [object_folder_path.to_string(), object_name.to_string()];

    for anchor in anchors {
        let anchor_segments = split_segments(&anchor);
        if anchor_segments.is_empty() || anchor_segments.len() > path_segments.len() {
            continue;
        }
        let Some(start_index) = find_anchor_start(&path_segments, &anchor_segments) else {
            continue;
        };
        let relative = path_segments[(start_index + anchor_segments.len())..].to_vec();
        if !relative.is_empty() {
            return relative;
        }
    }

    vec![path_leaf(path, fallback_name)]
}

pub(crate) fn split_segments(path: &str) -> Vec<String> {
    path.replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .collect()
}

pub(crate) fn find_anchor_start(
    path_segments: &[String],
    anchor_segments: &[String],
) -> Option<usize> {
    for index in 0..=(path_segments.len() - anchor_segments.len()) {
        let matches = anchor_segments.iter().enumerate().all(|(offset, anchor)| {
            canonical_name_key(&path_segments[index + offset]) == canonical_name_key(anchor)
        });
        if matches {
            return Some(index);
        }
    }
    None
}

pub(crate) fn path_leaf(path: &str, fallback_name: &str) -> String {
    split_segments(path)
        .last()
        .cloned()
        .unwrap_or_else(|| fallback_name.to_string())
}
