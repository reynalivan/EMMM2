use std::path::{Path, PathBuf};

use crate::common::classifier::{classify_folder, NodeType};
use crate::common::path_key::{canonical_name_key, resolve_collection_path};
use crate::domain::collection::{CollectionMod, CollectionObject};

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
        };
    }

    let terminal_path = resolve_collection_path(&member.mod_path, mods_path);
    if terminal_path.as_ref().is_some_and(|path| !path.exists()) {
        return PreviewDescriptor {
            display_path: member.mod_path.clone(),
            node_type: NodeType::FlatModRoot,
            warnings: Vec::new(),
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

        return PreviewDescriptor {
            display_path,
            node_type,
            warnings,
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
    }
}

fn descriptor_from_stored_metadata(
    object: Option<&CollectionObject>,
    member: &CollectionMod,
) -> Option<PreviewDescriptor> {
    let stored_path = member.preview_path.clone()?;
    let stored_type = node_type_from_str(member.node_type.as_deref()?)?;
    let fallback_name = member.display_name.as_deref().unwrap_or("mod");
    if relative_segments_for_path(object, &stored_path, fallback_name).is_empty() {
        return None;
    }

    Some(PreviewDescriptor {
        display_path: stored_path,
        node_type: stored_type,
        warnings: member.warnings.clone(),
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
