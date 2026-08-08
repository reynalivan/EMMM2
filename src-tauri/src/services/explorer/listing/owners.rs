use std::path::Path;

use crate::common::path_key::folder_path_key;
use crate::domain::errors::AppError;
use crate::domain::objects::ObjectRuntimeDescriptor;
use crate::repo::object_repo::get_runtime_descriptors;

use super::grid::list_mod_folders_inner;

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

pub async fn list_mod_folders_for_game(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: String,
    sub_path: Option<String>,
) -> Result<crate::services::explorer::types::FolderGridResponse, AppError> {
    let owners = get_runtime_descriptors(pool, game_id).await?;
    let mut response = list_mod_folders_inner(mods_path.clone(), sub_path.clone()).await?;
    enrich_owner_metadata(&mut response, &owners, &mods_path, sub_path.as_deref());
    Ok(response)
}
