pub mod browser_repo;
pub mod collection_repo;
pub mod conflict_repo;
pub mod corridor_repo;
pub mod dashboard_repo;
pub mod dedup_repo;
pub mod game_repo;
pub mod mod_repo;
pub mod object_repo;
pub mod pin_repo;
pub mod runtime_projection_repo;
pub mod settings_repo;
pub mod stable_ids;
pub mod task_repo;
pub mod unicode_keys;

#[cfg(test)]
#[path = "tests/folder_path_normalization_test.rs"]
mod folder_path_normalization_test;
