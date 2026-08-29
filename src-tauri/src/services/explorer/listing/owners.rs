use std::{collections::HashMap, path::Path};

use crate::common::path_key::folder_path_key;
use crate::domain::errors::AppError;
use crate::domain::objects::ObjectRuntimeDescriptor;
use crate::repo::mod_repo::get_safety_by_folder_path_key;
use crate::repo::object_repo::get_runtime_descriptors;

use super::grid::{list_mod_folders_inner, list_mod_folders_inner_shallow};

/// `prefix` covers `full` when it matches whole path components, so `Alice`
/// owns `Alice/Blue` but not `Alice Extra`.
fn key_covers(prefix: &str, full: &str) -> bool {
    full.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// Owners pre-resolved into the canonical key space once per request. Matching
/// against raw paths instead would re-derive both sides for every
/// child × owner pair.
struct OwnerIndex<'a> {
    entries: Vec<(&'a ObjectRuntimeDescriptor, String)>,
}

impl<'a> OwnerIndex<'a> {
    fn build(owners: &'a [ObjectRuntimeDescriptor], mods_path: &str) -> Self {
        Self {
            entries: owners
                .iter()
                .map(|owner| (owner, folder_path_key(&owner.folder_path, Some(mods_path))))
                .filter(|(_, key)| !key.is_empty())
                .collect(),
        }
    }

    /// The longest covering owner wins, so a nested object beats its ancestor.
    fn resolve(&self, folder_key: &str) -> Option<&'a ObjectRuntimeDescriptor> {
        let mut best_match = None;
        let mut best_key_length = 0usize;

        for (owner, owner_key) in &self.entries {
            if owner_key.len() <= best_key_length || !key_covers(owner_key, folder_key) {
                continue;
            }

            best_match = Some(*owner);
            best_key_length = owner_key.len();
        }

        best_match
    }
}

fn enrich_owner_metadata(
    response: &mut crate::services::explorer::types::FolderGridResponse,
    owners: &[ObjectRuntimeDescriptor],
    mods_path: &str,
    sub_path: Option<&str>,
) {
    let index = OwnerIndex::build(owners, mods_path);

    for folder in &mut response.children {
        let folder_key = folder_path_key(&folder.path, Some(mods_path));
        let Some(owner) = index.resolve(&folder_key) else {
            continue;
        };
        folder.owner_object_id = Some(owner.id.clone());
        folder.owner_object_folder_path = Some(owner.folder_path.clone());
    }

    let Some(relative_sub_path) = sub_path.filter(|value| !value.is_empty()) else {
        return;
    };

    let self_path = Path::new(mods_path).join(relative_sub_path);
    let self_key = folder_path_key(&self_path.to_string_lossy(), Some(mods_path));
    let Some(owner) = index.resolve(&self_key) else {
        return;
    };
    response.self_owner_object_id = Some(owner.id.clone());
    response.self_owner_object_folder_path = Some(owner.folder_path.clone());
}

fn aggregate_safety(
    known_safety: &HashMap<String, crate::repo::mod_repo::SafetyClassification>,
) -> HashMap<&str, (bool, bool)> {
    let mut aggregate_safety = HashMap::<&str, (bool, bool)>::new();
    for (key, classification) in known_safety {
        if !classification.is_classified {
            continue;
        }

        let mut current_key = key.as_str();
        loop {
            let aggregate = aggregate_safety.entry(current_key).or_default();
            if classification.is_safe {
                aggregate.0 = true;
            } else {
                aggregate.1 = true;
            }
            let Some((parent_key, _)) = current_key.rsplit_once('/') else {
                break;
            };
            current_key = parent_key;
        }
    }
    aggregate_safety
}

fn enrich_safety(
    response: &mut crate::services::explorer::types::FolderGridResponse,
    known_safety: &HashMap<String, crate::repo::mod_repo::SafetyClassification>,
    mods_path: &str,
) {
    let aggregate_safety = aggregate_safety(known_safety);

    for folder in &mut response.children {
        let key = folder_path_key(&folder.path, Some(mods_path));
        if let Some(classification) = known_safety.get(&key) {
            folder.is_safe = classification.is_safe;
            folder.is_safety_classified = classification.is_classified;
        }

        if let Some((contains_safe, contains_unsafe)) = aggregate_safety.get(key.as_str()) {
            folder.contains_safe_mods = *contains_safe;
            folder.contains_unsafe_mods = *contains_unsafe;
        }
    }
}

pub async fn list_mod_folders_for_game(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::services::explorer::types::FolderGridResponse, AppError> {
    let owners = get_runtime_descriptors(pool, game_id).await?;
    let known_safety = get_safety_by_folder_path_key(pool, game_id).await?;
    let mut response = list_mod_folders_inner(mods_path.clone(), sub_path.clone()).await?;
    enrich_safety(&mut response, &known_safety, &mods_path);
    enrich_owner_metadata(&mut response, &owners, &mods_path, sub_path.as_deref());
    Ok(response)
}

pub async fn list_mod_folders_for_game_shallow(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::services::explorer::types::FolderGridResponse, AppError> {
    let owners = get_runtime_descriptors(pool, game_id).await?;
    let known_safety = get_safety_by_folder_path_key(pool, game_id).await?;
    let mut response = list_mod_folders_inner_shallow(mods_path.clone(), sub_path.clone()).await?;
    enrich_safety(&mut response, &known_safety, &mods_path);
    enrich_owner_metadata(&mut response, &owners, &mods_path, sub_path.as_deref());
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::aggregate_safety;
    use crate::repo::mod_repo::SafetyClassification;
    use std::collections::HashMap;

    #[test]
    fn aggregates_mixed_descendant_safety_for_parent_navigation() {
        let known = HashMap::from([
            (
                "alice/variants/safe".to_string(),
                SafetyClassification {
                    is_safe: true,
                    is_classified: true,
                },
            ),
            (
                "alice/variants/unsafe".to_string(),
                SafetyClassification {
                    is_safe: false,
                    is_classified: true,
                },
            ),
            (
                "unknown".to_string(),
                SafetyClassification {
                    is_safe: true,
                    is_classified: false,
                },
            ),
        ]);

        let aggregate = aggregate_safety(&known);
        assert_eq!(aggregate.get("alice"), Some(&(true, true)));
        assert_eq!(aggregate.get("alice/variants"), Some(&(true, true)));
        assert_eq!(aggregate.get("unknown"), None);
    }
}
