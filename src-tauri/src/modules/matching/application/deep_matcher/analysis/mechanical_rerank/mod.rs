//! Deterministic mechanical AI reranker for the deep matcher pipeline.
//!
//! Runs after the standard pipeline + optional GameBanana enrichment when
//! the result is still `NeedsReview`. Uses a points-based system to score
//! each candidate and decide acceptance.
//!
//! **Independent**: Works with or without GB enrichment data. Works with or
//! without the trait-based AI provider. This is a fallback that runs even
//! if both are disabled.

mod config;
mod penalties;
mod points;
mod rerank;

pub use config::*;
pub use rerank::*;
