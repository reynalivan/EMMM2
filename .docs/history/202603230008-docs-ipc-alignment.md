# Docs Aligned to Typed IPC (Requirements & Test Cases)

### Context

Requirements and test case documentation still referenced legacy `invoke('cmd')` IPC patterns after the codebase completed typed IPC migration.

### Changes

- All explicit code-style `invoke('command', {...})` patterns replaced with `commands.cmd({...})` across 17 requirement files and 3 test case files.
- Semantic prose ("is invoked", "invoked when") preserved — no meaning changes.
- `tc-36` updated to reference `commands.*()` instead of `invoke()` in success criteria and coverage notes.

### Impacted Files

**Requirements (17):**

- `req-07-object-list.md`, `req-08-smart-filters.md`, `req-09-object-schema.md`
- `req-12-folder-grid-ui.md`, `req-14-bulk-operations.md`, `req-15-foldergrid-interactions.md`
- `req-17-metadata-editor.md`, `req-18-ini-viewer.md`, `req-19-image-gallery.md`
- `req-25-scan-engine.md`, `req-27-sync-database.md`, `req-33-dashboard.md`
- `req-35-smart-randomizer.md`, `req-36-toast-error-handling.md`, `req-38-auto-organizer.md`
- `req-39-folder-collision.md`, `req-40-metadata-actions.md`

**Test Cases (3):**

- `tc-13-core-mod-ops.md`, `tc-36-toast-error-handling.md`, `tc-38-auto-organizer.md`

### Goal

All project documentation now consistently reflects the typed `commands` API, preventing regression to legacy invoke patterns in future development.

### Impact

- Documentation-only changes. No code logic affected.
- Agent sessions reading requirements/test cases will now generate correct typed IPC patterns.
