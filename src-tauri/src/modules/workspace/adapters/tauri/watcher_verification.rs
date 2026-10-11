//! Private native fixture instrumentation; absent from normal builds.

use crate::modules::mutation::api::MutationCoordinator;
use crate::modules::reconciliation::api::disk_reconcile::{
    orchestrator::DiskReconcileState,
    types::{DiskReconcileResult, RuntimeSyncPhase, RuntimeSyncStatus},
};
use crate::modules::system::api::app::post_apply::{
    keyviewer_runtime_diagnostics, RuntimeSyncPublicationStatus,
};
use crate::modules::workspace::application::scanner::watcher::{
    ExpectedRenameEcho, SuppressionGuard, WatcherSession, WatcherState,
};
use crate::shared::sync::lock;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{Listener, Manager};

const IDENTIFIER: &str = "com.reynalivan.emmm.e2e";
const NONCE_ENV: &str = "EMMM_WATCHER_VERIFICATION_NONCE";
const REPORT_NAME: &str = "watcher-verification.json";
const CASE_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Clone, Copy)]
struct WaitObservation {
    sequence: u64,
    retry_at: Option<Instant>,
    session_generation: u64,
}

#[derive(Clone, Copy)]
struct WakeObservation {
    sequence: u64,
    at: Instant,
    work: bool,
}

#[derive(Clone, Copy)]
struct PublicationBoundary {
    revision: u64,
    game_lease_available: bool,
    operation_lease_available: bool,
    runtime_generation: Option<u64>,
}

#[derive(Default)]
struct Observations {
    wait: Option<WaitObservation>,
    wakes: Vec<WakeObservation>,
    results: Vec<DiskReconcileResult>,
    runtime: Vec<RuntimeSyncStatus>,
    boundaries: Vec<PublicationBoundary>,
}

struct FixtureOwnership {
    root: PathBuf,
    identity: String,
    marker: String,
}

impl FixtureOwnership {
    fn revalidate(&self) -> Result<(), String> {
        if crate::platform::fs::file_utils::filesystem_identity(&self.root).as_ref()
            != Some(&self.identity)
            || std::fs::read_to_string(self.root.join(".emmm-e2e-fixture"))
                .map_err(|error| error.to_string())?
                != self.marker
        {
            return Err("Verification fixture ownership changed; no further writes allowed".into());
        }
        Ok(())
    }
}

struct Verification {
    fixture: FixtureOwnership,
    game_id: String,
    session: Mutex<WatcherSession>,
    armed: AtomicBool,
    observations: Mutex<Observations>,
    changed: tokio::sync::Notify,
}

#[derive(Serialize)]
struct CaseEvidence {
    name: &'static str,
    session_generation: u64,
    before_revision: u64,
    accepted_revision: u64,
    reconcile_event_revision: u64,
    runtime_generation: u64,
    runtime_phase: RuntimeSyncPhase,
    runtime_publication: RuntimeSyncPublicationStatus,
    trigger_wait_sequence: u64,
    accepted_wait_sequence: u64,
    trigger_before_deadline: bool,
    game_lease_available: bool,
    operation_lease_available: bool,
    first_owner_still_suppressed: Option<bool>,
    abort_before_expiry: Option<bool>,
}

#[derive(Serialize)]
struct Report {
    complete: bool,
    cases: Vec<CaseEvidence>,
    error: Option<String>,
}

fn validate_fixture(
    identifier: &str,
    debug: bool,
    nonce: &str,
    mods: &Path,
) -> Result<FixtureOwnership, String> {
    if identifier != IDENTIFIER || !debug {
        return Err("Watcher verification requires the private debug E2E profile".into());
    }
    let nonce = uuid::Uuid::parse_str(nonce).map_err(|_| "Invalid fixture nonce")?;
    let temporary =
        std::fs::canonicalize(std::env::temp_dir()).map_err(|error| error.to_string())?;
    let fixture = temporary.join(format!("EMMM_WatcherVerification_{nonce}"));
    let metadata = std::fs::symlink_metadata(&fixture).map_err(|error| error.to_string())?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || std::fs::canonicalize(&fixture).map_err(|error| error.to_string())? != fixture
        || std::fs::canonicalize(mods).map_err(|error| error.to_string())? != fixture.join("Mods")
    {
        return Err("Watcher verification refused an unowned or rebound fixture".into());
    }
    let marker = fixture.join(".emmm-e2e-fixture");
    if !std::fs::symlink_metadata(&marker)
        .map_err(|error| error.to_string())?
        .file_type()
        .is_file()
    {
        return Err("Fixture ownership marker is not a regular file".into());
    }
    let marker = std::fs::read_to_string(marker).map_err(|error| error.to_string())?;
    uuid::Uuid::parse_str(&marker).map_err(|_| "Invalid fixture ownership marker")?;
    let identity = crate::platform::fs::file_utils::filesystem_identity(&fixture)
        .ok_or("Missing fixture identity")?;
    Ok(FixtureOwnership {
        root: fixture,
        identity,
        marker,
    })
}

pub(super) fn on_wait(
    app: &tauri::AppHandle,
    game_id: &str,
    root: &Path,
    session: &WatcherSession,
    retry_at: Option<tokio::time::Instant>,
) {
    if app.try_state::<Verification>().is_none() {
        let Ok(nonce) = std::env::var(NONCE_ENV) else {
            return;
        };
        // Other saved games may start before the spec seeds its owned fixture.
        let expected = std::env::temp_dir()
            .join(format!("EMMM_WatcherVerification_{nonce}"))
            .join("Mods");
        if crate::shared::path_key::exact_location_key_for_path(root)
            != crate::shared::path_key::exact_location_key_for_path(&expected)
        {
            return;
        }
        let fixture = match validate_fixture(
            &app.config().identifier,
            cfg!(debug_assertions),
            &nonce,
            root,
        ) {
            Ok(fixture) => fixture,
            Err(error) => {
                log::error!("Watcher verification gate: {error}");
                return;
            }
        };
        if app.manage(Verification {
            fixture,
            game_id: game_id.into(),
            session: Mutex::new(session.clone()),
            armed: AtomicBool::new(false),
            observations: Mutex::default(),
            changed: tokio::sync::Notify::new(),
        }) {
            let result_app = app.clone();
            let result_listener = app.listen("disk_reconcile:result", move |event| {
                let state = result_app.state::<Verification>();
                if let Ok(result) = serde_json::from_str::<DiskReconcileResult>(event.payload()) {
                    if result.game_id == state.game_id {
                        let mut observations = lock(&state.observations);
                        if observations.results.len() < 64 {
                            observations.results.push(result);
                        }
                        drop(observations);
                        state.changed.notify_one();
                    }
                }
            });
            let runtime_app = app.clone();
            let runtime_listener = app.listen("runtime_sync:status", move |event| {
                let state = runtime_app.state::<Verification>();
                if let Ok(status) = serde_json::from_str::<RuntimeSyncStatus>(event.payload()) {
                    if status.game_id == state.game_id && status.cause == "recovery" {
                        let mut observations = lock(&state.observations);
                        if observations.runtime.len() < 64 {
                            observations.runtime.push(status);
                        }
                        drop(observations);
                        state.changed.notify_one();
                    }
                }
            });
            let owned_app = app.clone();
            tokio::spawn(async move {
                let mut report = Report {
                    complete: true,
                    cases: Vec::new(),
                    error: None,
                };
                if let Err(error) = wait_for_arm(&owned_app).await {
                    report.error = Some(error);
                }
                owned_app
                    .state::<Verification>()
                    .armed
                    .store(true, Ordering::Release);
                for name in ["last_owner_release", "echo_expiry", "echo_abort"] {
                    if report.error.is_some() {
                        break;
                    }
                    match run_case(&owned_app, name).await {
                        Ok(case) => report.cases.push(case),
                        Err(error) => {
                            report.error = Some(format!("{name}: {error}"));
                            break;
                        }
                    }
                }
                owned_app.unlisten(result_listener);
                owned_app.unlisten(runtime_listener);
                let probe = owned_app.state::<Verification>();
                let report_path = probe.fixture.root.join(REPORT_NAME);
                match serde_json::to_vec(&report)
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| {
                        probe.fixture.revalidate()?;
                        crate::platform::fs::atomic_file::atomic_write(&report_path, &bytes)
                            .map_err(|error| error.to_string())
                    }) {
                    Ok(()) => (),
                    Err(error) => log::error!("Could not record watcher verification: {error}"),
                }
            });
        }
    }
    if let Some(probe) = app.try_state::<Verification>() {
        if !probe.armed.load(Ordering::Acquire) {
            *lock(&probe.session) = session.clone();
        }
        if *lock(&probe.session) != *session {
            return;
        }
        let mut observations = lock(&probe.observations);
        let sequence = observations.wait.map_or(1, |wait| wait.sequence + 1);
        observations.wait = Some(WaitObservation {
            sequence,
            retry_at: retry_at.map(tokio::time::Instant::into_std),
            session_generation: session.generation(),
        });
        drop(observations);
        probe.changed.notify_one();
    }
}

pub(super) fn on_wake(app: &tauri::AppHandle, session: &WatcherSession, work: bool) {
    let Some(probe) = app.try_state::<Verification>() else {
        return;
    };
    if *lock(&probe.session) != *session {
        return;
    }
    let mut observations = lock(&probe.observations);
    if let Some(wait) = observations.wait {
        if observations.wakes.len() < 64 {
            observations.wakes.push(WakeObservation {
                sequence: wait.sequence,
                at: Instant::now(),
                work,
            });
        }
    }
    drop(observations);
    probe.changed.notify_one();
}

pub(super) fn on_publication(
    app: &tauri::AppHandle,
    session: &WatcherSession,
    result: &DiskReconcileResult,
) {
    let Some(probe) = app.try_state::<Verification>() else {
        return;
    };
    if *lock(&probe.session) != *session {
        return;
    }
    let state = app.state::<DiskReconcileState>();
    let coordinator = app.state::<MutationCoordinator>();
    let boundary = PublicationBoundary {
        revision: result.reconcile_revision,
        game_lease_available: state.try_game_guard(&result.game_id).is_some(),
        operation_lease_available: coordinator
            .inner_lock()
            .try_acquire_for_reconcile()
            .is_some(),
        runtime_generation: None,
    };
    let mut observations = lock(&probe.observations);
    if observations.boundaries.len() < 16 {
        observations.boundaries.push(boundary);
    }
    drop(observations);
    probe.changed.notify_one();
}

pub(super) fn on_runtime_enqueue(
    app: &tauri::AppHandle,
    game_id: &str,
    revision: u64,
    generation: u64,
) {
    let Some(probe) = app.try_state::<Verification>() else {
        return;
    };
    if probe.game_id != game_id {
        return;
    }
    if let Some(boundary) = lock(&probe.observations)
        .boundaries
        .iter_mut()
        .find(|boundary| boundary.revision == revision)
    {
        boundary.runtime_generation = Some(generation);
    }
    probe.changed.notify_one();
}

async fn wait_until<T>(
    probe: &Verification,
    description: &str,
    condition: impl Fn() -> Option<T>,
) -> Result<T, String> {
    tokio::time::timeout(CASE_TIMEOUT, async {
        loop {
            if let Some(value) = condition() {
                return value;
            }
            probe.changed.notified().await;
        }
    })
    .await
    .map_err(|_| {
        let observed = lock(&probe.observations);
        format!(
            "Timed out waiting for {description}; wait={:?}, wakes={}, results={}, runtime={}",
            observed.wait.map(|wait| (
                wait.sequence,
                wait.session_generation,
                wait.retry_at
                    .map(|at| at.saturating_duration_since(Instant::now()).as_millis())
            )),
            observed.wakes.len(),
            observed.results.len(),
            observed.runtime.len()
        )
    })
}

async fn wait_for_arm(app: &tauri::AppHandle) -> Result<(), String> {
    let probe = app.state::<Verification>();
    tokio::time::timeout(CASE_TIMEOUT, async {
        loop {
            probe.fixture.revalidate()?;
            match tokio::fs::read_to_string(probe.fixture.root.join(".watcher-verification-ready"))
                .await
            {
                Ok(marker) if marker == probe.fixture.marker => return Ok(()),
                Ok(_) => {
                    return Err("Verification arm marker does not match fixture ownership".into())
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    tokio::time::sleep(Duration::from_millis(50)).await
                }
                Err(error) => return Err(error.to_string()),
            }
        }
    })
    .await
    .map_err(|_| "Spec did not arm verification after UI seeding".to_string())?
}

async fn run_case(app: &tauri::AppHandle, name: &'static str) -> Result<CaseEvidence, String> {
    let probe = app.state::<Verification>();
    let session = lock(&probe.session).clone();
    let watcher = app.state::<WatcherState>();
    let state = app.state::<DiskReconcileState>();
    let coordinator = app.state::<MutationCoordinator>();
    probe.fixture.revalidate()?;
    let mods = probe.fixture.root.join("Mods");
    let before = wait_until(&probe, "Ready idle loop", || {
        let wait = lock(&probe.observations).wait?;
        (wait.session_generation == session.generation()
            && watcher.is_current_session(&session)
            && wait.retry_at.is_none()
            && state.ensure_core_ready_for_mutation(&probe.game_id).is_ok())
        .then(|| state.clean_authoritative_result(&probe.game_id, &mods, session.generation()))
        .flatten()
    })
    .await?;
    {
        let mut observations = lock(&probe.observations);
        observations.results.clear();
        observations.runtime.clear();
        observations.boundaries.clear();
        observations.wakes.clear();
    }
    let held_operation = coordinator
        .inner_lock()
        .acquire()
        .await
        .map_err(|error| error.to_string())?;
    let source_name = match name {
        "last_owner_release" => "ReleaseSource",
        "echo_expiry" => "ExpirySource",
        _ => "AbortSource",
    };
    let source = mods.join("ProbeObject").join(source_name);
    let destination = source.with_file_name(if name == "last_owner_release" {
        "RenamedRelease".into()
    } else {
        format!("DISABLED {source_name}")
    });
    let identity = crate::platform::fs::file_utils::filesystem_identity(&source)
        .ok_or("Missing source identity")?;
    let first_guard =
        (name == "last_owner_release").then(|| SuppressionGuard::new(&watcher.suppressor));
    let last_guard =
        (name == "last_owner_release").then(|| SuppressionGuard::new(&watcher.suppressor));
    let evidence = (name != "last_owner_release").then(|| {
        watcher.suppressor.expect_rename_echoes(
            &probe.game_id,
            &session,
            [ExpectedRenameEcho {
                old_path: source.clone(),
                new_path: destination.clone(),
                expected_identity: identity,
            }],
        )
    });
    let expiry = evidence
        .as_ref()
        .and_then(|evidence| watcher.suppressor.verification_echo_deadline(evidence));
    probe.fixture.revalidate()?;
    tokio::task::spawn_blocking(move || {
        crate::platform::fs::rename::rename_no_replace(&source, &destination)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())?;
    let trigger_wait = wait_until(
        &probe,
        "suppressed dirty demand and a parked receiver",
        || {
            if let Some(evidence) = evidence.as_ref() {
                if !watcher.suppressor.verification_echo_observed(evidence)
                    || watcher.suppressor.verification_echo_deadline(evidence)? <= Instant::now()
                {
                    return None;
                }
            } else {
                watcher.suppressor.pending_repair(&session)?;
            }
            let wait = lock(&probe.observations).wait?;
            let deadline = wait.retry_at?;
            let threshold = expiry
                .filter(|_| name == "echo_expiry")
                .unwrap_or_else(|| Instant::now() + Duration::from_secs(2));
            (deadline > threshold).then_some(wait)
        },
    )
    .await?;
    drop(first_guard);
    let first_owner_still_suppressed = (name == "last_owner_release").then(|| {
        watcher
            .suppressor
            .load(std::sync::atomic::Ordering::Acquire)
    });
    if first_owner_still_suppressed == Some(false) {
        return Err("First owner release ended blanket suppression prematurely".into());
    }
    if !lock(&probe.observations).results.is_empty() {
        return Err("Published while fixture operation was held".into());
    }
    drop(held_operation);
    let mut abort_before_expiry = None;
    match name {
        "last_owner_release" => drop(last_guard),
        "echo_abort" => {
            if Instant::now() >= expiry.ok_or("Missing live echo deadline")? {
                return Err("Abort echo expired before discard".into());
            }
            watcher
                .suppressor
                .discard_expected_rename_echoes(evidence.as_ref().ok_or("Missing abort evidence")?);
            if Instant::now() >= expiry.ok_or("Missing live echo deadline")? {
                return Err("Abort crossed echo expiry".into());
            }
            abort_before_expiry = Some(true);
        }
        _ => (), // The real expiry worker, not this fixture, releases the echo.
    }
    let accepted = wait_until(
        &probe,
        "accepted reconcile event and runtime publication",
        || {
            let observations = lock(&probe.observations);
            let result = observations.results.iter().find(|result| {
                result.status.applied() && result.reconcile_revision > before.reconcile_revision
            })?;
            let boundary = observations
                .boundaries
                .iter()
                .find(|boundary| boundary.revision == result.reconcile_revision)?;
            let runtime_generation = boundary.runtime_generation?;
            let runtime = observations.runtime.iter().find(|status| {
                status.generation == runtime_generation
                    && matches!(
                        status.phase,
                        RuntimeSyncPhase::Succeeded
                            | RuntimeSyncPhase::NeedsManualReload
                            | RuntimeSyncPhase::Failed
                    )
            })?;
            let wake = observations
                .wakes
                .iter()
                .find(|wake| wake.sequence >= trigger_wait.sequence && wake.work)?;
            let accepted_wait_sequence = observations.wait?.sequence;
            if accepted_wait_sequence <= trigger_wait.sequence {
                return None;
            }
            Some((
                result.clone(),
                *boundary,
                runtime.clone(),
                *wake,
                accepted_wait_sequence,
            ))
        },
    )
    .await?;
    let (result, boundary, runtime, wake, accepted_wait_sequence) = accepted;
    if runtime.phase == RuntimeSyncPhase::Failed {
        return Err(format!("Runtime publication failed: {:?}", runtime.message));
    }
    if name == "echo_expiry"
        && (wake.at < expiry.ok_or("Missing expiry deadline")?
            || watcher
                .suppressor
                .verification_echo_deadline(evidence.as_ref().ok_or("Missing expiry evidence")?)
                .is_some())
    {
        return Err("Wake did not come from the live echo's expiry".into());
    }
    let settings = app
        .state::<crate::modules::settings::api::config::ConfigService>()
        .get_settings();
    let diagnostics = keyviewer_runtime_diagnostics(&settings, &probe.game_id)
        .map_err(|error| error.to_string())?;
    let runtime_publication = diagnostics
        .publication
        .ok_or("Missing actual runtime publication diagnostics")?;
    if !matches!(
        runtime_publication,
        RuntimeSyncPublicationStatus::Published | RuntimeSyncPublicationStatus::Unchanged
    ) {
        return Err(format!(
            "Runtime did not publish a valid artifact: {runtime_publication:?}"
        ));
    }
    let authoritative = state
        .clean_authoritative_result(&probe.game_id, &mods, session.generation())
        .ok_or("No accepted clean authority after publication")?;
    if authoritative.reconcile_revision != result.reconcile_revision
        || watcher.suppressor.pending_repair(&session).is_some()
        || !boundary.game_lease_available
        || !boundary.operation_lease_available
        || wake.at >= trigger_wait.retry_at.ok_or("No retry deadline")?
    {
        return Err("Convergence/lease/wake ordering proof did not hold".into());
    }
    Ok(CaseEvidence {
        name,
        session_generation: session.generation(),
        before_revision: before.reconcile_revision,
        accepted_revision: authoritative.reconcile_revision,
        reconcile_event_revision: result.reconcile_revision,
        runtime_generation: runtime.generation,
        runtime_phase: runtime.phase,
        runtime_publication,
        trigger_wait_sequence: trigger_wait.sequence,
        accepted_wait_sequence,
        trigger_before_deadline: true,
        game_lease_available: boundary.game_lease_available,
        operation_lease_available: boundary.operation_lease_available,
        first_owner_still_suppressed,
        abort_before_expiry,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_gate_rejects_production_release_and_unowned_roots() {
        let nonce = uuid::Uuid::new_v4().to_string();
        assert!(validate_fixture("com.reynalivan.emmm", true, &nonce, Path::new("Mods")).is_err());
        assert!(validate_fixture(IDENTIFIER, false, &nonce, Path::new("Mods")).is_err());
        assert!(validate_fixture(IDENTIFIER, true, "../invalid", Path::new("Mods")).is_err());
        let fixture = tempfile::tempdir().unwrap();
        assert!(validate_fixture(IDENTIFIER, true, &nonce, fixture.path()).is_err());
    }

    #[test]
    fn fixture_revalidation_rejects_replaced_marker_and_identity() {
        let fixture = tempfile::tempdir().unwrap();
        let marker = uuid::Uuid::new_v4().to_string();
        std::fs::write(fixture.path().join(".emmm-e2e-fixture"), &marker).unwrap();
        let mut proof = FixtureOwnership {
            root: fixture.path().to_path_buf(),
            identity: crate::platform::fs::file_utils::filesystem_identity(fixture.path()).unwrap(),
            marker,
        };
        assert!(proof.revalidate().is_ok());
        proof.identity = "replaced-root".into();
        assert!(proof.revalidate().is_err());
        proof.identity =
            crate::platform::fs::file_utils::filesystem_identity(fixture.path()).unwrap();
        std::fs::write(
            fixture.path().join(".emmm-e2e-fixture"),
            uuid::Uuid::new_v4().to_string(),
        )
        .unwrap();
        assert!(proof.revalidate().is_err());
    }
}
