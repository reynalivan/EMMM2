//! Placeholder task registry for startup and background orchestrators.

#[derive(Debug, Default)]
pub struct TaskRegistry {
    #[allow(dead_code)]
    _private: (),
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self { _private: () }
    }
}
