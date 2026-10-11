use crate::shared::errors::AppError;
use std::future::Future;
use std::io;
use std::path::Path;
use std::time::Duration;

pub(crate) const LOCK_RETRY_DELAYS: [Duration; 2] =
    [Duration::from_millis(250), Duration::from_secs(2)];

#[derive(Debug, Default)]
pub(crate) struct RetryBudget {
    used: usize,
}

impl RetryBudget {
    pub(crate) fn next_delay(&mut self, error: &io::Error) -> Option<Duration> {
        if !is_file_lock(error) {
            return None;
        }
        let delay = LOCK_RETRY_DELAYS.get(self.used).copied()?;
        self.used += 1;
        Some(delay)
    }

    /// The closure must validate frozen ownership on every attempt and execute
    /// only one uncommitted step. Share this budget across an entire batch.
    pub(crate) async fn run<T, F, Fut>(&mut self, mut attempt: F) -> io::Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = io::Result<T>>,
    {
        loop {
            match attempt().await {
                Ok(value) => return Ok(value),
                Err(error) => match self.next_delay(&error) {
                    Some(delay) => tokio::time::sleep(delay).await,
                    None => return Err(error),
                },
            }
        }
    }
}

pub(crate) fn is_file_lock(error: &io::Error) -> bool {
    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(32 | 33))
    }
    #[cfg(not(windows))]
    {
        let _ = error;
        false
    }
}

pub(crate) fn map_file_error(path: &Path, action: &str, error: io::Error) -> AppError {
    if is_file_lock(&error) {
        return AppError::PathBusy {
            path: path.to_string_lossy().into_owned(),
        };
    }
    AppError::Io(format!(
        "Failed to {action}: {error} (OS error {:?})",
        error.raw_os_error()
    ))
}

pub(crate) fn map_file_error_with_identity(
    path: &Path,
    expected_identity: &str,
    action: &str,
    error: io::Error,
) -> AppError {
    if !is_file_lock(&error) {
        return map_file_error(path, action, error);
    }
    if crate::platform::fs::file_utils::filesystem_identity(path).as_deref()
        != Some(expected_identity)
    {
        return AppError::Validation(format!(
            "Locked target changed at {}; refresh before retrying",
            path.display()
        ));
    }
    let path_string = path.to_string_lossy().into_owned();
    match crate::modules::system::api::register_file_lock_with_identity(path, expected_identity) {
        Some(recovery_id) => AppError::FileLock {
            path: path_string,
            recovery_id,
        },
        None => AppError::PathBusy { path: path_string },
    }
}

pub(crate) fn blocking_error(error: tokio::task::JoinError) -> io::Error {
    io::Error::other(format!("Filesystem worker failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn sharing_and_lock_violations_have_exactly_two_operation_retries() {
        for code in [32, 33] {
            let mut budget = RetryBudget::default();
            let error = io::Error::from_raw_os_error(code);
            assert_eq!(budget.next_delay(&error), Some(Duration::from_millis(250)));
            assert_eq!(budget.next_delay(&error), Some(Duration::from_secs(2)));
            assert_eq!(budget.next_delay(&error), None);
        }
    }

    #[test]
    fn permission_and_collision_do_not_consume_retry_budget() {
        let mut budget = RetryBudget::default();
        for code in [5, 2, 80, 183] {
            assert_eq!(budget.next_delay(&io::Error::from_raw_os_error(code)), None);
        }
        assert_eq!(budget.used, 0);
    }

    #[tokio::test]
    async fn successful_initial_attempt_has_no_retry() {
        let mut budget = RetryBudget::default();
        let mut calls = 0;
        budget
            .run(|| {
                calls += 1;
                std::future::ready(Ok(()))
            })
            .await
            .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(budget.used, 0);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn persistent_lock_attempts_three_times_with_the_requested_delays() {
        let mut budget = RetryBudget::default();
        let mut calls = 0;
        let started = tokio::time::Instant::now();
        let result = budget
            .run(|| {
                calls += 1;
                std::future::ready(Err::<(), _>(io::Error::from_raw_os_error(32)))
            })
            .await;
        assert_eq!(result.unwrap_err().raw_os_error(), Some(32));
        assert_eq!(calls, 3);
        assert!(started.elapsed() >= Duration::from_millis(2250));
        let mut second_calls = 0;
        let _ = budget
            .run(|| {
                second_calls += 1;
                std::future::ready(Err::<(), _>(io::Error::from_raw_os_error(33)))
            })
            .await;
        assert_eq!(
            second_calls, 1,
            "batch must not restart the delay budget per item"
        );
    }

    #[cfg(windows)]
    #[test]
    fn proven_locks_keep_diagnostics_and_reject_a_replacement_identity() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("mod.ini");
        std::fs::write(&path, "[Test]").unwrap();
        let expected = crate::platform::fs::file_utils::filesystem_identity(&path).unwrap();
        for code in [32, 33] {
            assert!(matches!(
                map_file_error_with_identity(
                    &path,
                    &expected,
                    "save",
                    io::Error::from_raw_os_error(code)
                ),
                AppError::FileLock { .. } | AppError::PathBusy { .. }
            ));
        }
        assert!(matches!(
            map_file_error(&path, "save", io::Error::from_raw_os_error(5)),
            AppError::Io(_)
        ));
        assert!(matches!(
            map_file_error(
                &temp.path().join("missing"),
                "save",
                io::Error::from_raw_os_error(32)
            ),
            AppError::PathBusy { .. }
        ));
        std::fs::rename(&path, temp.path().join("original.ini")).unwrap();
        std::fs::write(&path, "replacement").unwrap();
        assert!(matches!(
            map_file_error_with_identity(
                &path,
                &expected,
                "save",
                io::Error::from_raw_os_error(32)
            ),
            AppError::Validation(_)
        ));
    }
}
