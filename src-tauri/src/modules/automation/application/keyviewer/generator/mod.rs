//! File generation pipeline — keybind text, KeyViewer.ini, status banner, reload key discovery.
//!
//! Generates the files consumed by 3DMigoto at runtime:
//! - `generations/keybinds/active/<sentinel_hash>.txt` — per-object keybind text
//! - `.emmm_data/KeyViewer.ini` — 3DMigoto runtime overlay
//! - `generations/status/runtime_status.txt` — in-game status banner
//!
//! All writes are atomic (`.tmp` → rename).

mod atomic;
mod ini;
mod keybind_text;
mod reload_key;
mod status;

pub use atomic::*;
pub use ini::*;
pub use keybind_text::*;
pub use reload_key::*;
pub use status::*;
