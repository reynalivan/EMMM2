//! Projected state -> collection member / preview-tree DTOs.
//!
//! Pure mapping: no filesystem, no database.

use std::collections::HashMap;

use crate::common::path_key::canonical_name_key;
use crate::domain::collection::{
    CollectionMod, CollectionObject, CollectionRoot, MemberKind, PreviewTreeNode,
    PreviewTreeNodeKind, ProjectedActiveRoot, ProjectedCollectionState,
};

pub fn build_preview_tree_from_projected_state(
    state: &ProjectedCollectionState,
) -> Vec<PreviewTreeNode> {
    let mut roots_by_object = HashMap::<String, Vec<&ProjectedActiveRoot>>::new();
    for root in &state.active_roots {
        roots_by_object
            .entry(root.object_id.clone())
            .or_default()
            .push(root);
    }

    state
        .object_states
        .iter()
        .map(|object| {
            let mut children = roots_by_object
                .remove(&object.object_id)
                .unwrap_or_default()
                .into_iter()
                .map(projected_root_to_node)
                .collect::<Vec<_>>();
            children.sort_by_cached_key(|child| canonical_name_key(&child.name));

            PreviewTreeNode {
                kind: PreviewTreeNodeKind::Object,
                id: object.object_id.clone(),
                name: object.display_name.clone(),
                path: Some(object.path_key.clone()),
                object_id: Some(object.object_id.clone()),
                node_type: None,
                is_enabled: object.is_enabled,
                is_effectively_active: object.is_enabled,
                inactive_reason: None,
                show_inactive_chip: false,
                status_kind: None,
                collapse_children: false,
                warnings: Vec::new(),
                mod_count: Some(object.active_root_count),
                children,
            }
        })
        .collect()
}

pub fn mods_from_projected_state(
    collection_id: &str,
    state: &ProjectedCollectionState,
) -> Vec<CollectionMod> {
    state
        .active_roots
        .iter()
        .map(|root| CollectionMod {
            kind: MemberKind::Mod,
            collection_id: collection_id.to_string(),
            mod_id: None,
            mod_path: root.source_path.clone(),
            mod_path_key: Some(root.root_key.clone()),
            object_id: root.object_id.clone(),
            display_name: Some(root.display_name.clone()),
            preview_path: Some(root.source_path.clone()),
            node_type: Some(root.root_type.clone()),
            warnings: root.warnings.clone(),
            is_enabled: true,
        })
        .collect()
}

pub fn objects_from_projected_state(
    collection_id: &str,
    state: &ProjectedCollectionState,
) -> Vec<CollectionObject> {
    state
        .object_states
        .iter()
        .map(|object| CollectionObject {
            kind: MemberKind::Object,
            collection_id: collection_id.to_string(),
            object_id: object.object_id.clone(),
            is_enabled: object.is_enabled,
            display_name: Some(object.display_name.clone()),
            path_key: Some(object.path_key.clone()),
        })
        .collect()
}

pub fn roots_from_projected_state(
    collection_id: &str,
    is_safe: bool,
    state: &ProjectedCollectionState,
) -> Vec<CollectionRoot> {
    state
        .active_roots
        .iter()
        .map(|root| CollectionRoot {
            kind: MemberKind::Root,
            collection_id: collection_id.to_string(),
            root_path: root.source_path.clone(),
            root_path_key: root.root_key.clone(),
            display_name: root.display_name.clone(),
            display_name_key: canonical_name_key(&root.display_name),
            object_id: Some(root.object_id.clone()),
            object_name: None,
            object_type: None,
            root_kind: root.root_type.clone(),
            is_safe,
            is_enabled: true,
            thumbnail_hint: root.thumbnail_hint.clone(),
            corridor_source: None,
        })
        .collect()
}

fn projected_root_to_node(root: &ProjectedActiveRoot) -> PreviewTreeNode {
    PreviewTreeNode {
        kind: PreviewTreeNodeKind::Mod,
        id: format!("root::{}", root.root_key),
        name: root.display_name.clone(),
        path: Some(root.source_path.clone()),
        object_id: Some(root.object_id.clone()),
        node_type: Some(root.root_type.clone()),
        is_enabled: true,
        is_effectively_active: !root.is_missing,
        inactive_reason: None,
        show_inactive_chip: false,
        status_kind: if root.is_missing {
            Some("missing".to_string())
        } else {
            None
        },
        collapse_children: false,
        warnings: root.warnings.clone(),
        mod_count: None,
        children: Vec::new(),
    }
}
