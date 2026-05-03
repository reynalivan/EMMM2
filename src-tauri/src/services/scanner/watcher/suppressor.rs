use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct WatcherSuppressor {
    guard_depth: AtomicUsize,
    manual_depth: AtomicUsize,
}

impl WatcherSuppressor {
    pub fn new(suppressed: bool) -> Self {
        Self {
            guard_depth: AtomicUsize::new(0),
            manual_depth: AtomicUsize::new(if suppressed { 1 } else { 0 }),
        }
    }

    pub fn load(&self, ordering: Ordering) -> bool {
        self.guard_depth.load(ordering) + self.manual_depth.load(ordering) > 0
    }

    pub fn store(&self, suppressed: bool, ordering: Ordering) {
        if suppressed {
            self.manual_depth.fetch_add(1, ordering);
            return;
        }

        let _ = self
            .manual_depth
            .fetch_update(ordering, Ordering::Acquire, |current| {
                Some(current.saturating_sub(1))
            });
    }

    fn increment(&self) {
        self.guard_depth.fetch_add(1, Ordering::AcqRel);
    }

    fn decrement(&self) {
        let _ = self
            .guard_depth
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.saturating_sub(1))
            });
    }
}

pub struct SuppressionGuard {
    suppressor: Arc<WatcherSuppressor>,
}

impl SuppressionGuard {
    pub fn new(suppressor: &Arc<WatcherSuppressor>) -> Self {
        suppressor.increment();
        Self {
            suppressor: suppressor.clone(),
        }
    }
}

impl Drop for SuppressionGuard {
    fn drop(&mut self) {
        self.suppressor.decrement();
    }
}
