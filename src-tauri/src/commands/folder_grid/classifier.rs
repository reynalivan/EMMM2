// All folder classification logic now lives in services::explorer::classifier.
// This file re-exports everything so internal usage within commands:: still works.

pub use crate::services::explorer::classifier::classify_folder;
pub use crate::services::explorer::classifier::NodeType;
