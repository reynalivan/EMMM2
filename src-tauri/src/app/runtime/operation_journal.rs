//! Placeholder operation journal API for long-running mutation workflows.
//!
//! The final migration phase will persist `PENDING -> COMPLETED` transitions here
//! and let startup recovery close the gap when mutations are interrupted.

#[derive(Debug, Default)]
pub struct OperationJournal {
    #[allow(dead_code)]
    _private: (),
}

impl OperationJournal {
    pub fn new() -> Self {
        Self { _private: () }
    }
}
