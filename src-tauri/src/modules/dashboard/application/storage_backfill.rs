use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;

const STORAGE_SIZE_BACKFILL_KEY: &str = "storage_size_backfill_version";
const STORAGE_SIZE_BACKFILL_VERSION: &str = "1";

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum StorageSizeBackfillStateKind {
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct StorageSizeBackfillStatus {
    pub state: StorageSizeBackfillStateKind,
    #[specta(type = f64)]
    pub total_games: usize,
    #[specta(type = f64)]
    pub completed_games: usize,
    pub current_game_id: Option<String>,
    pub errors: Vec<String>,
}

impl Default for StorageSizeBackfillStatus {
    fn default() -> Self {
        Self {
            state: StorageSizeBackfillStateKind::Idle,
            total_games: 0,
            completed_games: 0,
            current_game_id: None,
            errors: Vec::new(),
        }
    }
}

/// Process-local job state. Completion is persisted separately in
/// `app_settings`, while pending games stay in memory so Retry only revisits
/// games that did not finish during this app session.
#[derive(Default)]
pub struct StorageSizeBackfillState {
    status: Mutex<StorageSizeBackfillStatus>,
    pending_game_ids: Mutex<Vec<String>>,
}

impl StorageSizeBackfillState {
    pub fn status(&self) -> StorageSizeBackfillStatus {
        crate::shared::sync::lock(&self.status).clone()
    }

    pub fn mark_completed_from_marker(&self, total_games: usize) -> StorageSizeBackfillStatus {
        let status = StorageSizeBackfillStatus {
            state: StorageSizeBackfillStateKind::Completed,
            total_games,
            completed_games: total_games,
            current_game_id: None,
            errors: Vec::new(),
        };
        *crate::shared::sync::lock(&self.status) = status.clone();
        crate::shared::sync::lock(&self.pending_game_ids).clear();
        status
    }

    /// Returns only the games that still require sizing, or `None` if a job
    /// is already in-flight.
    pub fn begin(&self, game_ids: &[String]) -> Option<Vec<String>> {
        let mut status = crate::shared::sync::lock(&self.status);
        if status.state == StorageSizeBackfillStateKind::Running {
            return None;
        }

        let mut pending = crate::shared::sync::lock(&self.pending_game_ids);
        if pending.is_empty() {
            *pending = game_ids.to_vec();
            status.total_games = game_ids.len();
            status.completed_games = 0;
        }
        status.state = StorageSizeBackfillStateKind::Running;
        status.current_game_id = pending.first().cloned();
        status.errors.clear();
        Some(pending.clone())
    }

    pub fn record_game_completed(&self, game_id: &str) {
        let mut status = crate::shared::sync::lock(&self.status);
        let mut pending = crate::shared::sync::lock(&self.pending_game_ids);
        pending.retain(|id| id != game_id);
        status.completed_games = status.total_games.saturating_sub(pending.len());
        status.current_game_id = pending.first().cloned();
    }

    pub fn fail(&self, message: String) {
        let mut status = crate::shared::sync::lock(&self.status);
        status.state = StorageSizeBackfillStateKind::Failed;
        status.current_game_id = None;
        status.errors.push(message);
    }

    pub fn complete(&self) {
        let mut status = crate::shared::sync::lock(&self.status);
        status.state = StorageSizeBackfillStateKind::Completed;
        status.completed_games = status.total_games;
        status.current_game_id = None;
        status.errors.clear();
    }
}

pub async fn is_completed(pool: &sqlx::SqlitePool) -> Result<bool, sqlx::Error> {
    Ok(
        crate::modules::system::adapters::sqlite::settings::get_setting(
            pool,
            STORAGE_SIZE_BACKFILL_KEY,
        )
        .await?
        .as_deref()
            == Some(STORAGE_SIZE_BACKFILL_VERSION),
    )
}

pub async fn mark_completed(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    crate::modules::system::adapters::sqlite::settings::set_setting(
        pool,
        STORAGE_SIZE_BACKFILL_KEY,
        STORAGE_SIZE_BACKFILL_VERSION,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_only_reuses_games_left_pending_after_failure() {
        let state = StorageSizeBackfillState::default();
        let games = vec!["g1".to_string(), "g2".to_string()];
        assert_eq!(state.begin(&games), Some(games.clone()));
        state.record_game_completed("g1");
        state.fail("g2 unavailable".to_string());

        assert_eq!(state.begin(&games), Some(vec!["g2".to_string()]));
        let status = state.status();
        assert_eq!(status.completed_games, 1);
        assert_eq!(status.current_game_id.as_deref(), Some("g2"));
    }

    #[test]
    fn concurrent_start_does_not_duplicate_the_job() {
        let state = StorageSizeBackfillState::default();
        let games = vec!["g1".to_string()];
        assert!(state.begin(&games).is_some());
        assert!(state.begin(&games).is_none());
    }

    #[tokio::test]
    async fn completion_marker_is_not_set_until_backfill_finishes() {
        let context = crate::test_utils::init_test_db().await;
        assert!(!is_completed(&context.pool).await.expect("completion state"));
        mark_completed(&context.pool)
            .await
            .expect("completion marker should save");
        assert!(is_completed(&context.pool).await.expect("completion state"));
    }
}
