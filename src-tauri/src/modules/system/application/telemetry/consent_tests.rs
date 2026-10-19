use super::*;
use crate::modules::settings::api::persist_telemetry_consent;
use crate::modules::settings::application::config::ConfigService;

fn event() -> TelemetryEvent {
    TelemetryEvent::new(
        TelemetryOperation::BulkAction,
        TelemetryOutcome::Success,
        TelemetryErrorCode::None,
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opt_out_fences_completion_across_reopt_in() {
    let context = crate::test_utils::init_test_db().await;
    let config = ConfigService::new_for_test_async(context.pool.clone()).await;
    config.set_telemetry_enabled(true).unwrap();
    let store = TelemetryStore::new(context.pool.clone(), true);
    let producer = store.capture();
    let (complete, completion) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        completion.await.unwrap();
        producer
            .record_rollup("1.0.0", event(), Utc::now())
            .await
            .unwrap();
    });

    persist_telemetry_consent(false, &config, &store)
        .await
        .unwrap();
    persist_telemetry_consent(true, &config, &store)
        .await
        .unwrap();
    complete.send(()).unwrap();
    task.await.unwrap();

    assert!(store.list_rollups().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disabled_store_keeps_diagnostics_queue_empty() {
    let context = crate::test_utils::init_test_db().await;
    let config = ConfigService::new_for_test_async(context.pool.clone()).await;
    let store = TelemetryStore::new(context.pool.clone(), false);
    persist_telemetry_consent(false, &config, &store)
        .await
        .unwrap();

    store
        .record_rollup("1.0.0", event(), Utc::now())
        .await
        .unwrap();
    store
        .record_export_attempt(Utc::now(), ExportStatus::Failed)
        .await
        .unwrap();

    assert!(store.list_rollups().await.unwrap().is_empty());
    assert_eq!(
        store.export_state().await.unwrap().last_status,
        ExportStatus::Never
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_settings_save_cannot_bypass_consent_revocation() {
    let context = crate::test_utils::init_test_db().await;
    let config = ConfigService::new_for_test_async(context.pool.clone()).await;
    config.set_telemetry_enabled(true).unwrap();
    let mut snapshot = config.get_settings();
    snapshot.diagnostics.telemetry_enabled = false;

    assert!(config.save_settings(snapshot).is_err());
    assert!(config.get_settings().diagnostics.telemetry_enabled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_batches_and_old_sink_are_rejected_after_revocation() {
    let context = crate::test_utils::init_test_db().await;
    let config = ConfigService::new_for_test_async(context.pool.clone()).await;
    config.set_telemetry_enabled(true).unwrap();
    let store = TelemetryStore::new(context.pool.clone(), true);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(64);
    let sink = TelemetrySink {
        sender,
        store: store.clone(),
    };
    let old_sink = sink.capture();
    old_sink.try_enqueue([event()]);
    let queued = receiver.try_recv().unwrap();

    persist_telemetry_consent(false, &config, &store)
        .await
        .unwrap();
    old_sink.try_enqueue([event()]);
    sink.capture().try_enqueue([event()]);
    assert!(matches!(
        receiver.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
    persist_telemetry_consent(true, &config, &store)
        .await
        .unwrap();
    old_sink.try_enqueue([event()]);
    assert!(matches!(
        receiver.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
    store.persist_batch(queued).await.unwrap();
    assert!(store.list_rollups().await.unwrap().is_empty());

    sink.capture().try_enqueue([event()]);
    store
        .persist_batch(receiver.try_recv().unwrap())
        .await
        .unwrap();
    assert_eq!(store.list_rollups().await.unwrap()[0].count, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opt_out_waits_for_inflight_sql_then_purges_it() {
    let context = crate::test_utils::init_test_db().await;
    let config = ConfigService::new_for_test_async(context.pool.clone()).await;
    config.set_telemetry_enabled(true).unwrap();
    let store = TelemetryStore::new(context.pool.clone(), true);
    let producer = store.capture();
    let connection = context.pool.acquire().await.unwrap();
    let recording = producer.record_rollup("1.0.0", event(), Utc::now());
    tokio::pin!(recording);
    assert!(futures_util::poll!(&mut recording).is_pending());
    let revoke = persist_telemetry_consent(false, &config, &store);
    tokio::pin!(revoke);
    assert!(futures_util::poll!(&mut revoke).is_pending());

    drop(connection);
    recording.await.unwrap();
    revoke.await.unwrap();
    assert!(!config.get_settings().diagnostics.telemetry_enabled);
    assert!(store.list_rollups().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opt_out_write_error_preserves_settings_without_reviving_old_epoch() {
    let context = crate::test_utils::init_test_db().await;
    let config = ConfigService::new_for_test_async(context.pool.clone()).await;
    config.set_telemetry_enabled(true).unwrap();
    let store = TelemetryStore::new(context.pool.clone(), true);
    let producer = store.capture();
    sqlx::query("CREATE TRIGGER reject_diagnostics BEFORE INSERT ON app_settings WHEN NEW.key = 'diagnostics' BEGIN SELECT RAISE(ABORT, 'fixture rejected consent'); END")
        .execute(&context.pool).await.unwrap();

    assert!(persist_telemetry_consent(false, &config, &store)
        .await
        .is_err());
    assert!(config.get_settings().diagnostics.telemetry_enabled);
    assert!(store.is_current());
    assert!(!producer.is_current());
    producer
        .record_rollup("1.0.0", event(), Utc::now())
        .await
        .unwrap();
    assert!(store.list_rollups().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opt_out_purge_error_keeps_committed_consent_and_existing_error() {
    let context = crate::test_utils::init_test_db().await;
    let config = ConfigService::new_for_test_async(context.pool.clone()).await;
    config.set_telemetry_enabled(true).unwrap();
    let store = TelemetryStore::new(context.pool.clone(), true);
    let producer = store.capture();
    context.pool.close().await;

    let error = persist_telemetry_consent(false, &config, &store)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Database error: Could not clear queued diagnostics while opting out"
    );
    assert!(config.get_settings().diagnostics.telemetry_enabled);
    assert!(store.is_current());
    assert!(!producer.is_current());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_fingerprint_crash_and_export_completion_cannot_reappear() {
    let context = crate::test_utils::init_test_db().await;
    let config = ConfigService::new_for_test_async(context.pool.clone()).await;
    config.set_telemetry_enabled(true).unwrap();
    let store = TelemetryStore::new(context.pool.clone(), true);
    let producer = store.capture();
    let fingerprint = ErrorFingerprint::from_stack_frame(
        "1.0.0",
        TelemetryOperation::Error,
        TelemetryErrorCode::Panic,
        None,
    );
    persist_telemetry_consent(false, &config, &store)
        .await
        .unwrap();
    persist_telemetry_consent(true, &config, &store)
        .await
        .unwrap();

    assert_eq!(
        producer
            .record_fingerprint(fingerprint.clone(), Utc::now())
            .await
            .unwrap(),
        FingerprintDisposition::SuppressedWithoutConsent
    );
    producer
        .save_pending_crash(PendingCrashReport {
            fingerprint,
            source: CrashSource::RustPanic,
            created_at_utc: Utc::now().to_rfc3339(),
        })
        .await
        .unwrap();
    producer
        .record_export_attempt(Utc::now(), ExportStatus::Failed)
        .await
        .unwrap();
    assert!(store
        .list_unreported_fingerprints()
        .await
        .unwrap()
        .is_empty());
    assert!(store.pending_crash().await.unwrap().is_none());
    assert_eq!(
        store.export_state().await.unwrap().last_status,
        ExportStatus::Never
    );

    store
        .record_rollup("1.0.0", event(), Utc::now())
        .await
        .unwrap();
    let current = store.list_pending_rollups().await.unwrap();
    producer.mark_rollups_exported(&current).await.unwrap();
    assert_eq!(store.list_pending_rollups().await.unwrap(), current);
}

#[tokio::test]
async fn default_off_preserves_voluntary_local_crash_prompt() {
    let context = crate::test_utils::init_test_db().await;
    let store = TelemetryStore::new(context.pool, false);
    let report = PendingCrashReport {
        fingerprint: ErrorFingerprint::from_stack_frame(
            "1.0.0",
            TelemetryOperation::Error,
            TelemetryErrorCode::Panic,
            None,
        ),
        source: CrashSource::PreviousSession,
        created_at_utc: Utc::now().to_rfc3339(),
    };
    store.save_pending_crash(report.clone()).await.unwrap();
    assert_eq!(store.pending_crash().await.unwrap(), Some(report));
    assert!(!store
        .should_attempt_scheduled_export(Utc::now())
        .await
        .unwrap());
    store.clear_pending_crash().await.unwrap();
    assert!(store.pending_crash().await.unwrap().is_none());
}
