use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::collection::{
    CollectionMod, CollectionObject, PreviewTreeNode, PreviewTreeNodeKind,
};
use crate::services::explorer::classifier::{classify_folder, NodeType};
use crate::services::path_key::{canonical_name_key, resolve_collection_path};
use crate::services::scanner::core::normalizer::{is_disabled_folder, normalize_display_name};

const UNCATEGORIZED_ID: &str = "__uncategorized__";
const INACTIVE_REASON: &str =
    "Children in this folder are treated as inactive because this container is disabled.";
const INACTIVE_SECTION_NODE_TYPE: &str = "InactiveContainerSection";
const STATUS_DISABLED_BY_CONTAINER: &str = "disabled_by_container";
const STATUS_INACTIVE_CONTAINER: &str = "inactive_container";

#[derive(Debug, Clone)]
pub struct PreviewTerminalMetadata {
    pub preview_path: Option<String>,
    pub node_type: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct PreviewDescriptor {
    display_path: String,
    node_type: NodeType,
    warnings: Vec<String>,
    display_segments: Vec<String>,
}

pub fn build_preview_tree(
    objects: &[CollectionObject],
    mods: &[CollectionMod],
    mods_path: Option<&str>,
) -> Vec<PreviewTreeNode> {
    let object_lookup: HashMap<&str, &CollectionObject> = objects
        .iter()
        .map(|object| (object.object_id.as_str(), object))
        .collect();
    let mut mods_by_object: HashMap<String, Vec<&CollectionMod>> = HashMap::new();

    for member in mods {
        let object_id = if member.object_id.is_empty() {
            UNCATEGORIZED_ID.to_string()
        } else {
            member.object_id.clone()
        };
        mods_by_object.entry(object_id).or_default().push(member);
    }

    let mut object_ids: Vec<String> = mods_by_object.keys().cloned().collect();
    object_ids.sort_by(|left, right| {
        let left_name = object_lookup
            .get(left.as_str())
            .and_then(|object| object.display_name.as_deref())
            .unwrap_or(left);
        let right_name = object_lookup
            .get(right.as_str())
            .and_then(|object| object.display_name.as_deref())
            .unwrap_or(right);
        canonical_name_key(left_name).cmp(&canonical_name_key(right_name))
    });

    let mut tree = Vec::new();
    for object_id in object_ids {
        let object = object_lookup.get(object_id.as_str()).copied();
        let object_mods = match mods_by_object.get(&object_id) {
            Some(entries) if !entries.is_empty() => entries,
            _ => continue,
        };

        let object_name = object
            .and_then(|entry| entry.display_name.as_deref())
            .unwrap_or(if object_id == UNCATEGORIZED_ID {
                "Uncategorized"
            } else {
                object_id.as_str()
            });
        let object_enabled = object.map(|entry| entry.is_enabled).unwrap_or(true);
        let object_path = object.and_then(|entry| entry.path_key.clone());
        let mut active_children = Vec::new();
        let mut inactive_children = Vec::new();

        for member in object_mods {
            append_mod_branch(
                &mut active_children,
                &mut inactive_children,
                object,
                member,
                mods_path,
                object_enabled,
            );
        }

        prune_empty_branches(&mut active_children);
        prune_empty_branches(&mut inactive_children);
        sort_children(&mut active_children);
        sort_children(&mut inactive_children);
        let active_mod_count = count_preview_mods(&active_children);
        if active_children.is_empty() && inactive_children.is_empty() {
            continue;
        }

        let mut object_node = PreviewTreeNode {
            kind: PreviewTreeNodeKind::Object,
            id: object_id.clone(),
            name: object_name.to_string(),
            path: object_path,
            object_id: Some(object_id.clone()),
            node_type: None,
            is_enabled: object_enabled,
            is_effectively_active: object_enabled,
            inactive_reason: None,
            show_inactive_chip: false,
            status_kind: None,
            collapse_children: false,
            warnings: Vec::new(),
            mod_count: Some(active_mod_count),
            children: active_children,
        };

        if !inactive_children.is_empty() {
            object_node.children.push(PreviewTreeNode {
                kind: PreviewTreeNodeKind::Folder,
                id: format!("inactive-section::{object_id}"),
                name: "Inactive Containers".to_string(),
                path: None,
                object_id: Some(object_id.clone()),
                node_type: Some(INACTIVE_SECTION_NODE_TYPE.to_string()),
                is_enabled: false,
                is_effectively_active: false,
                inactive_reason: Some(INACTIVE_REASON.to_string()),
                show_inactive_chip: false,
                status_kind: None,
                collapse_children: false,
                warnings: Vec::new(),
                mod_count: Some(0),
                children: inactive_children,
            });
        }

        tree.push(object_node);
    }

    tree
}

pub fn count_preview_mods(nodes: &[PreviewTreeNode]) -> usize {
    nodes.iter().map(count_node_mods).sum()
}

pub fn resolve_preview_terminal_metadata(
    object: Option<&CollectionObject>,
    member: &CollectionMod,
    mods_path: Option<&str>,
) -> PreviewTerminalMetadata {
    let descriptor = build_preview_descriptor(object, member, mods_path);
    PreviewTerminalMetadata {
        preview_path: Some(descriptor.display_path),
        node_type: Some(descriptor.node_type.as_str().to_string()),
        warnings: descriptor.warnings,
    }
}

fn count_node_mods(node: &PreviewTreeNode) -> usize {
    if node.node_type.as_deref() == Some(INACTIVE_SECTION_NODE_TYPE) {
        return 0;
    }

    if node.kind == PreviewTreeNodeKind::Mod {
        return 1;
    }

    if node.kind == PreviewTreeNodeKind::Folder
        && matches!(
            node.node_type.as_deref(),
            Some("VariantContainer") | Some("ModPackRoot")
        )
    {
        return 1;
    }

    count_preview_mods(&node.children)
}

fn append_mod_branch(
    active_children: &mut Vec<PreviewTreeNode>,
    inactive_children: &mut Vec<PreviewTreeNode>,
    object: Option<&CollectionObject>,
    member: &CollectionMod,
    mods_path: Option<&str>,
    object_enabled: bool,
) {
    let descriptor = build_preview_descriptor(object, member, mods_path);
    if descriptor.display_segments.is_empty() {
        return;
    }

    let disabled_ancestor_index = descriptor
        .display_segments
        .iter()
        .take(descriptor.display_segments.len().saturating_sub(1))
        .position(|segment| is_disabled_folder(segment));

    let branch_children = if disabled_ancestor_index.is_some() {
        inactive_children
    } else {
        active_children
    };
    let descriptor_path = PathBuf::from(&descriptor.display_path);
    let ancestor_paths =
        ancestor_paths_for_terminal(&Some(descriptor_path), descriptor.display_segments.len());
    let mut current_children = branch_children;
    let mut effective_active = object_enabled;

    for (index, raw_segment) in descriptor
        .display_segments
        .iter()
        .take(descriptor.display_segments.len().saturating_sub(1))
        .enumerate()
    {
        let folder_path = ancestor_paths.get(index).cloned().flatten();
        let is_enabled = !is_disabled_folder(raw_segment);
        let is_effectively_active = effective_active && is_enabled;
        let status_kind = if disabled_ancestor_index == Some(index) {
            Some(STATUS_INACTIVE_CONTAINER.to_string())
        } else {
            None
        };
        let node = ensure_folder_node(
            current_children,
            raw_segment,
            folder_path,
            member.object_id.clone(),
            NodeType::ContainerFolder,
            is_enabled,
            is_effectively_active,
            status_kind.clone(),
            status_kind.is_some(),
            if status_kind.is_some() {
                Some(INACTIVE_REASON.to_string())
            } else {
                None
            },
            Vec::new(),
            false,
        );
        effective_active = node.is_effectively_active;
        current_children = &mut node.children;
    }

    let leaf_segment = descriptor
        .display_segments
        .last()
        .cloned()
        .unwrap_or_default();
    let terminal_status_kind = if disabled_ancestor_index.is_some() && member.is_enabled {
        Some(STATUS_DISABLED_BY_CONTAINER.to_string())
    } else {
        None
    };

    if descriptor.node_type == NodeType::FlatModRoot {
        let leaf_enabled = member.is_enabled && !is_disabled_folder(&leaf_segment);
        ensure_mod_node(
            current_children,
            member,
            &descriptor,
            leaf_enabled,
            effective_active && leaf_enabled,
            terminal_status_kind,
        );
        return;
    }

    let terminal_enabled = member.is_enabled && !is_disabled_folder(&leaf_segment);
    ensure_folder_node(
        current_children,
        &leaf_segment,
        Some(PathBuf::from(&descriptor.display_path)),
        member.object_id.clone(),
        descriptor.node_type,
        terminal_enabled,
        effective_active && terminal_enabled,
        terminal_status_kind,
        false,
        None,
        descriptor.warnings,
        matches!(
            descriptor.node_type,
            NodeType::VariantContainer | NodeType::ModPackRoot
        ),
    );
}

fn ensure_mod_node(
    children: &mut Vec<PreviewTreeNode>,
    member: &CollectionMod,
    descriptor: &PreviewDescriptor,
    is_enabled: bool,
    is_effectively_active: bool,
    status_kind: Option<String>,
) {
    let node_path = descriptor.display_path.clone();
    let node_id = member
        .preview_path
        .clone()
        .or_else(|| member.mod_id.clone())
        .or_else(|| member.mod_path_key.clone())
        .unwrap_or_else(|| node_path.clone());
    let leaf_name = member.display_name.clone().unwrap_or_else(|| {
        descriptor
            .display_segments
            .last()
            .cloned()
            .unwrap_or_default()
    });

    if let Some(index) = children
        .iter()
        .position(|child| child.path.as_deref() == Some(node_path.as_str()))
    {
        let node = &mut children[index];
        node.is_enabled = node.is_enabled || is_enabled;
        node.is_effectively_active = node.is_effectively_active || is_effectively_active;
        if node.status_kind.is_none() {
            node.status_kind = status_kind;
        }
        merge_warnings(&mut node.warnings, &descriptor.warnings);
        return;
    }

    children.push(PreviewTreeNode {
        kind: PreviewTreeNodeKind::Mod,
        id: node_id,
        name: leaf_name,
        path: Some(node_path),
        object_id: Some(member.object_id.clone()),
        node_type: Some(descriptor.node_type.as_str().to_string()),
        is_enabled,
        is_effectively_active,
        inactive_reason: None,
        show_inactive_chip: false,
        status_kind,
        collapse_children: false,
        warnings: descriptor.warnings.clone(),
        mod_count: None,
        children: Vec::new(),
    });
}

#[allow(clippy::too_many_arguments)] // Tree construction keeps node identity, type, status, and path explicit.
fn ensure_folder_node<'a>(
    children: &'a mut Vec<PreviewTreeNode>,
    raw_segment: &str,
    path: Option<PathBuf>,
    object_id: String,
    folder_type: NodeType,
    is_enabled: bool,
    is_effectively_active: bool,
    status_kind: Option<String>,
    show_inactive_chip: bool,
    inactive_reason: Option<String>,
    warnings: Vec<String>,
    collapse_children: bool,
) -> &'a mut PreviewTreeNode {
    let node_name = normalize_display_name(raw_segment);
    let node_path = path
        .as_ref()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| raw_segment.to_string());

    if let Some(index) = children
        .iter()
        .position(|child| child.path.as_deref() == Some(node_path.as_str()))
    {
        let node = &mut children[index];
        node.is_enabled = node.is_enabled || is_enabled;
        node.is_effectively_active = node.is_effectively_active || is_effectively_active;
        node.show_inactive_chip = node.show_inactive_chip || show_inactive_chip;
        if node.inactive_reason.is_none() {
            node.inactive_reason = inactive_reason;
        }
        if node.status_kind.is_none() {
            node.status_kind = status_kind;
        }
        node.collapse_children = node.collapse_children || collapse_children;
        merge_warnings(&mut node.warnings, &warnings);
        return node;
    }

    children.push(PreviewTreeNode {
        kind: PreviewTreeNodeKind::Folder,
        id: format!("folder::{node_path}"),
        name: node_name,
        path: Some(node_path),
        object_id: Some(object_id),
        node_type: Some(folder_type.as_str().to_string()),
        is_enabled,
        is_effectively_active,
        inactive_reason,
        show_inactive_chip,
        status_kind,
        collapse_children,
        warnings,
        mod_count: None,
        children: Vec::new(),
    });
    let last_index = children.len() - 1;
    &mut children[last_index]
}

fn build_preview_descriptor(
    object: Option<&CollectionObject>,
    member: &CollectionMod,
    mods_path: Option<&str>,
) -> PreviewDescriptor {
    if let Some(descriptor) = descriptor_from_stored_metadata(object, member) {
        return descriptor;
    }

    let fallback_name = member.display_name.as_deref().unwrap_or("mod");
    let relative_segments = relative_segments_for_path(object, &member.mod_path, fallback_name);
    if relative_segments.is_empty() {
        return PreviewDescriptor {
            display_path: member.mod_path.clone(),
            node_type: NodeType::FlatModRoot,
            warnings: Vec::new(),
            display_segments: vec![fallback_name.to_string()],
        };
    }

    let terminal_path = resolve_collection_path(&member.mod_path, mods_path);
    if terminal_path.as_ref().is_some_and(|path| !path.exists()) {
        return PreviewDescriptor {
            display_path: member.mod_path.clone(),
            node_type: NodeType::FlatModRoot,
            warnings: Vec::new(),
            display_segments: relative_segments,
        };
    }

    let candidate_paths = cumulative_candidate_paths(&terminal_path, relative_segments.len());
    for candidate_path in candidate_paths {
        let Some((node_type, warnings)) = classify_path(candidate_path.as_deref()) else {
            continue;
        };
        if !matches!(
            node_type,
            NodeType::VariantContainer | NodeType::ModPackRoot | NodeType::FlatModRoot
        ) {
            continue;
        }

        let display_path = candidate_path
            .as_ref()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| member.mod_path.clone());
        let display_segments = relative_segments_for_path(object, &display_path, fallback_name);

        return PreviewDescriptor {
            display_path,
            node_type,
            warnings,
            display_segments,
        };
    }

    let (terminal_type, warnings) =
        classify_path(terminal_path.as_deref()).unwrap_or((NodeType::FlatModRoot, Vec::new()));

    PreviewDescriptor {
        display_path: terminal_path
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| member.mod_path.clone()),
        node_type: terminal_type,
        warnings,
        display_segments: relative_segments,
    }
}

fn descriptor_from_stored_metadata(
    object: Option<&CollectionObject>,
    member: &CollectionMod,
) -> Option<PreviewDescriptor> {
    let stored_path = member.preview_path.clone()?;
    let stored_type = node_type_from_str(member.node_type.as_deref()?)?;
    let fallback_name = member.display_name.as_deref().unwrap_or("mod");
    let display_segments = relative_segments_for_path(object, &stored_path, fallback_name);
    if display_segments.is_empty() {
        return None;
    }

    Some(PreviewDescriptor {
        display_path: stored_path,
        node_type: stored_type,
        warnings: member.warnings.clone(),
        display_segments,
    })
}

fn classify_path(path: Option<&Path>) -> Option<(NodeType, Vec<String>)> {
    let target = path?;
    let (node_type, _reasons, warnings) = classify_folder(target);
    Some((node_type, warnings))
}

fn node_type_from_str(value: &str) -> Option<NodeType> {
    match value {
        "ContainerFolder" => Some(NodeType::ContainerFolder),
        "ModPackRoot" => Some(NodeType::ModPackRoot),
        "VariantContainer" => Some(NodeType::VariantContainer),
        "InternalAssets" => Some(NodeType::InternalAssets),
        "FlatModRoot" => Some(NodeType::FlatModRoot),
        _ => None,
    }
}

fn cumulative_candidate_paths(
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
        let parent_buf = parent.to_path_buf();
        reversed.push(Some(parent_buf.clone()));
        current = parent_buf;
    }
    reversed.reverse();
    reversed
}

fn ancestor_paths_for_terminal(
    path: &Option<PathBuf>,
    segment_count: usize,
) -> Vec<Option<PathBuf>> {
    if segment_count <= 1 {
        return Vec::new();
    }

    let Some(mut current) = path.clone() else {
        return vec![None; segment_count - 1];
    };
    let mut reversed = Vec::with_capacity(segment_count - 1);
    for _ in 0..(segment_count - 1) {
        let Some(parent) = current.parent() else {
            reversed.push(None);
            continue;
        };
        let parent_buf = parent.to_path_buf();
        reversed.push(Some(parent_buf.clone()));
        current = parent_buf;
    }
    reversed.reverse();
    reversed
}

fn relative_segments_for_path(
    object: Option<&CollectionObject>,
    path: &str,
    fallback: &str,
) -> Vec<String> {
    let path_segments = split_segments(path);
    let mut anchors = Vec::new();
    if let Some(object) = object {
        if let Some(path_key) = object.path_key.as_deref() {
            anchors.push(path_key.to_string());
        }
        if let Some(display_name) = object.display_name.as_deref() {
            anchors.push(display_name.to_string());
        }
    }

    for anchor in anchors {
        let anchor_segments = split_segments(&anchor);
        if anchor_segments.is_empty() || anchor_segments.len() > path_segments.len() {
            continue;
        }
        if let Some(start_index) = find_anchor_start(&path_segments, &anchor_segments) {
            let relative = path_segments[(start_index + anchor_segments.len())..].to_vec();
            if !relative.is_empty() {
                return relative;
            }
        }
    }

    vec![path_leaf(path, fallback)]
}

fn split_segments(path: &str) -> Vec<String> {
    path.replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .collect()
}

fn find_anchor_start(path_segments: &[String], anchor_segments: &[String]) -> Option<usize> {
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

fn path_leaf(path: &str, fallback: &str) -> String {
    split_segments(path)
        .last()
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

fn merge_warnings(target: &mut Vec<String>, source: &[String]) {
    for warning in source {
        if target.iter().any(|existing| existing == warning) {
            continue;
        }
        target.push(warning.clone());
    }
}

fn sort_children(children: &mut Vec<PreviewTreeNode>) {
    children.sort_by(|left, right| {
        if left.node_type.as_deref() == Some(INACTIVE_SECTION_NODE_TYPE) {
            return std::cmp::Ordering::Greater;
        }
        if right.node_type.as_deref() == Some(INACTIVE_SECTION_NODE_TYPE) {
            return std::cmp::Ordering::Less;
        }

        match (left.kind, right.kind) {
            (PreviewTreeNodeKind::Folder, PreviewTreeNodeKind::Mod) => std::cmp::Ordering::Less,
            (PreviewTreeNodeKind::Mod, PreviewTreeNodeKind::Folder) => std::cmp::Ordering::Greater,
            _ => canonical_name_key(&left.name).cmp(&canonical_name_key(&right.name)),
        }
    });

    for child in children {
        sort_children(&mut child.children);
    }
}

fn prune_empty_branches(children: &mut Vec<PreviewTreeNode>) {
    let mut index = 0;
    while index < children.len() {
        prune_empty_branches(&mut children[index].children);

        let keep_node = match children[index].kind {
            PreviewTreeNodeKind::Mod => true,
            PreviewTreeNodeKind::Object => {
                !children[index].children.is_empty() || children[index].mod_count.unwrap_or(0) > 0
            }
            PreviewTreeNodeKind::Folder => {
                if children[index].node_type.as_deref() == Some(INACTIVE_SECTION_NODE_TYPE) {
                    !children[index].children.is_empty()
                } else if matches!(
                    children[index].node_type.as_deref(),
                    Some("VariantContainer") | Some("ModPackRoot")
                ) {
                    true
                } else {
                    !children[index].children.is_empty()
                }
            }
        };

        if keep_node {
            index += 1;
            continue;
        }

        children.remove(index);
    }
}

#[cfg(test)]
#[path = "tests/collection_preview_tree_tests.rs"]
mod tests;
