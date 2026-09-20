//! Durable, privacy-preserving telemetry primitives.
//!
//! This module owns local aggregation only. Network transport, user consent,
//! and Tauri commands deliberately live outside this module.

use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

const RETENTION_DAYS: i64 = 7;
const EXPORT_STREAM: &str = "rollups";

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("failed to serialize crash marker")]
    CrashMarkerSerialization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelemetryOperation {
    Onboarding,
    OnboardingPreparation,
    OnboardingClassification,
    OnboardingApply,
    OnboardingRecheck,
    Classification,
    ClassificationReview,
    AutoMatch,
    Watcher,
    Reconcile,
    Toggle,
    BulkAction,
    Import,
    Extract,
    CollectionApply,
    Restore,
    Launch,
    Error,
}

impl TelemetryOperation {
    pub fn from_label(value: &str) -> Self {
        match value {
            "onboarding" => Self::Onboarding,
            "onboarding_preparation" => Self::OnboardingPreparation,
            "onboarding_classification" => Self::OnboardingClassification,
            "onboarding_apply" => Self::OnboardingApply,
            "onboarding_recheck" => Self::OnboardingRecheck,
            "classification" => Self::Classification,
            "classification_review" => Self::ClassificationReview,
            "auto_match" => Self::AutoMatch,
            "watcher" => Self::Watcher,
            "reconcile" => Self::Reconcile,
            "toggle" => Self::Toggle,
            "bulk_action" => Self::BulkAction,
            "import" => Self::Import,
            "extract" => Self::Extract,
            "collection_apply" => Self::CollectionApply,
            "restore" => Self::Restore,
            "launch" => Self::Launch,
            _ => Self::Error,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Onboarding => "onboarding",
            Self::OnboardingPreparation => "onboarding_preparation",
            Self::OnboardingClassification => "onboarding_classification",
            Self::OnboardingApply => "onboarding_apply",
            Self::OnboardingRecheck => "onboarding_recheck",
            Self::Classification => "classification",
            Self::ClassificationReview => "classification_review",
            Self::AutoMatch => "auto_match",
            Self::Watcher => "watcher",
            Self::Reconcile => "reconcile",
            Self::Toggle => "toggle",
            Self::BulkAction => "bulk_action",
            Self::Import => "import",
            Self::Extract => "extract",
            Self::CollectionApply => "collection_apply",
            Self::Restore => "restore",
            Self::Launch => "launch",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelemetryOutcome {
    Started,
    Success,
    Partial,
    Failed,
    AutoAccepted,
    NeedsReview,
    Matched,
    Unmatched,
    Accepted,
    Rejected,
    Cancelled,
    Triggered,
    Overflow,
}

impl TelemetryOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::AutoAccepted => "auto_accepted",
            Self::NeedsReview => "needs_review",
            Self::Matched => "matched",
            Self::Unmatched => "unmatched",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::Triggered => "triggered",
            Self::Overflow => "overflow",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelemetryErrorCode {
    None,
    Unknown,
    Validation,
    Io,
    Database,
    Network,
    Conflict,
    Timeout,
    Cancelled,
    Panic,
    Invariant,
    Permission,
    NotFound,
    Unsupported,
    External,
}

impl TelemetryErrorCode {
    pub fn from_label(value: &str) -> Self {
        match value {
            "validation" => Self::Validation,
            "io" => Self::Io,
            "database" => Self::Database,
            "network" => Self::Network,
            "conflict" => Self::Conflict,
            "timeout" => Self::Timeout,
            "cancelled" => Self::Cancelled,
            "panic" => Self::Panic,
            "invariant" => Self::Invariant,
            "permission" => Self::Permission,
            "not_found" => Self::NotFound,
            "unsupported" => Self::Unsupported,
            "external" => Self::External,
            _ => Self::Unknown,
        }
    }
    /// Maps the typed native error boundary to the small, stable vocabulary
    /// used in telemetry. Error text, paths, and nested payloads never leave
    /// the process.
    pub fn from_app_error(error: &crate::shared::errors::AppError) -> Self {
        match error {
            crate::shared::errors::AppError::RuntimeState(_) => Self::Invariant,
            crate::shared::errors::AppError::Collection(error) => match error {
                crate::shared::errors::CollectionError::NotFound { .. }
                | crate::shared::errors::CollectionError::MissingMods { .. } => Self::NotFound,
                crate::shared::errors::CollectionError::DuplicateName { .. } => Self::Conflict,
                crate::shared::errors::CollectionError::Validation(_) => Self::Validation,
                crate::shared::errors::CollectionError::Db(_) => Self::Database,
                crate::shared::errors::CollectionError::RuntimeState(_) => Self::Invariant,
                crate::shared::errors::CollectionError::Io(_) => Self::Io,
                crate::shared::errors::CollectionError::FileInUse { .. }
                | crate::shared::errors::CollectionError::PathBusy { .. } => Self::External,
            },
            crate::shared::errors::AppError::Metadata(error) => match error {
                crate::shared::errors::MetadataError::Security(_) => Self::Permission,
                crate::shared::errors::MetadataError::NotFound(_) => Self::NotFound,
                crate::shared::errors::MetadataError::Io(_) => Self::Io,
                crate::shared::errors::MetadataError::Db(_) => Self::Database,
                crate::shared::errors::MetadataError::Validation(_) => Self::Validation,
            },
            crate::shared::errors::AppError::Browser(error) => match error {
                crate::shared::errors::BrowserError::InvalidUrl(_)
                | crate::shared::errors::BrowserError::InvalidSetting(_)
                | crate::shared::errors::BrowserError::JobIncomplete { .. } => Self::Validation,
                crate::shared::errors::BrowserError::Download(_) => Self::Network,
                crate::shared::errors::BrowserError::QueueFull
                | crate::shared::errors::BrowserError::DownloadAlreadyActive => Self::Conflict,
                crate::shared::errors::BrowserError::DownloadConfirmationUnavailable => {
                    Self::NotFound
                }
                crate::shared::errors::BrowserError::Io(_) => Self::Io,
                crate::shared::errors::BrowserError::Db(_) => Self::Database,
                crate::shared::errors::BrowserError::WindowUnavailable
                | crate::shared::errors::BrowserError::WebviewNotFound { .. }
                | crate::shared::errors::BrowserError::Import(_)
                | crate::shared::errors::BrowserError::QueueClosed => Self::External,
            },
            crate::shared::errors::AppError::Scanner(error) => match error {
                crate::shared::errors::ScannerError::Security(_)
                | crate::shared::errors::ScannerError::PathEscape { .. } => Self::Permission,
                crate::shared::errors::ScannerError::PathNotFound { .. }
                | crate::shared::errors::ScannerError::NotADirectory { .. } => Self::NotFound,
                crate::shared::errors::ScannerError::Parse { .. }
                | crate::shared::errors::ScannerError::Validation(_) => Self::Validation,
                crate::shared::errors::ScannerError::Network(_) => Self::Network,
                crate::shared::errors::ScannerError::Io(_) => Self::Io,
                crate::shared::errors::ScannerError::Db(_) => Self::Database,
            },
            crate::shared::errors::AppError::Security(_) => Self::Permission,
            crate::shared::errors::AppError::NotFound(_)
            | crate::shared::errors::AppError::RuntimePathNotFound { .. } => Self::NotFound,
            crate::shared::errors::AppError::Internal(_) => Self::Invariant,
            crate::shared::errors::AppError::Db(_) => Self::Database,
            crate::shared::errors::AppError::Validation(_)
            | crate::shared::errors::AppError::ArchivePasswordRequired
            | crate::shared::errors::AppError::ArchivePasswordIncorrect => Self::Validation,
            crate::shared::errors::AppError::ArchiveUnsupported { .. } => Self::Unsupported,
            crate::shared::errors::AppError::Io(_) => Self::Io,
            crate::shared::errors::AppError::DuplicateConflict(_) => Self::Conflict,
            crate::shared::errors::AppError::FileInUse { .. }
            | crate::shared::errors::AppError::PathBusy { .. } => Self::External,
            crate::shared::errors::AppError::ObjectHasMods(_)
            | crate::shared::errors::AppError::ExplorerSnapshotExpired => Self::Conflict,
            crate::shared::errors::AppError::Cancelled => Self::Cancelled,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Unknown => "unknown",
            Self::Validation => "validation",
            Self::Io => "io",
            Self::Database => "database",
            Self::Network => "network",
            Self::Conflict => "conflict",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::Panic => "panic",
            Self::Invariant => "invariant",
            Self::Permission => "permission",
            Self::NotFound => "not_found",
            Self::Unsupported => "unsupported",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetryEvent {
    pub operation: TelemetryOperation,
    pub outcome: TelemetryOutcome,
    pub error_code: TelemetryErrorCode,
    pub duration: Option<Duration>,
}

impl TelemetryEvent {
    pub const fn new(
        operation: TelemetryOperation,
        outcome: TelemetryOutcome,
        error_code: TelemetryErrorCode,
    ) -> Self {
        Self {
            operation,
            outcome,
            error_code,
            duration: None,
        }
    }

    pub const fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }
}

/// Bounded, best-effort telemetry sink used by latency-sensitive event loops.
/// Diagnostics must never delay a reconcile result or grow an unbounded task
/// queue when the local database is busy.
#[derive(Clone)]
pub struct TelemetrySink {
    sender: tokio::sync::mpsc::Sender<Vec<TelemetryEvent>>,
}

impl TelemetrySink {
    pub fn start(store: TelemetryStore) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Vec<TelemetryEvent>>(64);
        tauri::async_runtime::spawn(async move {
            while let Some(mut batch) = receiver.recv().await {
                while let Ok(next) = receiver.try_recv() {
                    batch.extend(next);
                    if batch.len() >= 128 {
                        break;
                    }
                }
                let _ = store.record_rollup_batch(&batch, Utc::now()).await;
            }
        });
        Self { sender }
    }

    pub fn try_enqueue(&self, events: impl IntoIterator<Item = TelemetryEvent>) {
        let batch = events.into_iter().collect::<Vec<_>>();
        if batch.is_empty() {
            return;
        }
        let _ = self.sender.try_send(batch);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryRollup {
    pub day_utc: String,
    pub release: String,
    pub operation: String,
    pub outcome: String,
    pub error_code: String,
    pub count: i64,
    pub duration_ms_total: i64,
    pub duration_sample_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorFingerprint {
    pub release: String,
    pub operation: TelemetryOperation,
    pub error_code: TelemetryErrorCode,
    pub digest: String,
}

impl ErrorFingerprint {
    /// Produces a stable digest while retaining no stack frame, error message,
    /// filesystem path, or user data in the resulting value.
    pub fn from_stack_frame(
        release: &str,
        operation: TelemetryOperation,
        error_code: TelemetryErrorCode,
        top_stack_frame: Option<&str>,
    ) -> Self {
        let release = normalize_release(release);
        let mut hasher = Sha256::new();
        hasher.update(release.as_bytes());
        hasher.update([0]);
        hasher.update(operation.as_str().as_bytes());
        hasher.update([0]);
        hasher.update(error_code.as_str().as_bytes());
        hasher.update([0]);
        if let Some(frame) = top_stack_frame {
            hasher.update(frame.as_bytes());
        }

        Self {
            release,
            operation,
            error_code,
            digest: format!("{:x}", hasher.finalize()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FingerprintRecord {
    pub fingerprint: ErrorFingerprint,
    pub first_seen_at_utc: String,
    pub last_seen_at_utc: String,
    pub occurrence_count: i64,
    pub reported_at_utc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FingerprintDisposition {
    New(FingerprintRecord),
    Existing(FingerprintRecord),
    SuppressedAtCapacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashSource {
    RustPanic,
    PreviousSession,
    Webview,
}

impl CrashSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RustPanic => "rust_panic",
            Self::PreviousSession => "previous_session",
            Self::Webview => "webview",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCrashReport {
    pub fingerprint: ErrorFingerprint,
    pub source: CrashSource,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryExportState {
    pub last_attempt_at_utc: Option<String>,
    pub last_success_at_utc: Option<String>,
    pub last_status: ExportStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportStatus {
    Never,
    Success,
    Failed,
}

impl ExportStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "success" => Self::Success,
            "failed" => Self::Failed,
            _ => Self::Never,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CleanupStats {
    pub rollups: u64,
    pub fingerprints: u64,
    pub pending_crash_reports: u64,
}

#[derive(Clone)]
pub struct TelemetryStore {
    pool: SqlitePool,
}

impl TelemetryStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn record_rollup(
        &self,
        release: &str,
        event: TelemetryEvent,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), TelemetryError> {
        let duration_ms = event
            .duration
            .map(duration_to_i64_millis)
            .unwrap_or_default();
        let duration_sample_count = if event.duration.is_some() { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO telemetry_rollups (
                day_utc, release, operation, outcome, error_code,
                count, duration_ms_total, duration_sample_count
             ) VALUES (?, ?, ?, ?, ?, 1, ?, ?)
             ON CONFLICT(day_utc, release, operation, outcome, error_code) DO UPDATE SET
                count = telemetry_rollups.count + 1,
                duration_ms_total = telemetry_rollups.duration_ms_total + excluded.duration_ms_total,
                duration_sample_count = telemetry_rollups.duration_sample_count + excluded.duration_sample_count",
        )
        .bind(utc_day(occurred_at))
        .bind(normalize_release(release))
        .bind(event.operation.as_str())
        .bind(event.outcome.as_str())
        .bind(event.error_code.as_str())
        .bind(duration_ms)
        .bind(duration_sample_count)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_rollup_batch(
        &self,
        events: &[TelemetryEvent],
        occurred_at: DateTime<Utc>,
    ) -> Result<(), TelemetryError> {
        if events.is_empty() {
            return Ok(());
        }
        let mut transaction = self.pool.begin().await?;
        for event in events {
            let duration_ms = event
                .duration
                .map(duration_to_i64_millis)
                .unwrap_or_default();
            let duration_sample_count = if event.duration.is_some() { 1 } else { 0 };
            sqlx::query(
                "INSERT INTO telemetry_rollups (
                    day_utc, release, operation, outcome, error_code,
                    count, duration_ms_total, duration_sample_count
                 ) VALUES (?, ?, ?, ?, ?, 1, ?, ?)
                 ON CONFLICT(day_utc, release, operation, outcome, error_code) DO UPDATE SET
                    count = telemetry_rollups.count + 1,
                    duration_ms_total = telemetry_rollups.duration_ms_total + excluded.duration_ms_total,
                    duration_sample_count = telemetry_rollups.duration_sample_count + excluded.duration_sample_count",
            )
            .bind(utc_day(occurred_at))
            .bind(normalize_release(env!("CARGO_PKG_VERSION")))
            .bind(event.operation.as_str())
            .bind(event.outcome.as_str())
            .bind(event.error_code.as_str())
            .bind(duration_ms)
            .bind(duration_sample_count)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_rollups(&self) -> Result<Vec<TelemetryRollup>, TelemetryError> {
        let rows = sqlx::query(
            "SELECT day_utc, release, operation, outcome, error_code,
                    count, duration_ms_total, duration_sample_count
             FROM telemetry_rollups
             ORDER BY day_utc, release, operation, outcome, error_code",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(rollup_from_row).collect()
    }

    /// Returns only the unacknowledged portion of each rollup. The transport
    /// must pass these exact delta rows to [`Self::mark_rollups_exported`]
    /// after a successful response.
    pub async fn list_pending_rollups(&self) -> Result<Vec<TelemetryRollup>, TelemetryError> {
        let rows = sqlx::query(
            "SELECT day_utc, release, operation, outcome, error_code,
                    count - exported_count AS count,
                    duration_ms_total - exported_duration_ms_total AS duration_ms_total,
                    duration_sample_count - exported_duration_sample_count AS duration_sample_count
             FROM telemetry_rollups
             WHERE count > exported_count
                OR duration_ms_total > exported_duration_ms_total
                OR duration_sample_count > exported_duration_sample_count
             ORDER BY day_utc, release, operation, outcome, error_code",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(rollup_from_row).collect()
    }

    /// Acknowledges the delta rows that a transport has received. Adding the
    /// acknowledged delta rather than copying the current totals preserves
    /// events recorded while the request was in flight.
    pub async fn mark_rollups_exported(
        &self,
        rollups: &[TelemetryRollup],
    ) -> Result<(), TelemetryError> {
        let mut transaction = self.pool.begin().await?;
        for rollup in rollups {
            sqlx::query(
                "UPDATE telemetry_rollups
                 SET exported_count = MIN(count, exported_count + ?),
                     exported_duration_ms_total = MIN(
                         duration_ms_total, exported_duration_ms_total + ?
                     ),
                     exported_duration_sample_count = MIN(
                         duration_sample_count, exported_duration_sample_count + ?
                     )
                 WHERE day_utc = ? AND release = ? AND operation = ?
                   AND outcome = ? AND error_code = ?",
            )
            .bind(rollup.count)
            .bind(rollup.duration_ms_total)
            .bind(rollup.duration_sample_count)
            .bind(&rollup.day_utc)
            .bind(&rollup.release)
            .bind(&rollup.operation)
            .bind(&rollup.outcome)
            .bind(&rollup.error_code)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn record_fingerprint(
        &self,
        fingerprint: ErrorFingerprint,
        observed_at: DateTime<Utc>,
    ) -> Result<FingerprintDisposition, TelemetryError> {
        let timestamp = utc_timestamp(observed_at);
        let existing =
            load_fingerprint(&self.pool, &fingerprint.release, &fingerprint.digest).await?;
        if let Some(existing) = existing {
            sqlx::query(
                "UPDATE error_fingerprints
                 SET occurrence_count = occurrence_count + 1, last_seen_at_utc = ?
                 WHERE release = ? AND fingerprint = ?",
            )
            .bind(&timestamp)
            .bind(&fingerprint.release)
            .bind(&fingerprint.digest)
            .execute(&self.pool)
            .await?;

            return Ok(FingerprintDisposition::Existing(FingerprintRecord {
                fingerprint,
                last_seen_at_utc: timestamp,
                occurrence_count: existing.occurrence_count + 1,
                ..existing
            }));
        }

        let insert = sqlx::query(
            "INSERT INTO error_fingerprints (
                release, fingerprint, operation, error_code,
                first_seen_at_utc, last_seen_at_utc, occurrence_count
             ) VALUES (?, ?, ?, ?, ?, ?, 1)",
        )
        .bind(&fingerprint.release)
        .bind(&fingerprint.digest)
        .bind(fingerprint.operation.as_str())
        .bind(fingerprint.error_code.as_str())
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&self.pool)
        .await?;

        if insert.rows_affected() == 0 {
            return Ok(FingerprintDisposition::SuppressedAtCapacity);
        }

        Ok(FingerprintDisposition::New(FingerprintRecord {
            fingerprint,
            first_seen_at_utc: timestamp.clone(),
            last_seen_at_utc: timestamp,
            occurrence_count: 1,
            reported_at_utc: None,
        }))
    }

    pub async fn list_unreported_fingerprints(
        &self,
    ) -> Result<Vec<FingerprintRecord>, TelemetryError> {
        let rows = sqlx::query(
            "SELECT release, fingerprint, operation, error_code, first_seen_at_utc,
                    last_seen_at_utc, occurrence_count, reported_at_utc
             FROM error_fingerprints
             WHERE reported_at_utc IS NULL
             ORDER BY first_seen_at_utc",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(fingerprint_from_row).collect()
    }

    pub async fn mark_fingerprint_reported(
        &self,
        fingerprint: &ErrorFingerprint,
        reported_at: DateTime<Utc>,
    ) -> Result<(), TelemetryError> {
        sqlx::query(
            "UPDATE error_fingerprints
             SET reported_at_utc = ?
             WHERE release = ? AND fingerprint = ?",
        )
        .bind(utc_timestamp(reported_at))
        .bind(&fingerprint.release)
        .bind(&fingerprint.digest)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_pending_crash(
        &self,
        report: PendingCrashReport,
    ) -> Result<(), TelemetryError> {
        sqlx::query(
            "INSERT INTO pending_crash_report (
                singleton, release, operation, error_code, fingerprint, source, created_at_utc
             ) VALUES (1, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(singleton) DO UPDATE SET
                release = excluded.release,
                operation = excluded.operation,
                error_code = excluded.error_code,
                fingerprint = excluded.fingerprint,
                source = excluded.source,
                created_at_utc = excluded.created_at_utc",
        )
        .bind(&report.fingerprint.release)
        .bind(report.fingerprint.operation.as_str())
        .bind(report.fingerprint.error_code.as_str())
        .bind(&report.fingerprint.digest)
        .bind(report.source.as_str())
        .bind(&report.created_at_utc)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn pending_crash(&self) -> Result<Option<PendingCrashReport>, TelemetryError> {
        let row = sqlx::query(
            "SELECT release, operation, error_code, fingerprint, source, created_at_utc
             FROM pending_crash_report WHERE singleton = 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(pending_crash_from_row).transpose()
    }

    pub async fn clear_pending_crash(&self) -> Result<(), TelemetryError> {
        sqlx::query("DELETE FROM pending_crash_report WHERE singleton = 1")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn export_state(&self) -> Result<TelemetryExportState, TelemetryError> {
        let row = sqlx::query(
            "SELECT last_attempt_at_utc, last_success_at_utc, last_status
             FROM telemetry_export_state WHERE stream = ?",
        )
        .bind(EXPORT_STREAM)
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => TelemetryExportState {
                last_attempt_at_utc: row.try_get("last_attempt_at_utc")?,
                last_success_at_utc: row.try_get("last_success_at_utc")?,
                last_status: ExportStatus::parse(row.try_get::<String, _>("last_status")?.as_str()),
            },
            None => TelemetryExportState {
                last_attempt_at_utc: None,
                last_success_at_utc: None,
                last_status: ExportStatus::Never,
            },
        })
    }

    pub async fn should_attempt_scheduled_export(
        &self,
        now: DateTime<Utc>,
    ) -> Result<bool, TelemetryError> {
        let state = self.export_state().await?;
        let Some(last_attempt) = state.last_attempt_at_utc else {
            return Ok(true);
        };
        let Ok(last_attempt) = DateTime::parse_from_rfc3339(&last_attempt) else {
            return Ok(true);
        };

        Ok(now.signed_duration_since(last_attempt.with_timezone(&Utc)) >= ChronoDuration::days(1))
    }

    pub async fn record_export_attempt(
        &self,
        attempted_at: DateTime<Utc>,
        status: ExportStatus,
    ) -> Result<(), TelemetryError> {
        let timestamp = utc_timestamp(attempted_at);
        sqlx::query(
            "INSERT INTO telemetry_export_state (
                stream, last_attempt_at_utc, last_success_at_utc, last_status
             ) VALUES (?, ?, CASE WHEN ? = 'success' THEN ? ELSE NULL END, ?)
             ON CONFLICT(stream) DO UPDATE SET
                last_attempt_at_utc = excluded.last_attempt_at_utc,
                last_success_at_utc = CASE
                    WHEN excluded.last_status = 'success' THEN excluded.last_attempt_at_utc
                    ELSE telemetry_export_state.last_success_at_utc
                END,
                last_status = excluded.last_status",
        )
        .bind(EXPORT_STREAM)
        .bind(&timestamp)
        .bind(status.as_str())
        .bind(&timestamp)
        .bind(status.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn cleanup_expired(
        &self,
        now: DateTime<Utc>,
    ) -> Result<CleanupStats, TelemetryError> {
        let oldest_day_to_keep = utc_day(now - ChronoDuration::days(RETENTION_DAYS - 1));
        let oldest_timestamp_to_keep = utc_timestamp(now - ChronoDuration::days(RETENTION_DAYS));
        let mut transaction = self.pool.begin().await?;

        let rollups = sqlx::query("DELETE FROM telemetry_rollups WHERE day_utc < ?")
            .bind(oldest_day_to_keep)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        let fingerprints = sqlx::query("DELETE FROM error_fingerprints WHERE last_seen_at_utc < ?")
            .bind(&oldest_timestamp_to_keep)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        let pending_crash_reports =
            sqlx::query("DELETE FROM pending_crash_report WHERE created_at_utc < ?")
                .bind(&oldest_timestamp_to_keep)
                .execute(&mut *transaction)
                .await?
                .rows_affected();
        transaction.commit().await?;

        Ok(CleanupStats {
            rollups,
            fingerprints,
            pending_crash_reports,
        })
    }

    /// Removes every local telemetry artifact. Call this synchronously with a
    /// user's opt-out so no pre-consent data can be exported later.
    pub async fn purge_all(&self) -> Result<(), TelemetryError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query("DELETE FROM telemetry_rollups")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM error_fingerprints")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM pending_crash_report")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM telemetry_export_state")
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}

/// Records a failure from work that has no IPC response to cross the shared
/// frontend boundary. Only the bounded error code is persisted.
pub async fn record_background_failure(
    app: &tauri::AppHandle,
    error: &crate::shared::errors::AppError,
) {
    use tauri::Manager;

    let diagnostics_enabled = app
        .try_state::<crate::modules::settings::application::config::ConfigService>()
        .is_some_and(|config| config.get_settings().diagnostics.telemetry_enabled);
    if !diagnostics_enabled {
        return;
    }
    let Some(telemetry) = app
        .try_state::<TelemetryStore>()
        .map(|state| state.inner().clone())
    else {
        return;
    };
    let event = TelemetryEvent::new(
        TelemetryOperation::Error,
        TelemetryOutcome::Failed,
        TelemetryErrorCode::from_app_error(error),
    );
    let _ = telemetry
        .record_rollup(env!("CARGO_PKG_VERSION"), event, Utc::now())
        .await;
}

fn rollup_from_row(row: sqlx::sqlite::SqliteRow) -> Result<TelemetryRollup, TelemetryError> {
    Ok(TelemetryRollup {
        day_utc: row.try_get("day_utc")?,
        release: row.try_get("release")?,
        operation: row.try_get("operation")?,
        outcome: row.try_get("outcome")?,
        error_code: row.try_get("error_code")?,
        count: row.try_get("count")?,
        duration_ms_total: row.try_get("duration_ms_total")?,
        duration_sample_count: row.try_get("duration_sample_count")?,
    })
}

async fn load_fingerprint(
    pool: &SqlitePool,
    release: &str,
    digest: &str,
) -> Result<Option<FingerprintRecord>, TelemetryError> {
    let row = sqlx::query(
        "SELECT release, fingerprint, operation, error_code, first_seen_at_utc,
                last_seen_at_utc, occurrence_count, reported_at_utc
         FROM error_fingerprints WHERE release = ? AND fingerprint = ?",
    )
    .bind(release)
    .bind(digest)
    .fetch_optional(pool)
    .await?;

    row.map(fingerprint_from_row).transpose()
}

fn fingerprint_from_row(row: sqlx::sqlite::SqliteRow) -> Result<FingerprintRecord, TelemetryError> {
    let release: String = row.try_get("release")?;
    let operation = telemetry_operation_from_str(&row.try_get::<String, _>("operation")?);
    let error_code = telemetry_error_code_from_str(&row.try_get::<String, _>("error_code")?);

    Ok(FingerprintRecord {
        fingerprint: ErrorFingerprint {
            release,
            operation,
            error_code,
            digest: row.try_get("fingerprint")?,
        },
        first_seen_at_utc: row.try_get("first_seen_at_utc")?,
        last_seen_at_utc: row.try_get("last_seen_at_utc")?,
        occurrence_count: row.try_get("occurrence_count")?,
        reported_at_utc: row.try_get("reported_at_utc")?,
    })
}

fn pending_crash_from_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<PendingCrashReport, TelemetryError> {
    let source = match row.try_get::<String, _>("source")?.as_str() {
        "rust_panic" => CrashSource::RustPanic,
        "webview" => CrashSource::Webview,
        _ => CrashSource::PreviousSession,
    };
    Ok(PendingCrashReport {
        fingerprint: ErrorFingerprint {
            release: row.try_get("release")?,
            operation: telemetry_operation_from_str(&row.try_get::<String, _>("operation")?),
            error_code: telemetry_error_code_from_str(&row.try_get::<String, _>("error_code")?),
            digest: row.try_get("fingerprint")?,
        },
        source,
        created_at_utc: row.try_get("created_at_utc")?,
    })
}

fn telemetry_operation_from_str(value: &str) -> TelemetryOperation {
    match value {
        "onboarding" => TelemetryOperation::Onboarding,
        "onboarding_preparation" => TelemetryOperation::OnboardingPreparation,
        "onboarding_classification" => TelemetryOperation::OnboardingClassification,
        "onboarding_apply" => TelemetryOperation::OnboardingApply,
        "onboarding_recheck" => TelemetryOperation::OnboardingRecheck,
        "classification" => TelemetryOperation::Classification,
        "classification_review" => TelemetryOperation::ClassificationReview,
        "auto_match" => TelemetryOperation::AutoMatch,
        "watcher" => TelemetryOperation::Watcher,
        "reconcile" => TelemetryOperation::Reconcile,
        "toggle" => TelemetryOperation::Toggle,
        "bulk_action" => TelemetryOperation::BulkAction,
        "import" => TelemetryOperation::Import,
        "extract" => TelemetryOperation::Extract,
        "collection_apply" => TelemetryOperation::CollectionApply,
        "restore" => TelemetryOperation::Restore,
        "launch" => TelemetryOperation::Launch,
        _ => TelemetryOperation::Error,
    }
}

fn telemetry_error_code_from_str(value: &str) -> TelemetryErrorCode {
    match value {
        "none" => TelemetryErrorCode::None,
        "validation" => TelemetryErrorCode::Validation,
        "io" => TelemetryErrorCode::Io,
        "database" => TelemetryErrorCode::Database,
        "network" => TelemetryErrorCode::Network,
        "conflict" => TelemetryErrorCode::Conflict,
        "timeout" => TelemetryErrorCode::Timeout,
        "cancelled" => TelemetryErrorCode::Cancelled,
        "panic" => TelemetryErrorCode::Panic,
        "invariant" => TelemetryErrorCode::Invariant,
        "permission" => TelemetryErrorCode::Permission,
        "not_found" => TelemetryErrorCode::NotFound,
        "unsupported" => TelemetryErrorCode::Unsupported,
        "external" => TelemetryErrorCode::External,
        _ => TelemetryErrorCode::Unknown,
    }
}

fn normalize_release(value: &str) -> String {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+'))
    {
        return "unknown".to_string();
    }
    value.to_string()
}

fn duration_to_i64_millis(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

fn utc_day(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%d").to_string()
}

fn utc_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMarkerPayload {
    release: String,
    started_at_utc: String,
    panic_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviousSession {
    pub release: String,
    pub panic_fingerprint: Option<String>,
}

/// A small file marker used by startup/shutdown and the panic hook. It contains
/// no raw panic message or paths and does not perform any network I/O.
#[derive(Debug, Clone)]
pub struct CrashMarker {
    path: PathBuf,
}

impl CrashMarker {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn begin_session(
        &self,
        release: &str,
        started_at: DateTime<Utc>,
    ) -> Result<(), TelemetryError> {
        self.write(SessionMarkerPayload {
            release: normalize_release(release),
            started_at_utc: utc_timestamp(started_at),
            panic_fingerprint: None,
        })
    }

    pub fn record_panic(&self, fingerprint: &ErrorFingerprint) -> Result<(), TelemetryError> {
        self.write(SessionMarkerPayload {
            release: fingerprint.release.clone(),
            started_at_utc: utc_timestamp(Utc::now()),
            panic_fingerprint: Some(fingerprint.digest.clone()),
        })
    }

    pub fn take_previous_session(&self) -> Result<Option<PreviousSession>, TelemetryError> {
        if !self.path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&self.path)?;
        let previous = match serde_json::from_slice::<SessionMarkerPayload>(&bytes) {
            Ok(payload) => PreviousSession {
                release: normalize_release(&payload.release),
                panic_fingerprint: payload.panic_fingerprint.filter(|value| is_digest(value)),
            },
            Err(_) => PreviousSession {
                release: "unknown".to_string(),
                panic_fingerprint: None,
            },
        };
        self.clear()?;
        Ok(Some(previous))
    }

    pub fn clear(&self) -> Result<(), TelemetryError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn write(&self, payload: SessionMarkerPayload) -> Result<(), TelemetryError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized =
            serde_json::to_vec(&payload).map_err(|_| TelemetryError::CrashMarkerSerialization)?;
        fs::write(&self.path, serialized)?;
        Ok(())
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    async fn store() -> TelemetryStore {
        let context = crate::test_utils::init_test_db().await;
        TelemetryStore::new(context.pool)
    }

    fn at(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, 0, 0)
            .single()
            .expect("valid fixture time")
    }

    #[test]
    fn fingerprint_hashes_stack_frame_without_retaining_it() {
        let frame = "C:\\Users\\alice\\Games\\mods\\broken.rs:42";
        let first = ErrorFingerprint::from_stack_frame(
            "1.2.3",
            TelemetryOperation::Reconcile,
            TelemetryErrorCode::Io,
            Some(frame),
        );
        let second = ErrorFingerprint::from_stack_frame(
            "1.2.3",
            TelemetryOperation::Reconcile,
            TelemetryErrorCode::Io,
            Some(frame),
        );

        assert_eq!(first.digest, second.digest);
        assert_eq!(first.digest.len(), 64);
        assert!(!first.digest.contains("alice"));
        assert_eq!(normalize_release("path/should-not-be-a-release"), "unknown");
    }

    #[test]
    fn maps_nested_app_errors_to_actionable_codes() {
        use crate::shared::errors::{
            AppError, BrowserError, CollectionError, MetadataError, ScannerError,
        };

        let cases = [
            (
                AppError::Collection(CollectionError::Db("db".to_string())),
                TelemetryErrorCode::Database,
            ),
            (
                AppError::Metadata(MetadataError::Security("path".to_string())),
                TelemetryErrorCode::Permission,
            ),
            (
                AppError::Browser(BrowserError::Io("disk".to_string())),
                TelemetryErrorCode::Io,
            ),
            (
                AppError::Scanner(ScannerError::Network("offline".to_string())),
                TelemetryErrorCode::Network,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(TelemetryErrorCode::from_app_error(&error), expected);
        }
    }

    #[test]
    fn onboarding_phase_operation_labels_round_trip_without_payload_fields() {
        for operation in [
            TelemetryOperation::OnboardingPreparation,
            TelemetryOperation::OnboardingClassification,
            TelemetryOperation::OnboardingApply,
            TelemetryOperation::OnboardingRecheck,
        ] {
            let label = operation.as_str();
            assert_eq!(TelemetryOperation::from_label(label), operation);
            assert_eq!(telemetry_operation_from_str(label), operation);
        }
    }

    #[tokio::test]
    async fn persists_only_the_fingerprint_not_the_stack_frame() {
        let context = crate::test_utils::init_test_db().await;
        let pool = context.pool;
        let store = TelemetryStore::new(pool.clone());
        let raw_frame = "C:\\Users\\alice\\Games\\mods\\private-folder\\broken.rs:42";
        let fingerprint = ErrorFingerprint::from_stack_frame(
            "1.0.0",
            TelemetryOperation::Error,
            TelemetryErrorCode::Panic,
            Some(raw_frame),
        );

        store
            .record_fingerprint(fingerprint, at(2026, 9, 13, 10))
            .await
            .expect("record fingerprint");
        let stored: String = sqlx::query_scalar("SELECT fingerprint FROM error_fingerprints")
            .fetch_one(&pool)
            .await
            .expect("read persisted fingerprint");

        assert_eq!(stored.len(), 64);
        assert!(!stored.contains("alice"));
        assert!(!stored.contains("private-folder"));
    }

    #[tokio::test]
    async fn aggregates_low_cardinality_rollups_by_utc_day() {
        let store = store().await;
        let event = TelemetryEvent::new(
            TelemetryOperation::Reconcile,
            TelemetryOutcome::Success,
            TelemetryErrorCode::None,
        )
        .with_duration(Duration::from_millis(125));
        store
            .record_rollup("1.0.0", event, at(2026, 9, 13, 23))
            .await
            .expect("record first rollup");
        store
            .record_rollup("1.0.0", event, at(2026, 9, 13, 23))
            .await
            .expect("record second rollup");

        let rollups = store.list_rollups().await.expect("read rollups");
        assert_eq!(rollups.len(), 1);
        assert_eq!(rollups[0].day_utc, "2026-09-13");
        assert_eq!(rollups[0].count, 2);
        assert_eq!(rollups[0].duration_ms_total, 250);
        assert_eq!(rollups[0].duration_sample_count, 2);

        let pending = store
            .list_pending_rollups()
            .await
            .expect("read pending rollups");
        assert_eq!(pending, rollups);
        store
            .mark_rollups_exported(&pending)
            .await
            .expect("acknowledge exported delta");
        assert!(store
            .list_pending_rollups()
            .await
            .expect("read acknowledged rollups")
            .is_empty());

        store
            .record_rollup("1.0.0", event, at(2026, 9, 13, 23))
            .await
            .expect("record a post-export event");
        let pending = store.list_pending_rollups().await.expect("read new delta");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].count, 1);
        assert_eq!(pending[0].duration_ms_total, 125);
    }

    #[tokio::test]
    async fn deduplicates_fingerprints_and_enforces_release_cap() {
        let store = store().await;
        let now = at(2026, 9, 13, 10);
        let repeated = ErrorFingerprint::from_stack_frame(
            "1.0.0",
            TelemetryOperation::Error,
            TelemetryErrorCode::Panic,
            Some("first"),
        );
        assert!(matches!(
            store
                .record_fingerprint(repeated.clone(), now)
                .await
                .unwrap(),
            FingerprintDisposition::New(_)
        ));
        let repeated_result = store.record_fingerprint(repeated, now).await.unwrap();
        assert!(matches!(
            repeated_result,
            FingerprintDisposition::Existing(FingerprintRecord {
                occurrence_count: 2,
                ..
            })
        ));

        for value in 0..99 {
            let fingerprint = ErrorFingerprint::from_stack_frame(
                "1.0.0",
                TelemetryOperation::Error,
                TelemetryErrorCode::Unknown,
                Some(&format!("frame-{value}")),
            );
            assert!(matches!(
                store.record_fingerprint(fingerprint, now).await.unwrap(),
                FingerprintDisposition::New(_)
            ));
        }

        let overflow = ErrorFingerprint::from_stack_frame(
            "1.0.0",
            TelemetryOperation::Error,
            TelemetryErrorCode::Unknown,
            Some("overflow"),
        );
        assert!(matches!(
            store.record_fingerprint(overflow, now).await.unwrap(),
            FingerprintDisposition::SuppressedAtCapacity
        ));
    }

    #[tokio::test]
    async fn retains_one_pending_crash_and_purges_expired_data() {
        let store = store().await;
        let old = at(2026, 9, 1, 0);
        let now = at(2026, 9, 13, 0);
        let first = ErrorFingerprint::from_stack_frame(
            "1.0.0",
            TelemetryOperation::Error,
            TelemetryErrorCode::Panic,
            Some("first"),
        );
        store
            .record_rollup(
                "1.0.0",
                TelemetryEvent::new(
                    TelemetryOperation::Error,
                    TelemetryOutcome::Failed,
                    TelemetryErrorCode::Panic,
                ),
                old,
            )
            .await
            .unwrap();
        store.record_fingerprint(first.clone(), old).await.unwrap();
        store
            .save_pending_crash(PendingCrashReport {
                fingerprint: first,
                source: CrashSource::RustPanic,
                created_at_utc: utc_timestamp(old),
            })
            .await
            .unwrap();

        let replacement = ErrorFingerprint::from_stack_frame(
            "1.0.0",
            TelemetryOperation::Error,
            TelemetryErrorCode::Unknown,
            Some("replacement"),
        );
        store
            .save_pending_crash(PendingCrashReport {
                fingerprint: replacement.clone(),
                source: CrashSource::Webview,
                created_at_utc: utc_timestamp(now),
            })
            .await
            .unwrap();
        assert_eq!(
            store
                .pending_crash()
                .await
                .unwrap()
                .unwrap()
                .fingerprint
                .digest,
            replacement.digest
        );

        let cleanup = store.cleanup_expired(now).await.unwrap();
        assert_eq!(cleanup.rollups, 1);
        assert_eq!(cleanup.fingerprints, 1);
        assert_eq!(cleanup.pending_crash_reports, 0);
    }

    #[tokio::test]
    async fn schedules_daily_export_from_last_attempt() {
        let store = store().await;
        let now = at(2026, 9, 13, 10);
        assert!(store.should_attempt_scheduled_export(now).await.unwrap());
        store
            .record_export_attempt(now, ExportStatus::Failed)
            .await
            .unwrap();
        assert!(!store
            .should_attempt_scheduled_export(now + ChronoDuration::hours(23))
            .await
            .unwrap());
        assert!(store
            .should_attempt_scheduled_export(now + ChronoDuration::hours(24))
            .await
            .unwrap());
    }

    #[test]
    fn crash_marker_detects_unclosed_session_and_clears_normal_exit() {
        let directory = TempDir::new().expect("create temp directory");
        let marker = CrashMarker::new(directory.path().join("session-marker.json"));
        marker
            .begin_session("1.0.0", at(2026, 9, 13, 10))
            .expect("begin session");
        assert_eq!(
            marker.take_previous_session().unwrap(),
            Some(PreviousSession {
                release: "1.0.0".to_string(),
                panic_fingerprint: None,
            })
        );

        marker
            .begin_session("1.0.0", at(2026, 9, 13, 11))
            .expect("begin session");
        marker.clear().expect("normal exit clears marker");
        assert_eq!(marker.take_previous_session().unwrap(), None);
    }
}
