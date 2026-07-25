//! Collection persistence, split by concern. Public API is unchanged: every item
//! the rest of the crate used to import from `repo::collection_repo` is
//! re-exported here.

mod crud;
mod live;
mod mapping;
mod members;
mod references;
mod state;

pub use crud::*;
pub use live::*;
pub use mapping::*;
pub use members::*;
pub use references::*;
pub use state::*;
