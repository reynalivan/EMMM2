//! Collection business logic, split by concern. Public API is unchanged: every
//! item the rest of the crate used to import from `services::collection_service`
//! is re-exported here.

mod apply;
mod crud;
mod current_state;
mod live_state;
mod path_transition;
mod preview;
mod projection;
mod references;

pub use apply::*;
pub use crud::*;
pub use current_state::*;
pub(crate) use live_state::*;
pub(crate) use path_transition::*;
pub use preview::*;
pub use projection::*;
pub use references::*;

#[cfg(test)]
mod tests;
