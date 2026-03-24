## Fix safe-mode switch preview runtime parity

### Context

- Switch preview could show stale `Unsaved Preset` on the leaving side even when the strict active collection in the topbar was already named.
- The preview query key did not track the current corridor runtime identity, so cached preview data could be reused after save/apply.

### Changes

- Safe-mode toggle now prefetches the strict current corridor runtime before opening the switch confirm modal and clears stale switch-preview cache for that corridor target.
- Switch preview query keys now include current corridor mode and current strict runtime token, so preview data is refreshed when the active collection/signature changes.
- Switch modal waits for the current runtime query before rendering preview content, avoiding false empty-state renders.
- Added backend parity tests to assert switch preview leaving state matches `get_corridor_runtime_snapshot()` for named and unsaved corridors.

### Impacted Files

- `src/features/collections/queryKeys.ts` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.test.tsx` (modified)
- `src/hooks/useSafeModeToggle.ts` (modified)
- `src-tauri/src/services/privacy/tests/privacy_service_tests.rs` (modified)

### Goal

- Safe-mode switch preview now follows the current strict active corridor state instead of reusing stale preview data.

### Impact

- Switching corridors now forces a fresh current-runtime read before the confirm modal opens.
- Preview cache churn increases slightly, but only for switch-modal opens.
- No public API or schema changes.

### Notes

- The backend parity tests lock leaving-side preview semantics to the same strict runtime snapshot used by the topbar.
