//! Placeholder recovery runner for crash-restart mutation repair.

#[derive(Debug, Default)]
pub struct RecoveryRunner {
    #[allow(dead_code)]
    _private: (),
}

impl RecoveryRunner {
    pub fn new() -> Self {
        Self { _private: () }
    }
}
