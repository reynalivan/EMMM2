//! In-app browser tab lifecycle, split by concern. Public API is unchanged:
//! every item the rest of the crate used to import from
//! `services::browser::browser_service` is re-exported here.

mod adblock;
mod metadata;
mod paths;
mod settings;
mod tabs;
mod webview;

pub use adblock::{get_enabled, set_enabled, BrowserAdblockState};
pub use metadata::*;
pub use paths::*;
pub use settings::*;
pub use tabs::*;
pub use webview::*;
