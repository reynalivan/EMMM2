//! Mod-row persistence, split by concern. Public API is unchanged: every item
//! the rest of the crate used to import from `repo::mod_repo` is re-exported here.

mod batch;
mod corridor;
mod listing;
mod lookup;
mod mutate;
mod paths;
mod sync;
mod types;
mod update;

pub use batch::*;
pub use corridor::*;
pub use listing::*;
pub use lookup::*;
pub use mutate::*;
pub use sync::*;
pub use types::*;
pub use update::*;

#[cfg(test)]
#[path = "../tests/mod_repo_test.rs"]
mod tests;
