//! Smart Import pipeline for browser downloads, split by concern. Public API is
//! unchanged: every item the rest of the crate used to import from
//! `services::browser::import_service` is re-exported here.

mod jobs;
mod matching;
mod pipeline;
mod placement;
mod queue;

pub use jobs::*;
pub use queue::*;

#[cfg(test)]
mod tests;
