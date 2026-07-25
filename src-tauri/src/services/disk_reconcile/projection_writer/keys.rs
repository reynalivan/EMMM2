//! Path-key derivation shared by every projection write pass.

use std::path::Path;

pub(super) fn root_key(root: &str) -> String {
    crate::common::path_key::canonical_name_key(root)
}

pub(super) fn root_key_for_folder_path(folder_path: &str) -> Option<String> {
    let first = Path::new(folder_path).components().next()?;
    Some(root_key(&first.as_os_str().to_string_lossy()))
}

pub(super) fn runtime_logical_path_key(folder_path: &str) -> String {
    crate::common::path_key::folder_path_key(folder_path, None)
}

pub(super) fn is_runtime_prefix_transition(old_path: &str, new_path: &str) -> bool {
    crate::services::collection_service::classify_collection_path_transition(old_path, new_path)
        == crate::services::collection_service::CollectionPathTransitionKind::RuntimeTogglePrefix
}
