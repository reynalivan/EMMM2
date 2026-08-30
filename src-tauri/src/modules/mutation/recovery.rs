pub struct RecoveryRunner {
}

impl RecoveryRunner {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn run_recovery(&self) {
        // Look at OperationJournal and recover incomplete operations
    }
}

impl Default for RecoveryRunner {
    fn default() -> Self {
        Self::new()
    }
}
