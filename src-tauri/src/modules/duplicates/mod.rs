pub mod adapters;
pub mod application;
pub mod domain;
pub mod facade;

// DupScanState is managed by Tauri via .manage() in lib.rs
pub(crate) use adapters::tauri::tauri::DupScanState;

