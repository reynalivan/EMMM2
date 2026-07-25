//! Object persistence, split by concern. Public API is unchanged: every item
//! the rest of the crate used to import from `repo::object_repo` is re-exported here.

mod counts;
mod listing;
mod lookup;
mod matching;
mod mutate;
mod sync;
mod types;
mod update;

pub use listing::*;
pub use lookup::*;
pub use matching::*;
pub use mutate::*;
pub use sync::*;
pub use types::*;
pub use update::*;

#[cfg(test)]
#[path = "../object_repo_tests.rs"]
mod tests;
