use std::sync::{Mutex, MutexGuard};

/// Locks `mutex`, recovering rather than panicking if it was poisoned.
///
/// Every `Mutex` behind this helper guards plain in-memory state — settings,
/// caches, hotkey registration — that stays structurally valid even when an
/// earlier holder panicked mid-update. Propagating the poison instead would
/// turn one transient panic into a permanently unusable subsystem, since every
/// later `lock().unwrap()` on the same mutex would panic too.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
