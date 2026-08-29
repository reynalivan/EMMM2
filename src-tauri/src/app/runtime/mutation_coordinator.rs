//! Coordination boundary for recoverable mutations.
//!
//! This starter scaffold keeps this module compile-safe while the first storage
//! optimizer migration is completed. Destructive operations should flow through
//! this coordinator before touching filesystem/SQLite layers.

#[derive(Debug, Default)]
pub struct MutationCoordinator {
    enabled: bool,
}

impl MutationCoordinator {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
