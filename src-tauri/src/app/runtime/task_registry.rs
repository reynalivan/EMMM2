pub struct TaskRegistry {
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}
