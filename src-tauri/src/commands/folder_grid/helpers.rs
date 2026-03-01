// All explorer helper functions now live in services::explorer::helpers.
// Re-export so command-internal usage (mod.rs) still compiles.

pub use crate::services::explorer::helpers::analyze_mod_metadata;
pub use crate::services::explorer::helpers::apply_safe_mode_filter;
pub use crate::services::explorer::helpers::normalize_keywords;

#[cfg(test)]
#[path = "tests/helpers_tests.rs"]
mod tests;
