//! File generation pipeline — keybind text, KeyViewer.ini, status banner, reload key discovery.
//!
//! Generates the files consumed by 3DMigoto at runtime:
//! - `EMM2/keybinds/active/<sentinel_hash>.txt` — per-object keybind text
//! - `EMM2/keybinds/active/_fallback.txt` — default text when no object matched
//! - `Mods/EMM2_System/KeyViewer.ini` — 3DMigoto runtime overlay
//! - `EMM2/status/runtime_status.txt` — in-game status banner
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
