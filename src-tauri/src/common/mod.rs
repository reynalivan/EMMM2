/// Leaf utilities shared by every layer (commands, services, repo, pipeline).
/// Nothing in this module may import from services, repo, or commands.
pub mod classifier;
pub mod normalizer;
pub mod path_key;
pub mod safety_constants;
pub mod sync;
