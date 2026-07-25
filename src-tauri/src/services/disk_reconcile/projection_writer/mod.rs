//! Sole DB writer during disk reconcile: applies a `DiskProjection` (scanned
//! disk snapshot) to the `objects`/`mods` tables so the DB matches disk.
//!
//! Never mutates the filesystem, and only runs inside the disk_reconcile
//! orchestrator. `object_runtime_projection` refresh happens afterwards via
//! `repo::runtime_projection_repo` (see `disk_reconcile::reconcile`).

mod index;
mod keys;
mod mods;
mod objects;
mod prune;
mod state;
mod write;

pub(crate) use write::*;

#[cfg(test)]
mod tests;
