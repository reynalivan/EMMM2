//! Collection business logic, split by concern. Public API is unchanged: every
//! item the rest of the crate used to import from `services::collection`
//! is re-exported here.

mod apply;
mod crud;
mod current_state;
mod live_state;
mod path_transition;
mod preview;
mod projection;
mod references;
mod runtime;
mod safe_target;

pub use apply::*;
pub use crud::*;
pub use current_state::*;
pub(crate) use live_state::*;
pub(crate) use path_transition::*;
pub use preview::*;
pub use projection::*;
pub use references::*;
pub use runtime::*;
pub(crate) use safe_target::*;

#[cfg(test)]
mod tests;

pub mod preview_tree;
