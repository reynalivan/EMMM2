# Epic 11 Implementation Plan (Settings)

## Scope

Align current implementation with:

- `.docs/epic11-settings.md`
- `.docs/trd.md`
- `.docs/.testcase/TC-Epic-11-Settings.md`

## Gap Summary

1. Logs tab is placeholder and does not read real log files.
2. Maintenance command does not clean orphaned `collection_items` rows and does not purge old trash entries.
3. Config migration path writes directly with `fs::write` (non-atomic path).
4. Frontend game form validation does not enforce invalid path constraints or duplicate path checks with disabled submit behavior.
5. PIN behavior tests do not yet cover lockout policy and no-pin flow explicitly.

## RED (Verification-first)

### Backend tests to add/update

1. `TC-11.4-01`, `DI-11.04`:
   - Verify 5 failed PIN attempts produce lockout (`locked_seconds_remaining > 0`).
2. `DI-11.03`:
   - Verify stored config contains hash and not plaintext PIN.
3. `EC-11.01`:
   - Verify corrupt config bytes fall back to defaults without panic.

### Frontend verification plan (manual)

1. `TC-11.3-01`:
   - Open Settings -> Logs tab.
   - Verify recent logs are rendered and level filters (INFO/WARN/ERROR) work.
   - Verify "Open Log Folder" triggers backend command.
2. `NC-11.1-01` and `NC-11.1-03`:
   - Open Add Game modal.
   - Enter invalid path chars (`?*<>`) and verify red input + save disabled.
   - Enter duplicate existing mod path and verify warning + save disabled.

## GREEN (Implementation)

1. Backend (`src-tauri`)
   - Make config migration save path atomic in `ConfigService::new`.
   - Extend `run_maintenance` to:
     - remove orphaned `collection_items` rows,
     - purge trash entries older than 30 days,
     - keep existing VACUUM + thumbnail prune.
   - Add log-read command reuse from `app_cmds` on frontend via existing commands.

2. Frontend (`src`)
   - Implement functional `LogsTab` with:
     - level filter,
     - refresh,
     - open log folder action.
   - Strengthen `GameFormModal` schema and UX:
     - invalid-path char check,
     - duplicate `mod_path` check,
     - disabled submit when invalid.

3. Tests
   - Add/extend backend tests for lockout, hash-not-plaintext, corrupt config fallback.

## REFACTOR

1. Keep command -> service separation unchanged.
2. Keep file sizes below project thresholds.
3. Reuse existing toast and invoke patterns.

## Completion Verification

1. Run diagnostics on modified TS/Rust files.
2. Run backend tests for config service.
3. Run frontend typecheck.
4. Run project build.
