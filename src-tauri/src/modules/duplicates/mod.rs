pub(crate) mod adapters;
pub(crate) mod application;
pub(crate) mod domain;
pub mod facade;

// DupScanState is managed by Tauri via .manage() in lib.rs
pub(crate) use adapters::tauri::tauri::DupScanState;


pub mod api;
