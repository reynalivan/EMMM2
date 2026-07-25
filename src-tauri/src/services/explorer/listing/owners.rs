use std::path::Path;

use crate::repo::object_repo::{get_runtime_descriptors, ObjectRuntimeDescriptor};

use super::grid::list_mod_folders_inner;

fn resolve_owner_descriptor<'a>(
    owners: &'a [ObjectRuntimeDescriptor],
    folder_path: &str,
    mods_path: &str,
) -> Option<&'a ObjectRuntimeDescriptor> {
    let mut best_match: Option<&ObjectRuntimeDescriptor> = None;
    let mut best_key_length = 0usize;

    for owner in owners {
        if !crate::common::path_key::path_starts_with_key(
            folder_path,
            &owner.folder_path,
            Some(mods_path),
        ) {
            continue;
        }

        let key_length = owner.folder_path_key.len();
        if key_length <= best_key_length {
            continue;
        }

        best_match = Some(owner);
        best_key_length = key_length;
    }

    best_match
}

fn enrich_owner_metadata(
    response: &mut crate::services::explorer::types::FolderGridResponse,
    owners: &[ObjectRuntimeDescriptor],
    mods_path: &str,
    sub_path: Option<&str>,
) {
    for folder in &mut response.children {
        let Some(owner) = resolve_owner_descriptor(owners, &folder.path, mods_path) else {
            continue;
        };
        folder.owner_object_id = Some(owner.id.clone());
        folder.owner_object_folder_path = Some(owner.folder_path.clone());
    }

    let Some(relative_sub_path) = sub_path else {
        return;
    };
    if relative_sub_path.is_empty() {
        return;
    }

    let self_path = Path::new(mods_path).join(relative_sub_path);
    let self_path_str = self_path.to_string_lossy().to_string();
    let Some(owner) = resolve_owner_descriptor(owners, &self_path_str, mods_path) else {
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
) -> Result<crate::services::explorer::types::FolderGridResponse, String> {
    let owners = get_runtime_descriptors(pool, game_id)
        .await
        .map_err(|error| error.to_string())?;
    let mut response = list_mod_folders_inner(mods_path.clone(), sub_path.clone()).await?;
    enrich_owner_metadata(&mut response, &owners, &mods_path, sub_path.as_deref());
    Ok(response)
}
