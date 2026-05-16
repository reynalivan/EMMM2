use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::domain::collection::{
    CollectionMod, CollectionObject, CollectionRoot, MemberKind, PreviewTreeNode,
    PreviewTreeNodeKind, ProjectedActiveRoot, ProjectedCollectionState, ProjectedObjectState,
    ProjectedStateSummary,
};
use crate::services::collection_preview_tree::resolve_preview_terminal_metadata;
use crate::services::path_key::{
    canonical_collection_path_key, canonical_name_key, resolve_collection_path,
};
use crate::services::scanner::core::normalizer::normalize_display_name;

const ROOT_TYPE_MODPACK: &str = "ModPackRoot";
const ROOT_TYPE_FLAT: &str = "FlatModRoot";
const ROOT_TYPE_VARIANT: &str = "VariantContainer";

pub fn empty_projected_state() -> ProjectedCollectionState {
    ProjectedCollectionState {
        object_states: Vec::new(),
        active_roots: Vec::new(),
        summary: ProjectedStateSummary {
            object_count: 0,
            enabled_object_count: 0,
            active_root_count: 0,
            missing_root_count: 0,
        },
    }
}

pub fn parse_snapshot_json(snapshot_json: &str) -> Option<ProjectedCollectionState> {
    serde_json::from_str::<ProjectedCollectionState>(snapshot_json).ok()
}

pub fn serialize_snapshot_json(state: &ProjectedCollectionState) -> Option<String> {
    serde_json::to_string(state).ok()
}

pub fn build_projected_state(
    mods: &[CollectionMod],
    objects: &[CollectionObject],
    mods_path: Option<&str>,
) -> ProjectedCollectionState {
    let mut object_states = objects
        .iter()
        .map(|object| ProjectedObjectState {
            object_id: object.object_id.clone(),
            display_name: object
                .display_name
                .clone()
                .unwrap_or_else(|| object.object_id.clone()),
            path_key: object
                .path_key
                .clone()
                .unwrap_or_else(|| object.object_id.clone()),
            is_enabled: object.is_enabled,
            active_root_count: 0,
        })
        .collect::<Vec<_>>();

    let object_lookup = objects
        .iter()
        .map(|object| (object.object_id.as_str(), object))
        .collect::<HashMap<_, _>>();
    let mut root_map = BTreeMap::<(String, String), ProjectedActiveRoot>::new();

    for member in mods.iter().filter(|member| member.is_enabled) {
        let object = object_lookup.get(member.object_id.as_str()).copied();
        let metadata = resolve_preview_terminal_metadata(object, member, mods_path);
        let Some(root_type) = metadata.node_type.as_deref() else {
            continue;
        };
        if !is_visible_root_type(root_type) {
            continue;
        }

        let source_path = metadata
            .preview_path
            .as_deref()
            .map(|path| relative_source_path(path, mods_path))
            .unwrap_or_else(|| member.mod_path.clone());
        let root_key =
            canonical_collection_path_key(&source_path, mods_path).unwrap_or_else(|| {
                member
                    .mod_path_key
                    .clone()
                    .unwrap_or_else(|| canonical_name_key(&source_path))
            });
        let display_name = root_display_name(&source_path, member.display_name.as_deref());
        let is_missing = source_is_missing(&source_path, mods_path);
        let key = (member.object_id.clone(), root_key.clone());

        root_map
            .entry(key)
            .and_modify(|existing| {
                merge_warnings(&mut existing.warnings, &metadata.warnings);
                if existing.thumbnail_hint.is_none() {
                    existing.thumbnail_hint = metadata.preview_path.clone();
                }
            })
            .or_insert_with(|| ProjectedActiveRoot {
                object_id: member.object_id.clone(),
                root_key,
                display_name,
                root_type: root_type.to_string(),
                source_path,
                thumbnail_hint: metadata.preview_path.clone(),
                warnings: metadata.warnings.clone(),
                is_missing,
            });
    }

    let active_roots = root_map.into_values().collect::<Vec<_>>();
    let mut root_count_by_object = HashMap::<String, usize>::new();
    for root in &active_roots {
        let counter = root_count_by_object
            .entry(root.object_id.clone())
            .or_insert(0);
        *counter += 1;
    }

    for object_state in &mut object_states {
        object_state.active_root_count = root_count_by_object
            .get(&object_state.object_id)
            .copied()
            .unwrap_or(0);
    }

    object_states.sort_by(|left, right| {
        canonical_name_key(&left.display_name).cmp(&canonical_name_key(&right.display_name))
    });

    let mut active_roots = active_roots;
    active_roots.sort_by(|left, right| {
        let left_key = format!(
            "{}:{}",
            canonical_name_key(&left.display_name),
            canonical_name_key(&left.object_id)
        );
        let right_key = format!(
            "{}:{}",
            canonical_name_key(&right.display_name),
            canonical_name_key(&right.object_id)
        );
        left_key.cmp(&right_key)
    });

    let enabled_object_count = object_states
        .iter()
        .filter(|object| object.is_enabled)
        .count();
    let missing_root_count = active_roots.iter().filter(|root| root.is_missing).count();
    let active_root_count = active_roots.len();
    let object_count = object_states.len();

    ProjectedCollectionState {
        object_states,
        active_roots,
        summary: ProjectedStateSummary {
            object_count,
            enabled_object_count,
            active_root_count,
            missing_root_count,
        },
    }
}

pub fn signature_for_projected_state(state: &ProjectedCollectionState) -> String {
    let mut entries = state
        .active_roots
        .iter()
        .filter(|root| !root.is_missing)
        .map(|root| {
            format!(
                "root:{}:{}:{}",
                root.object_id, root.root_type, root.root_key
            )
        })
        .collect::<Vec<_>>();

    entries.extend(state.object_states.iter().map(|object| {
        format!(
            "object:{}:{}",
            object.path_key,
            if object.is_enabled { "1" } else { "0" }
        )
    }));
    entries.sort();

    blake3::hash(entries.join("\n").as_bytes())
        .to_hex()
        .to_string()
}

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
            children.sort_by(|left, right| {
                canonical_name_key(&left.name).cmp(&canonical_name_key(&right.name))
            });

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

fn is_visible_root_type(root_type: &str) -> bool {
    matches!(
        root_type,
        ROOT_TYPE_MODPACK | ROOT_TYPE_FLAT | ROOT_TYPE_VARIANT
    )
}

fn relative_source_path(path: &str, mods_path: Option<&str>) -> String {
    let Some(mods_root) = mods_path else {
        return path.to_string();
    };

    let absolute_path = Path::new(path);
    let absolute_root = Path::new(mods_root);
    let Ok(relative_path) = absolute_path.strip_prefix(absolute_root) else {
        return path.to_string();
    };

    relative_path.to_string_lossy().replace('\\', "/")
}

fn root_display_name(source_path: &str, fallback: Option<&str>) -> String {
    let path_name = Path::new(source_path)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty());

    normalize_display_name(path_name.as_deref().or(fallback).unwrap_or("Unnamed Mod"))
}

fn source_is_missing(source_path: &str, mods_path: Option<&str>) -> bool {
    let Some(resolved_path) = resolve_collection_path(source_path, mods_path) else {
        return false;
    };

    !resolved_path.exists()
}

fn merge_warnings(target: &mut Vec<String>, source: &[String]) {
    for warning in source {
        if target.iter().any(|existing| existing == warning) {
            continue;
        }
        target.push(warning.clone());
    }
}
