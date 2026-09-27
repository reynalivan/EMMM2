//! Shader/buffer overlap inspection and activation ancestor discovery.
//!
//! Two independent concerns, kept in separate files:
//! - `hash_scan` — filesystem: parse `.ini` for `[TextureOverride*]` hashes and
//!   report mods that share one.
//! - `duplicates` — filesystem: locate disabled ancestors during activation.
//!
//! # Covers: US-2.Z, TC-2.4-01

pub mod detect;
pub mod duplicates;
pub mod hash_scan;

pub(crate) use duplicates::*;
pub use hash_scan::*;
