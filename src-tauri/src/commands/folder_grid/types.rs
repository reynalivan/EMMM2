// Re-export all explorer domain types from the service layer.
// Commands must not own these — they belong in services::explorer::types.
pub use crate::services::explorer::types::{
    ConflictGroup, ConflictMember, FolderGridResponse, InfoAnalysis, ModFolder,
};

#[cfg(test)]
#[path = "tests/types_tests.rs"]
mod tests;
