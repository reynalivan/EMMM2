use std::{collections::HashMap, path::Path};

use crate::modules::catalog::adapters::sqlite::object::{
    get_runtime_descriptors, get_runtime_descriptors_for_folder_path_keys,
};
use crate::modules::catalog::domain::objects::ObjectRuntimeDescriptor;
use crate::modules::library::adapters::sqlite::mods::{
    get_safety_by_folder_path_key, get_safety_for_folder_subtree,
};
use crate::shared::errors::AppError;
use crate::shared::path_key::folder_path_key;

use super::grid::{list_mod_folders_inner, list_mod_folders_inner_shallow};

/// DB metadata shared by root and active-folder listings in one workspace read.
/// Loading it once avoids repeating full owner and safety-table reads whenever
/// a selected folder requires both listings.
pub struct ListingEnrichment {
    owner_index: OwnerIndex,
    known_safety:
        HashMap<String, crate::modules::library::adapters::sqlite::mods::SafetyClassification>,
    safety_aggregate: HashMap<String, (bool, bool)>,
}

pub async fn load_listing_enrichment(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &str,
) -> Result<ListingEnrichment, AppError> {
    let owners = get_runtime_descriptors(pool, game_id).await?;
    let known_safety = get_safety_by_folder_path_key(pool, game_id).await?;
    Ok(ListingEnrichment {
        owner_index: OwnerIndex::build(owners, mods_path),
        safety_aggregate: aggregate_safety(&known_safety),
        known_safety,
    })
}

/// Owners pre-resolved into the canonical key space once per request. Matching
/// starts at the deepest folder key, then walks its ancestors. This preserves
/// nearest-owner precedence without scanning every owner for every folder.
struct OwnerIndex {
    entries: HashMap<String, ObjectRuntimeDescriptor>,
}

impl OwnerIndex {
    fn build(owners: Vec<ObjectRuntimeDescriptor>, mods_path: &str) -> Self {
        let mut entries = HashMap::with_capacity(owners.len());
        for owner in owners {
            let key = folder_path_key(&owner.folder_path, Some(mods_path));
            if !key.is_empty() {
                // The previous longest-match scan kept the first descriptor
                // for duplicate keys, so preserve that deterministic tie rule.
                entries.entry(key).or_insert(owner);
            }
        }
        Self { entries }
    }

    fn resolve(&self, folder_key: &str) -> Option<&ObjectRuntimeDescriptor> {
        let mut candidate = folder_key;
        loop {
            if let Some(owner) = self.entries.get(candidate) {
                return Some(owner);
            }
            let (parent, _) = candidate.rsplit_once('/')?;
            candidate = parent;
        }
    }
}

fn enrich_owner_metadata(
    response: &mut crate::modules::workspace::application::explorer::types::FolderGridResponse,
    index: &OwnerIndex,
    mods_path: &str,
    sub_path: Option<&str>,
) {
    enrich_folder_owner_metadata(&mut response.children, index, mods_path);

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

fn enrich_folder_owner_metadata(
    folders: &mut [crate::modules::workspace::application::explorer::types::ModFolder],
    index: &OwnerIndex,
    mods_path: &str,
) {
    for folder in folders {
        let folder_key = folder_path_key(&folder.path, Some(mods_path));
        let Some(owner) = index.resolve(&folder_key) else {
            continue;
        };
        folder.owner_object_id = Some(owner.id.clone());
        folder.owner_object_folder_path = Some(owner.folder_path.clone());
    }
}

fn aggregate_safety(
    known_safety: &HashMap<
        String,
        crate::modules::library::adapters::sqlite::mods::SafetyClassification,
    >,
) -> HashMap<String, (bool, bool)> {
    let mut aggregate_safety = HashMap::<String, (bool, bool)>::new();
    for (key, classification) in known_safety {
        if !classification.is_classified {
            continue;
        }

        let mut current_key = key.as_str();
        loop {
            let aggregate = aggregate_safety.entry(current_key.to_string()).or_default();
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
    response: &mut crate::modules::workspace::application::explorer::types::FolderGridResponse,
    known_safety: &HashMap<
        String,
        crate::modules::library::adapters::sqlite::mods::SafetyClassification,
    >,
    safety_aggregate: &HashMap<String, (bool, bool)>,
    mods_path: &str,
) {
    enrich_folder_safety(
        &mut response.children,
        known_safety,
        safety_aggregate,
        mods_path,
    );
}

fn enrich_folder_safety(
    folders: &mut [crate::modules::workspace::application::explorer::types::ModFolder],
    known_safety: &HashMap<
        String,
        crate::modules::library::adapters::sqlite::mods::SafetyClassification,
    >,
    safety_aggregate: &HashMap<String, (bool, bool)>,
    mods_path: &str,
) {
    for folder in folders {
        let key = folder_path_key(&folder.path, Some(mods_path));
        if let Some(classification) = known_safety.get(&key) {
            folder.is_safe = classification.is_safe;
            folder.is_safety_classified = classification.is_classified;
        }

        if let Some((contains_safe, contains_unsafe)) = safety_aggregate.get(&key) {
            folder.contains_safe_mods = *contains_safe;
            folder.contains_unsafe_mods = *contains_unsafe;
        }
    }
}

fn folder_key_and_ancestors(folder_key: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut current = folder_key;
    loop {
        if current.is_empty() {
            break;
        }
        keys.push(current.to_string());
        let Some((parent, _)) = current.rsplit_once('/') else {
            break;
        };
        current = parent;
    }
    keys
}

/// Adds the same owner and safety fields as a full listing, but scopes DB work
/// to one selected folder and its ancestor/subtree keys. Preview requests must
/// never load the game-wide owner or safety registries.
pub async fn enrich_mod_folder_for_game(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &str,
    folder: &mut crate::modules::workspace::application::explorer::types::ModFolder,
) -> Result<(), AppError> {
    let folder_key = folder_path_key(&folder.path, Some(mods_path));
    if folder_key.is_empty() {
        return Ok(());
    }

    let owner_keys = folder_key_and_ancestors(&folder_key);
    let (owners, safety) = tokio::try_join!(
        get_runtime_descriptors_for_folder_path_keys(pool, game_id, &owner_keys),
        get_safety_for_folder_subtree(pool, game_id, &folder_key),
    )?;

    let owner_index = OwnerIndex::build(owners, mods_path);
    if let Some(owner) = owner_index.resolve(&folder_key) {
        folder.owner_object_id = Some(owner.id.clone());
        folder.owner_object_folder_path = Some(owner.folder_path.clone());
    }

    if let Some(classification) = safety.get(&folder_key) {
        folder.is_safe = classification.is_safe;
        folder.is_safety_classified = classification.is_classified;
    }
    if let Some((contains_safe, contains_unsafe)) = aggregate_safety(&safety).get(&folder_key) {
        folder.contains_safe_mods = *contains_safe;
        folder.contains_unsafe_mods = *contains_unsafe;
    }

    Ok(())
}

pub async fn list_mod_folders_for_game(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::modules::workspace::application::explorer::types::FolderGridResponse, AppError> {
    let enrichment = load_listing_enrichment(pool, game_id, &mods_path).await?;
    list_mod_folders_for_game_with_enrichment(&enrichment, mods_path, sub_path).await
}

pub async fn list_mod_folders_for_game_with_enrichment(
    enrichment: &ListingEnrichment,
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::modules::workspace::application::explorer::types::FolderGridResponse, AppError> {
    let mut response = list_mod_folders_inner(mods_path.clone(), sub_path.clone()).await?;
    enrich_safety(
        &mut response,
        &enrichment.known_safety,
        &enrichment.safety_aggregate,
        &mods_path,
    );
    enrich_owner_metadata(
        &mut response,
        &enrichment.owner_index,
        &mods_path,
        sub_path.as_deref(),
    );
    Ok(response)
}

pub async fn list_mod_folders_for_game_shallow(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::modules::workspace::application::explorer::types::FolderGridResponse, AppError> {
    let enrichment = load_listing_enrichment(pool, game_id, &mods_path).await?;
    list_mod_folders_for_game_shallow_with_enrichment(&enrichment, mods_path, sub_path).await
}

pub async fn list_mod_folders_for_game_shallow_with_enrichment(
    enrichment: &ListingEnrichment,
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::modules::workspace::application::explorer::types::FolderGridResponse, AppError> {
    let mut response = list_mod_folders_inner_shallow(mods_path.clone(), sub_path.clone()).await?;
    enrich_safety(
        &mut response,
        &enrichment.known_safety,
        &enrichment.safety_aggregate,
        &mods_path,
    );
    enrich_owner_metadata(
        &mut response,
        &enrichment.owner_index,
        &mods_path,
        sub_path.as_deref(),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::{aggregate_safety, OwnerIndex};
    use crate::modules::catalog::domain::objects::ObjectRuntimeDescriptor;
    use crate::modules::library::adapters::sqlite::mods::SafetyClassification;
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

    fn owner(id: &str, folder_path: &str) -> ObjectRuntimeDescriptor {
        ObjectRuntimeDescriptor {
            id: id.to_string(),
            name: id.to_string(),
            folder_path: folder_path.to_string(),
            folder_path_key: folder_path.to_string(),
            matched_entry_key: None,
            matched_alias_name: None,
            object_type: "Character".to_string(),
            thumbnail_path: None,
        }
    }

    #[test]
    fn resolves_the_nearest_nested_owner_from_the_normalized_key_index() {
        let temp = tempfile::tempdir().expect("temp directory");
        let mods_path = temp.path().join("Mods");
        let mods_path = mods_path.to_string_lossy().to_string();
        let owners = vec![owner("alice", "Alice"), owner("variant", "Alice/Variant")];
        let index = OwnerIndex::build(owners, &mods_path);
        let folder_key = crate::shared::path_key::folder_path_key(
            &std::path::Path::new(&mods_path)
                .join("Alice")
                .join("Variant")
                .join("Blue")
                .to_string_lossy(),
            Some(&mods_path),
        );

        assert_eq!(
            index
                .resolve(&folder_key)
                .map(|descriptor| descriptor.id.as_str()),
            Some("variant")
        );
    }
}
