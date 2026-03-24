# flow.md and trd.md Aligned to Typed IPC

### Context

`flow.md` (data flow architecture) and `trd.md` (TRD) still referenced legacy `invoke()` IPC patterns and raw `window.__TAURI_INTERNALS__` mocking after typed IPC migration completed.

### Changes

- `flow.md`: 4 code-style `invoke('...')` patterns replaced with `commands.*()` equivalents: ObjectList query, dirty-state handler, save-collection, apply-collection.
- `trd.md` §7.2: Mock strategy updated from `window.__TAURI_INTERNALS__` to `vi.mock('../../lib/bindings')`.
- `trd.md` §7.3: "IPC bridge" clarified to "typed IPC bridge".
- **Additional req files** (found during re-audit): `req-01`, `req-03`, `req-04` (10 patterns), `req-05` (4 patterns) — all updated.

### Impacted Files

- `.docs/flow.md` (modified)
- `.docs/trd.md` (modified)
- `.docs/requirements/req-01-app-bootstrap.md` (modified)
- `.docs/requirements/req-03-onboarding.md` (modified)
- `.docs/requirements/req-04-settings-management.md` (modified)
- `.docs/requirements/req-05-workspace-layout.md` (modified)

### Goal

Zero `invoke(` code patterns remain anywhere in `.docs/` (excluding history post-logs which naturally reference the migration).

### Impact

- Documentation-only changes; no code logic affected.
- Full-scope grep audit confirmed zero remaining `invoke(` in active docs.
