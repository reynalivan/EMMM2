//! In-memory projection builder for collection snapshots: collection members
//! -> `ProjectedCollectionState` (+ JSON (de)serialization and signatures).
//!
//! Pure compute — never reads or writes the DB or the filesystem. Persistence
//! of the resulting snapshot belongs to `repo::collection_repo`.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::common::normalizer::normalize_display_name;
use crate::common::path_key::{
    canonical_collection_path_key, canonical_name_key, resolve_collection_path,
};
use crate::domain::collection::{
    CollectionMod, CollectionObject, ProjectedActiveRoot, ProjectedCollectionState,
    ProjectedObjectState, ProjectedStateSummary,
};
use crate::services::collection_preview_tree::resolve_preview_terminal_metadata;

mod members;

pub use members::*;

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

    let mut active_roots = root_map.into_values().collect::<Vec<_>>();
    let mut root_count_by_object = HashMap::<String, usize>::new();
    for root in &active_roots {
        *root_count_by_object
            .entry(root.object_id.clone())
            .or_insert(0) += 1;
    }

    for object_state in &mut object_states {
        object_state.active_root_count = root_count_by_object
            .get(&object_state.object_id)
            .copied()
            .unwrap_or(0);
    }

    // Cached keys: canonical_name_key runs a regex and allocates twice, so
    // recomputing it inside the comparator costs O(n log n) regex passes where
    // one pass per element is enough. The root key stays a tuple rather than a
    // joined string so a display name containing the separator cannot reorder it.
    object_states.sort_by_cached_key(|object| canonical_name_key(&object.display_name));
    active_roots.sort_by_cached_key(|root| {
        (
            canonical_name_key(&root.display_name),
            canonical_name_key(&root.object_id),
        )
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

    normalize_display_name(path_name.as_deref().or(fallback).unwrap_or("Unnamed Mod")).into_owned()
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

#[cfg(test)]
#[path = "tests/projected_state_service_tests.rs"]
mod tests;
