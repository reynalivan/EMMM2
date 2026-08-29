//! Shader/buffer conflict detection and duplicate resolution for 3DMigoto mods.
//!
//! Two independent concerns, kept in separate files:
//! - `hash_scan` — filesystem: parse `.ini` for `[TextureOverride*]` hashes and
//!   report mods that share one.
//! - `duplicates` — database: find and resolve competing mods for one object.
//!
//! # Covers: US-2.Z, TC-2.4-01

pub mod detect;
pub mod duplicates;
pub mod hash_scan;

pub use duplicates::*;
pub use hash_scan::*;
