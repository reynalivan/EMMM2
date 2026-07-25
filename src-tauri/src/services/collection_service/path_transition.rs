//! Logical-path normalization used to tell a runtime toggle (DISABLED prefix)
//! apart from a real move/rename before rewriting collection references.

use crate::common::normalizer::normalize_display_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CollectionPathTransitionKind {
    RuntimeTogglePrefix,
    SemanticMoveOrRename,
}

pub(super) fn logical_collection_path(path: &str) -> String {
    path.split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .map(normalize_display_name)
        .collect::<Vec<_>>()
        .join("/")
}

pub(super) fn unique_reference_candidates(path: &str) -> Vec<String> {
    let logical_path = logical_collection_path(path);
    let mut candidates = vec![path.to_string()];
    if logical_path != path {
        candidates.push(logical_path);
    }
    candidates
}

pub(crate) fn classify_collection_path_transition(
    old_path: &str,
    new_path: &str,
) -> CollectionPathTransitionKind {
    if logical_collection_path(old_path) == logical_collection_path(new_path) {
        return CollectionPathTransitionKind::RuntimeTogglePrefix;
    }

    CollectionPathTransitionKind::SemanticMoveOrRename
}
