# Fix INI editor and KeyViewer generation

## Context

Fix six approved audit findings: stale INI saves, lost pending edits, incorrect
catalog ownership, colliding panel names, damaged resource recovery and hidden
failed-save drafts.

## Changes

- Freeze document sections, field positions and hashes with the dirty draft.
  Settle successful files without replacing newer edits or other confirmations.
- Retire save confirmations on discard/session changes or a newer committed-read
  snapshot; retain validation and editing when a newer draft remains.
- Keep typed hash/resource/slot/draw identity through catalog and mod ownership.
- Pair sources by panel index; reject mismatched source counts before writing.
- Validate exact entrypoint and resource bytes. Publish a distinct immutable
  repair tree; preserve old trees until existing reload-gated cleanup is safe.
- Honor failed-save results. Add native integration and UI state regressions.

## Impacted Files

- Frontend: `src/widgets/mod-preview/PreviewPanel.tsx`,
  `components/{IniEditorSection.tsx,IniEditorSection.test.tsx}`,
  `hooks/{usePreviewRuntime.ts,usePreviewPanelState.ts,usePreviewPanelState.keyviewer-regression.test.tsx}`.
- KeyViewer: `src-tauri/src/modules/automation/application/keyviewer/matcher.rs`,
  `generator/keybind_text.rs`, `tests/matcher_tests.rs`,
  `tests/generator/{mod.rs,keybind_text_tests.rs}`.
- Publication/integration tests:
  `src-tauri/src/modules/system/application/app/post_apply.rs`.
- Local audit/fix artifacts under `.bug-hunter/`; this history file.

## Goal and Impact

INI drafts remain tied to their source, each panel receives its own controls,
and regeneration reconstructs damaged managed artifacts without replacing a live
resource tree. No dependency, migration, public IPC, commit, stash or worktree
change; unrelated shared-workspace edits preserved.

## Validation

- Regression-first checks reproduced both editor canaries and all three backend
  integration failures. Review corrections cover discard/rebase, partial retry,
  external supersession, navigation, newer invalid drafts and damaged entrypoints.
- Final frontend: 75 tests passed; production typecheck/build, ESLint and scoped
  Prettier passed. Rust: KeyViewer 65 and post-apply 49 passed/two manual benchmarks
  ignored; strict Clippy for lib/tests, scoped rustfmt and diff checks passed.
- Independent review findings resolved and spot-checked. Native tests use isolated
  temporary directories/databases; browser and in-game rendering were not run.

## Notes

Fresh query metadata follows TanStack Query's
[successful-fetch timestamp contract](https://github.com/TanStack/query/blob/main/packages/query-core/src/query.ts),
including unchanged structurally shared data. The local lock helper reports an
unsupported Windows directory-fsync error after its atomic file transition;
lease ownership/generation and final release were verified by read-back.
