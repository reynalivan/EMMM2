//! Full-scoring pipeline, split by concern. Public API is unchanged.

mod forced;
mod full_match;
mod scoring_stages;

pub use forced::*;
pub use full_match::*;

#[cfg(test)]
#[path = "../../tests/pipeline/full_pipeline_tests.rs"]
mod full_pipeline_tests;
