use crate::shared::errors::ScannerError;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::{
    watch_mod_directory_with_runtime_config_and_observer, ModWatcher, WatchEventReceiver,
    WatchObservation, WatcherSession, WatcherSuppressor,
};

pub(crate) struct WatcherCandidate {
    session: WatcherSession,
    watcher: ModWatcher,
    receiver: WatchEventReceiver,
    root: PathBuf,
    runtime_config: Option<PathBuf>,
    admitted: Arc<AtomicBool>,
}

impl WatcherCandidate {
    pub(crate) fn session(&self) -> &WatcherSession {
        &self.session
    }

    pub(crate) fn matches_binding(&self, root: &Path, runtime_config: Option<&Path>) -> bool {
        self.root == root
            && !self.receiver.is_closed()
            && self.runtime_config.as_deref() == runtime_config
            && self.session.matches_current_binding(root, runtime_config)
    }

    /// The caller must hold its current intent/config/session publication guard.
    /// Acceptance enables observations but never publishes managed watcher state.
    pub(crate) fn accept(
        self,
        root: &Path,
        runtime_config: Option<&Path>,
    ) -> Result<(WatcherSession, ModWatcher, WatchEventReceiver), ScannerError> {
        if !self.matches_binding(root, runtime_config) {
            return Err(ScannerError::Validation(
                "Watcher registration no longer matches filesystem coverage; retry activation"
                    .to_string(),
            ));
        }
        self.admitted.store(true, Ordering::Release);
        Ok((self.session, self.watcher, self.receiver))
    }
}

pub(crate) async fn register_watcher_candidate(
    root: PathBuf,
    runtime_config: Option<PathBuf>,
    suppressor: Arc<WatcherSuppressor>,
    session: WatcherSession,
    observer: WatchObservation,
) -> Result<WatcherCandidate, ScannerError> {
    let admitted = Arc::new(AtomicBool::new(false));
    let guarded_observer = guard_candidate_observer(
        Arc::clone(&admitted),
        session.identity_cache_validity(),
        observer,
    );
    run_registration(move || {
        let (watcher, receiver) = watch_mod_directory_with_runtime_config_and_observer(
            &root,
            runtime_config.as_deref(),
            suppressor,
            session.clone(),
            Some(guarded_observer),
        )?;
        Ok(WatcherCandidate {
            session,
            watcher,
            receiver,
            root,
            runtime_config,
            admitted,
        })
    })
    .await
}

fn guard_candidate_observer(
    admitted: Arc<AtomicBool>,
    validity: Arc<AtomicBool>,
    observer: WatchObservation,
) -> WatchObservation {
    Arc::new(move |paths, kind, events_lost| {
        if admitted.load(Ordering::Acquire) {
            observer(paths, kind, events_lost);
        } else if events_lost {
            validity.store(false, Ordering::Release);
        }
    })
}

async fn run_registration<T: Send + 'static>(
    factory: impl FnOnce() -> Result<T, ScannerError> + Send + 'static,
) -> Result<T, ScannerError> {
    tokio::task::spawn_blocking(factory)
        .await
        .map_err(|error| {
            ScannerError::Io(format!(
                "watcher registration worker failed; retry activation: {error}"
            ))
        })?
}

#[cfg(test)]
mod tests {
    use super::super::WatcherState;
    use super::*;
    use crate::modules::reconciliation::api::disk_reconcile::orchestrator::DiskReconcileState;
    use std::time::Duration;

    #[test]
    fn lost_unadmitted_observation_revokes_only_candidate_coverage() {
        let temp = tempfile::tempdir().expect("fixture root");
        let root = temp.path().to_path_buf();
        let watcher_state = WatcherState::new();
        let active = watcher_state.begin_session(&root);
        let candidate = watcher_state.prepare_session(&root);
        let authority = Arc::new(DiskReconcileState::default());
        authority.begin_authority_session("candidate", &root, active.generation());
        let observer = guard_candidate_observer(
            Arc::new(AtomicBool::new(false)),
            candidate.identity_cache_validity(),
            authority_observer(Arc::clone(&authority), root, candidate.generation()),
        );
        observer(&[], None, true);
        assert!(!candidate.identity_cache_validity().load(Ordering::Acquire));
        assert!(active.identity_cache_validity().load(Ordering::Acquire));
        assert_eq!(
            authority.authority_event_generation("candidate", active.generation()),
            Some(0)
        );
        assert_eq!(
            authority.authority_event_generation("candidate", candidate.generation()),
            None
        );
    }

    fn authority_observer(
        state: Arc<DiskReconcileState>,
        root: PathBuf,
        generation: u64,
    ) -> WatchObservation {
        Arc::new(move |paths, _, events_lost| {
            state.observe_authority_event("candidate", generation, &root, paths, events_lost);
        })
    }

    #[tokio::test]
    async fn unadmitted_native_candidate_does_not_replace_authority_or_the_active_session() {
        let temp = tempfile::tempdir().expect("fixture root");
        let root = temp.path().to_path_buf();
        let watcher_state = WatcherState::new();
        let active = watcher_state.begin_session(&root);
        let session = watcher_state.prepare_session(&root);
        let authority = Arc::new(DiskReconcileState::default());
        authority.begin_authority_session("candidate", &root, active.generation());
        let observer =
            authority_observer(Arc::clone(&authority), root.clone(), session.generation());
        let mut candidate = register_watcher_candidate(
            root.clone(),
            None,
            Arc::clone(&watcher_state.suppressor),
            session.clone(),
            observer,
        )
        .await
        .expect("native candidate registration");
        std::fs::write(root.join("unadmitted.ini"), "[Constants]\n").expect("native event");
        tokio::time::timeout(Duration::from_secs(5), candidate.receiver.recv())
            .await
            .expect("native event deadline")
            .expect("native event delivered");
        assert_eq!(
            authority.authority_event_generation("candidate", active.generation()),
            Some(0)
        );
        assert_eq!(
            authority.authority_event_generation("candidate", session.generation()),
            None
        );
        assert!(watcher_state.is_current_session(&active));
        drop(candidate);
        assert_eq!(
            authority.authority_event_generation("candidate", active.generation()),
            Some(0)
        );
        assert!(watcher_state.is_current_session(&active));
    }

    #[tokio::test]
    async fn accepted_native_candidate_delivers_observations_after_caller_publication() {
        let temp = tempfile::tempdir().expect("fixture root");
        let root = temp.path().to_path_buf();
        let watcher_state = WatcherState::new();
        let session = watcher_state.prepare_session(&root);
        let authority = Arc::new(DiskReconcileState::default());
        let observer =
            authority_observer(Arc::clone(&authority), root.clone(), session.generation());
        let candidate = register_watcher_candidate(
            root.clone(),
            None,
            Arc::clone(&watcher_state.suppressor),
            session,
            observer,
        )
        .await
        .expect("native candidate registration");
        let (accepted, watcher, mut receiver) =
            candidate.accept(&root, None).expect("current binding");
        assert!(!watcher_state.is_current_session(&accepted));
        watcher_state.publish_session(&accepted);
        authority.begin_authority_session_with_coverage("candidate", &root, &accepted);
        std::fs::write(root.join("admitted.ini"), "[Constants]\n").expect("native event");
        tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .expect("native event deadline")
            .expect("native event delivered");
        assert!(authority
            .authority_event_generation("candidate", accepted.generation())
            .is_some_and(|generation| generation > 0));
        drop(watcher);
    }

    #[tokio::test]
    async fn replacement_root_cannot_admit_a_registered_candidate() {
        let temp = tempfile::tempdir().expect("fixture root");
        let root = temp.path().join("Mods");
        std::fs::create_dir(&root).expect("original root");
        let watcher_state = WatcherState::new();
        let session = watcher_state.prepare_session(&root);
        let candidate = register_watcher_candidate(
            root.clone(),
            None,
            Arc::clone(&watcher_state.suppressor),
            session.clone(),
            Arc::new(|_, _, _| {}),
        )
        .await
        .expect("native candidate registration");
        assert!(candidate.matches_binding(&root, None));
        assert_eq!(candidate.session().generation(), session.generation());
        std::fs::rename(&root, temp.path().join("OriginalMods")).expect("retain original identity");
        std::fs::create_dir(&root).expect("replacement root");
        assert!(!candidate.matches_binding(&root, None));
        assert!(candidate.accept(&root, None).is_err());
        assert!(!watcher_state.is_current_session(&session));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blocking_registration_keeps_the_async_executor_responsive() {
        let (notify, wait) = std::sync::mpsc::channel();
        let notifier = tokio::spawn(async move { notify.send(()) });
        let registered = run_registration(move || {
            wait.recv_timeout(Duration::from_secs(2)).map_err(|error| {
                ScannerError::Io(format!(
                    "async notifier could not run during registration: {error}"
                ))
            })?;
            Ok(())
        })
        .await;
        let notification = notifier.await.expect("notifier should finish");
        assert!(
            registered.is_ok(),
            "blocking registration must yield the async executor: {registered:?}"
        );
        assert!(notification.is_ok());
    }

    #[tokio::test]
    async fn canceled_registration_drops_the_late_native_resource_without_publication() {
        struct Completion(Option<tokio::sync::oneshot::Sender<()>>);
        impl Drop for Completion {
            fn drop(&mut self) {
                if let Some(done) = self.0.take() {
                    let _ = done.send(());
                }
            }
        }
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().to_path_buf();
        let state = Arc::new(WatcherState::new());
        let active = state.begin_session(&root);
        let candidate_session = state.prepare_session(&root);
        let suppressor = Arc::clone(&state.suppressor);
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let (completed, dropped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(run_registration(move || {
            let resource = super::super::watch_mod_directory(&root, suppressor, candidate_session)?;
            let completion = Completion(Some(completed));
            let _ = started.send(());
            blocked.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok((resource, completion))
        }));
        ready.await.unwrap();
        task.abort();
        match task.await {
            Err(error) => assert!(error.is_cancelled()),
            Ok(_) => panic!("registration completed before cancellation"),
        }
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), dropped)
            .await
            .unwrap()
            .unwrap();
        assert!(state.is_current_session(&active));
    }

    #[tokio::test]
    async fn candidate_rejects_a_configured_runtime_without_parent_identity() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().to_path_buf();
        let runtime = root.join("missing/d3dx.ini");
        let state = WatcherState::new();
        let session = state.prepare_session_with_runtime_config(&root, Some(&runtime));
        let candidate = register_watcher_candidate(
            root.clone(),
            Some(runtime.clone()),
            state.suppressor.clone(),
            session,
            Arc::new(|_, _, _| {}),
        )
        .await
        .unwrap();
        assert!(!candidate.matches_binding(&root, Some(&runtime)));
        assert!(candidate.accept(&root, Some(&runtime)).is_err());
    }

    #[tokio::test]
    async fn registration_worker_panic_returns_an_actionable_error() {
        let result = run_registration(|| -> Result<(), ScannerError> {
            panic!("injected registration worker panic");
        })
        .await;
        assert!(matches!(result, Err(ScannerError::Io(message))
            if message.contains("watcher registration") && message.contains("retry")));
    }
}
