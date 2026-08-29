pub mod browser;
pub mod collection;
pub mod conflict;
pub mod dashboard;
pub mod dedup;
pub mod game;
pub mod import_batch;
pub mod mods;
pub mod object;
pub mod runtime_projection;
pub mod settings;
pub mod task;
pub mod utils;

#[cfg(test)]
#[path = "tests/folder_path_normalization_test.rs"]
mod folder_path_normalization_test;
