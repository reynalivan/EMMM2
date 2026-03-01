// All listing logic now lives in services::explorer::listing.
// This file is a delegation shim that re-exports the service functions
pub(crate) use crate::services::explorer::listing::list_mod_folders_inner;

#[cfg(test)]
#[path = "tests/listing_tests.rs"]
mod tests;
