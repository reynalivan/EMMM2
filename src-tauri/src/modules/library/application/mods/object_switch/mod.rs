//! Object-root switching, split by concern. Public API is unchanged: every item
//! the rest of the crate used to import from `services::mods::object_switch` is
//! re-exported here.

mod resolve;
mod toggle;

pub(crate) use resolve::resolve_object_root_paths;
pub use toggle::*;

#[cfg(test)]
#[path = "../tests/object_switch_tests.rs"]
mod tests;
