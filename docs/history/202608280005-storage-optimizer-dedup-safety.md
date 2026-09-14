# Storage Optimizer Dedup Safety — 2026-08-28

## Outcome

Storage Optimizer now scans terminal disk roots as logical mod units, so DB rows for merged child/subvariants cannot become independent duplicate candidates. Partial texture hashes are candidate filters only; exact identity is recomputed with full BLAKE3 across every regular content file.

Destructive resolution is now fail-closed. A request must reference a pending persisted group for the active game, both canonical paths must be distinct members of that group, and Keep/Hardlink requires a 100% group. The resolver then recomputes both full path-aware manifests immediately before any filesystem mutation.

## Root causes corrected

- DB rows and disk walker results were both treated as candidates, duplicating root/child ownership and allowing case-different spellings of one Windows folder.
- 1 KB head/tail DDS samples could become final exact evidence.
- Similarity edges were unioned transitively while the group inherited the maximum pair score.
- Dedup INI parsing stopped after 200 lines, accepted comment-like headers, and did not type resource versus shader hashes.
- Resolver trusted caller-supplied group/path data and hardlink validation checked size without full content identity.
- Frontend refreshed the report when the background command returned instead of when the scan emitted `Finished`.
- Report state was process-local and not reliably scoped/persisted per game.

## Main changes

- Walker terminal roots are the authoritative candidate inventory; DB paths only enrich members through canonical path keys.
- Merged roots own nested and disabled child variants; dot-prefixed internal directories, symlinks, and standard OS noise are excluded consistently.
- Shift-JIS INIs use the shared decoder during folder classification.
- Large DDS files use sampling for phase-one scoring and full BLAKE3 before a 100% result.
- All regular files participate in exact manifests; non-exact relations remain pairwise and never gain destructive controls.
- Scan lifecycle has a distinct `Failed` event, report refresh occurs on `Finished`, and completed reports persist in existing dedup tables per game.
- Resolved/ignored rows are excluded when rebuilding a report, and the command reads SQLite rather than a stale in-memory copy.
- Hardlink replacement rechecks size and full hash per file, stages the original, restores on link/recycle failure, and skips the same noise policy as the scanner.

## Verification

- Frontend full suite: 142 files passed; 756 tests passed, 1 skipped.
- TypeScript: `pnpm exec tsc --noEmit` passed.
- Frontend production build: passed.
- i18n lint: passed.
- ESLint: 0 errors; 6 existing max-lines warnings.
- Rust focused dedup after authorization hardening: 40 passed. Classifier: 6 passed.
- Full Rust suite: 803 passed, 2 ignored, 2 unrelated characterization tests failed (`collection_service` pending recovery and `recovery_service` rollback expectations).
- `cargo check` and `cargo clippy --lib -- -D warnings`: passed.
- All-target Clippy is blocked by an unrelated `while_let_loop` lint in `disk_reconcile/orchestrator/tests.rs`.
- Scoped `rustfmt --check` and `git diff --check`: passed.
- Global `cargo fmt --check` remains blocked by formatting in unrelated collections/reconcile/watcher files.

## Remaining follow-up

- Add explicit typed relations (`ExactCopy`, `SharedAssets`, `RelatedVariant`, `RuntimeConflict`) instead of using confidence plus full revalidation as the safety boundary.
- Extend the shared lossless INI decoder/writer contract to UTF-16 LE/BE; current ownership decoding supports UTF-8 and Shift-JIS.
- Thread real per-folder progress and cancellation checks through snapshot/hash phases; remove the duplicate inventory pass in the command.
- Record bounded unreadable-file warnings and benchmark the 1k-file/10k-root targets.
- Smoke-test Recycle Bin and NTFS hardlinks on a copied Mods library, including cross-volume behavior and partial hardlink batches.
- Resolve the two unrelated Rust characterization failures before requiring a completely green repository-wide suite.
