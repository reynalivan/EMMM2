# Audit follow-up fixes

The user authorized all nine findings and the verified cleanup candidates.
Preserve unrelated workspace changes, persisted migrations, journals and
filesystem ownership guards. No worktree, commit or dependency change.

## Scope

1. Route native drops to their owning surface, derive object-relative import
   subpaths, and dispose late asynchronous listener registrations.
2. Keep locked randomizer objects out of selection and mutation payloads;
   settle invalidated review requests and retain current-session guards.
3. Cancel stale collection reads before the existing committed refresh.
4. Persist ordinary staging failures as retryable item/batch failures; preserve
   cancellation settlement and wait-before-cleanup while making pack copying
   and root discovery cancellation-aware.
5. Resolve Windows disk space using equivalent physical path representations.
6. Move relevant legacy archive assertions to active staging/import contracts,
   then delete the unused extraction pipeline and its exclusive helpers.
7. Remove four unreferenced frontend barrels, an unused dialog helper,
   test-only unsupported-path helper and six unused wrapper re-exports.

## Verification

- Add failing regression tests first for changed behavior, then run targeted
  frontend/Rust suites. Archive coverage must survive pipeline deletion.
- Check source references, app/E2E types, scoped lint/architecture/format,
  Rust formatting and strict Clippy; build the frontend and private native app.
- Use the existing private WDIO/WebView2 harness for affected real import,
  randomizer and collection flows; never substitute browser demo evidence.
- Review the final scoped diff, update the backend architecture guide for changed
  ingestion flow, and record actual results and limitations in history.
