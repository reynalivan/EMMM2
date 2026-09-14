# Enable/disable boundary hardening

## Context

The earlier scoped-switch work removed unrelated folder-conflict interruptions. This follow-up implements the approved audit recommendations for predictable, user-friendly enable/disable behavior while keeping the filesystem as the source of truth and SQLite as its projection.

## Changes

- Bulk enable/disable and bulk delete reject the configured Mods root and containment escapes before mutation. Missing folders become per-item failures; canonical aliases run once; parent/child selections are rejected together.
- Bulk terminal progress and `BulkResult` now carry `cancelled`, `processed_count`, and `unprocessed_count`. The cancel flag starts only after the operation owns its mutation lock, and the UI reports the exact partial result.
- Prepared switches reject enabling a child under a disabled parent. The read model also hides Enable Only This in that state, so direct IPC and UI agree without silently enabling the parent.
- Sibling and duplicate queries exclude a mod by its stable database ID instead of caller-supplied path spelling. Normal, Enable Only This, organizer, and randomizer flows share this rule.
- Removed the obsolete direct workspace-switch executor and legacy Enable Only This service. Prepared switching is now the single path used by the command boundary.
- Randomized loadouts now use the same applied-reconcile requirement, rollback-reconcile protection, and committed sync warning settlement as workspace switches.
- Collection runtime counts and safety summaries use effective active paths, so a terminal row below a disabled ancestor is not counted as runtime-active. Snapshot membership remains unchanged.
- Workspace switch effects invalidate only the mod-health reports whose identity changed, including all paths returned by an exclusive switch.

## Data and source of truth

No migration or schema change was introduced. Folder renames remain the physical source of truth; reconciliation remains the sole writer for runtime path/status projection. `folder_path_key` continues to be the logical identity for storage lookup, while stable mod IDs select the exact row to exclude from sibling operations.

## Validation

- Frontend focused tests: 29 passed (`workspaceSwitchOps` and bulk toast messages).
- Frontend typecheck and production build passed (`npm run build`).
- Rust `cargo check --tests` passed with SQLx offline metadata after the scoped changes; one pre-existing dead-code warning remains outside this scope. A later re-run was blocked by unrelated incomplete work in `modules/mutation/journal.rs` (missing helper functions), which was not modified here.
- Added Rust regressions for root/missing toggle input, alias/nested selections, cancelled result totals, disabled-parent direct switches, and collection counts beneath disabled ancestors.
- Native Rust test execution compiled successfully but linking the test binary was blocked by a locally locked `src-tauri/target/debug/deps/emmm_lib-*.exe` file. No process was stopped or user data changed.

## Deferred performance work

The existing no-op runtime projection benchmark remains the measurement baseline. No speculative performance path was added because the correctness changes reduce duplicate work; a production-sized trace should justify any future optimization.
