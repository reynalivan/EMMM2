//! Direct OTLP export for the small local telemetry queue.
//!
//! The endpoint and write-only ingest token are build-time values.  A build
//! without both values behaves as if telemetry transport does not exist.

use super::telemetry::{
    ExportStatus, TelemetryErrorCode, TelemetryEvent, TelemetryOperation, TelemetryOutcome,
    TelemetryRollup, TelemetryStore,
};
use chrono::Utc;
use reqwest::{header::AUTHORIZATION, Client, Url};
use serde_json::{json, Value};
use std::time::Duration;

const METRICS_ENDPOINT: Option<&str> = option_env!("EMMM_GRAFANA_OTLP_METRICS_ENDPOINT");
const AUTHORIZATION_VALUE: Option<&str> = option_env!("EMMM_GRAFANA_OTLP_AUTHORIZATION");

#[derive(Clone)]
pub struct TelemetryExporter {
    client: Client,
    endpoint: Option<Url>,
    authorization: Option<String>,
}

impl TelemetryExporter {
    pub fn from_build_environment() -> Self {
        let endpoint = METRICS_ENDPOINT
            .and_then(|value| Url::parse(value.trim()).ok())
            .filter(|value| value.scheme() == "https" && value.path().ends_with("/v1/metrics"));
        let authorization = AUTHORIZATION_VALUE
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(3))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_else(|_| Client::new()),
            endpoint,
            authorization,
        }
    }

    pub fn is_configured(&self) -> bool {
        self.endpoint.is_some() && self.authorization.is_some()
    }

    /// Sends each queued counter delta once. Failures are recorded locally and
    /// are retried at the next lifecycle attempt, never in a tight loop.
    pub async fn export_scheduled(&self, store: &TelemetryStore) {
        if !self.is_configured()
            || !store
                .should_attempt_scheduled_export(Utc::now())
                .await
                .unwrap_or(false)
        {
            return;
        }

        let rollups = match store.list_pending_rollups().await {
            Ok(rollups) => rollups,
            Err(_) => return,
        };

        let result = if rollups.is_empty() {
            Ok(())
        } else {
            self.send_rollups(&rollups).await
        };

        match result {
            Ok(()) => {
                if !rollups.is_empty() && store.mark_rollups_exported(&rollups).await.is_err() {
                    return;
                }
                let _ = store
                    .record_export_attempt(Utc::now(), ExportStatus::Success)
                    .await;
            }
            Err(()) => {
                let _ = store
                    .record_export_attempt(Utc::now(), ExportStatus::Failed)
                    .await;
                let _ = store
                    .record_rollup(
                        env!("CARGO_PKG_VERSION"),
                        TelemetryEvent::new(
                            TelemetryOperation::Error,
                            TelemetryOutcome::Failed,
                            TelemetryErrorCode::Network,
                        ),
                        Utc::now(),
                    )
                    .await;
                log::warn!("Anonymous diagnostics export failed; it will wait until the next daily lifecycle attempt");
            }
        }
    }

    async fn send_rollups(&self, rollups: &[TelemetryRollup]) -> Result<(), ()> {
        let Some(endpoint) = &self.endpoint else {
            return Err(());
        };
        let Some(authorization) = &self.authorization else {
            return Err(());
        };

        let response = self
            .client
            .post(endpoint.clone())
            .header(AUTHORIZATION, authorization)
            .json(&otlp_metrics_payload(rollups))
            .send()
            .await
            .map_err(|_| ())?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(())
        }
    }
}

fn attributes(rollup: &TelemetryRollup) -> Vec<Value> {
    [
        ("release", &rollup.release),
        ("operation", &rollup.operation),
        ("outcome", &rollup.outcome),
        ("error_code", &rollup.error_code),
    ]
    .into_iter()
    .map(|(key, value)| json!({ "key": key, "value": { "stringValue": value } }))
    .collect()
}

fn sum_point(rollup: &TelemetryRollup, value: i64) -> Value {
    json!({
        "attributes": attributes(rollup),
        "asInt": value.to_string(),
        "timeUnixNano": Utc::now().timestamp_nanos_opt().unwrap_or_default().to_string(),
    })
}

fn sum_metric(
    name: &str,
    rollups: &[TelemetryRollup],
    values: impl Fn(&TelemetryRollup) -> i64,
) -> Value {
    json!({
        "name": name,
        "sum": {
            "aggregationTemporality": 1,
            "isMonotonic": true,
            "dataPoints": rollups.iter().map(|rollup| sum_point(rollup, values(rollup))).collect::<Vec<_>>(),
        }
    })
}

fn otlp_metrics_payload(rollups: &[TelemetryRollup]) -> Value {
    json!({
        "resourceMetrics": [{
            "resource": { "attributes": [{ "key": "service.name", "value": { "stringValue": "emmm-desktop" } }] },
            "scopeMetrics": [{
                "scope": { "name": "emmm.telemetry" },
                "metrics": [
                    sum_metric("emmm_operation_total", rollups, |rollup| rollup.count),
                    sum_metric("emmm_operation_duration_ms_total", rollups, |rollup| rollup.duration_ms_total),
                    sum_metric("emmm_operation_duration_samples_total", rollups, |rollup| rollup.duration_sample_count),
                ]
            }]
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_has_only_bounded_attributes() {
        let rollup = TelemetryRollup {
            day_utc: "2026-09-13".to_string(),
            release: "1.0.0".to_string(),
            operation: "reconcile".to_string(),
            outcome: "success".to_string(),
            error_code: "none".to_string(),
            count: 3,
            duration_ms_total: 42,
            duration_sample_count: 3,
        };
        let payload = otlp_metrics_payload(&[rollup]);
        let serialized = payload.to_string();
        assert!(serialized.contains("emmm_operation_total"));
        assert!(!serialized.contains("day_utc"));
        assert!(!serialized.contains("fingerprint"));
    }
}
