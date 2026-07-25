/// Leaf utilities shared by every layer (commands, services, repo, pipeline).
/// Nothing in this module may import from services, repo, or commands.
pub mod classifier;
pub mod corridor_constants;
pub mod normalizer;
pub mod path_key;
