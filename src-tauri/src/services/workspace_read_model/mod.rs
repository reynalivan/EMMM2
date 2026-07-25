//! Read-only mappers from DB rows / domain structs to workspace UI view
//! models (explorer listing, object rows, preview trees, selection).
//!
//! Never writes the DB or the filesystem. Runtime counters it reads come from
//! `object_runtime_projection`, maintained by `repo::runtime_projection_repo`.

pub mod common;
pub mod explorer_mapper;
pub mod object_mapper;
pub mod preview_builder;
pub mod selection;
