use super::TelemetryStore;
use crate::shared::sync::lock;
use std::sync::{Arc, Mutex};
use tokio::sync::MutexGuard;

pub(super) struct ConsentGate {
    state: Mutex<ConsentState>,
    pub(super) writer: tokio::sync::Mutex<()>,
}

struct ConsentState {
    enabled: bool,
    epoch: Arc<()>,
}

impl ConsentGate {
    pub(super) fn new(enabled: bool) -> Self {
        Self {
            state: Mutex::new(ConsentState {
                enabled,
                epoch: Arc::new(()),
            }),
            writer: tokio::sync::Mutex::new(()),
        }
    }

    pub(super) fn epoch(&self) -> Arc<()> {
        Arc::clone(&lock(&self.state).epoch)
    }

    pub(super) fn allows(&self, epoch: &Arc<()>, require_enabled: bool) -> bool {
        let state = lock(&self.state);
        Arc::ptr_eq(epoch, &state.epoch) && (!require_enabled || state.enabled)
    }

    pub(super) fn revoke(&self) {
        let mut state = lock(&self.state);
        state.enabled = false;
        state.epoch = Arc::new(());
    }

    fn enabled(&self) -> bool {
        lock(&self.state).enabled
    }

    fn set_enabled(&self, enabled: bool) {
        let mut state = lock(&self.state);
        if state.enabled != enabled {
            state.epoch = Arc::new(());
            state.enabled = enabled;
        }
    }
}

/// Serializes the consent setting's commit with telemetry persistence. A failed
/// or cancelled setting change restores admission without reviving old producers.
pub struct ConsentChange<'a> {
    store: &'a TelemetryStore,
    _writer: MutexGuard<'a, ()>,
    previous_enabled: bool,
    committed: bool,
}

impl TelemetryStore {
    /// Capture before asynchronous work; clones retain this original epoch.
    pub fn capture(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            consent: Arc::clone(&self.consent),
            epoch: Some(self.epoch.clone().unwrap_or_else(|| self.consent.epoch())),
        }
    }

    pub fn is_current(&self) -> bool {
        self.consent.allows(&self.captured_epoch(), true)
    }

    pub(super) fn captured_epoch(&self) -> Arc<()> {
        self.epoch.clone().unwrap_or_else(|| self.consent.epoch())
    }

    pub(super) async fn write_guard(&self, require_enabled: bool) -> Option<MutexGuard<'_, ()>> {
        let epoch = self.captured_epoch();
        let writer = self.consent.writer.lock().await;
        self.consent
            .allows(&epoch, require_enabled)
            .then_some(writer)
    }

    pub async fn begin_consent_change(&self, enabled: bool) -> ConsentChange<'_> {
        let writer = self.consent.writer.lock().await;
        let previous_enabled = self.consent.enabled();
        if !enabled {
            self.consent.revoke();
        }
        ConsentChange {
            store: self,
            _writer: writer,
            previous_enabled,
            committed: false,
        }
    }
}

impl ConsentChange<'_> {
    pub async fn purge(&self) -> Result<(), super::TelemetryError> {
        self.store.purge_unchecked().await
    }

    pub fn commit(mut self, enabled: bool) {
        self.store.consent.set_enabled(enabled);
        self.committed = true;
    }
}

impl Drop for ConsentChange<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.store.consent.set_enabled(self.previous_enabled);
        }
    }
}
