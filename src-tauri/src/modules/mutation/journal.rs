#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::shared::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub id: String,
    pub plan: String,
    pub status: OperationStatus,
}

pub struct OperationJournal {
    // Basic in-memory mock for now
    entries: Arc<RwLock<Vec<JournalEntry>>>,
}

impl OperationJournal {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn write(&self, plan: String, status: OperationStatus) -> Result<String, AppError> {
        // use uuid for id in a real app, but for now just mock it
        let id = format!("{}", self.entries.read().await.len());
        let mut entries = self.entries.write().await;
        entries.push(JournalEntry {
            id: id.clone(),
            plan,
            status,
        });
        Ok(id)
    }

    pub async fn update_status(&self, id: &str, status: OperationStatus) -> Result<(), AppError> {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.status = status;
        }
        Ok(())
    }
}

impl Default for OperationJournal {
    fn default() -> Self {
        Self::new()
    }
}
