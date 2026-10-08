//! Logical-path normalization used to tell a runtime toggle (DISABLED prefix)
//! apart from a real move/rename before rewriting collection references.

use crate::modules::workspace::domain::normalizer::normalize_display_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CollectionPathTransitionKind {
    RuntimeTogglePrefix,
    SemanticMoveOrRename,
}

pub(crate) fn logical_collection_path(path: &str) -> String {
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

fn path_with_reference_separator(path: &str, reference: &str) -> String {
    if reference.contains('/') {
        path.replace('\\', "/")
    } else if reference.contains('\\') {
        path.replace('/', "\\")
    } else {
        path.to_string()
    }
}

pub(super) fn rewrite_descendant_path(
    path: &str,
    old_root: &str,
    new_root: &str,
) -> Option<String> {
    let path_key = crate::shared::path_key::folder_path_key(path, None);
    let old_key = crate::shared::path_key::folder_path_key(old_root, None);
    if path_key == old_key {
        return Some(path_with_reference_separator(new_root, path));
    }
    let key_prefix = format!("{old_key}/");
    if !path_key.starts_with(&key_prefix) {
        return None;
    }

    let root_component_count = old_root
        .split(['/', '\\'])
        .filter(|component| !component.is_empty())
        .count();
    let mut seen_components = 1;
    for (index, separator) in path.char_indices() {
        if !matches!(separator, '/' | '\\') {
            continue;
        }
        if seen_components == root_component_count {
            let remainder = &path[index + separator.len_utf8()..];
            let new_root = path_with_reference_separator(new_root, path);
            return Some(format!("{new_root}{separator}{remainder}"));
        }
        seen_components += 1;
    }
    None
}
