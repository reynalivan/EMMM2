use std::collections::HashSet;
use std::sync::Mutex;

use crate::shared::errors::AppError;

pub struct TaskRegistry {
    active_ids: Mutex<HashSet<String>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            active_ids: Mutex::new(HashSet::new()),
        }
    }

    pub fn register(&self, id: String) -> Result<bool, AppError> {
        Ok(self.lock_active_ids()?.insert(id))
    }

    pub fn unregister(&self, id: &str) -> Result<bool, AppError> {
        Ok(self.lock_active_ids()?.remove(id))
    }

    pub fn contains(&self, id: &str) -> Result<bool, AppError> {
        Ok(self.lock_active_ids()?.contains(id))
    }

    fn lock_active_ids(&self) -> Result<std::sync::MutexGuard<'_, HashSet<String>>, AppError> {
        self.active_ids
            .lock()
            .map_err(|_| AppError::Internal("Mutation task registry lock poisoned".to_string()))
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}
