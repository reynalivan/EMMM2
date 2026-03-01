//! KeyViewer pipeline — hash harvesting, matching, and overlay file generation.
//!
//! This module powers the in-game KeyViewer overlay for 3DMigoto-based games.
//! It harvests hashes from active mods, matches them to known objects from the
//! MasterDb, selects sentinel hashes for runtime detection, and generates the
//! `KeyViewer.ini` file consumed by 3DMigoto.

pub mod generator;
pub mod harvester;
pub mod matcher;
pub mod resource_pack;

#[cfg(test)]
mod tests;
