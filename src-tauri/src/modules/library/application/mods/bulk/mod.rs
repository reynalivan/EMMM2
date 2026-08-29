//! Bulk mod operations, split by concern. Public API is unchanged: every item
//! the rest of the crate used to import from `services::mods::bulk` is
//! re-exported here.

mod attributes;
mod delete;
mod toggle;
mod types;

pub use attributes::*;
pub use delete::*;
pub use toggle::*;
pub use types::*;
