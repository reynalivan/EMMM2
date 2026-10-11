# Simplify KeyViewer and INI editor

## Context

Five approved cleanup items reduce duplicated state and invalid panel/source
pairings while preserving the six preceding bug fixes.

## Changes

- Extracted `useIniEditorState`; fields derive from frozen sections, with one
  per-file snapshot merge helper. Baseline values/raw lines remain independent.
- Coupled matches and bindings in `KeyViewerPanel`; removed ignored matcher
  counts/config and replaced identity tuples with `SentinelIdentity`.
- Moved artifact inspection/selection into `keyviewer_publication`; retained
  exact-byte checks, immutable repair trees, latest-wins gates and atomic rollback.
- Added a regression proving unconfirmed files cannot duplicate/replace frozen
  sections. Independent review found no remaining actionable issues.

## Impacted Files

- Frontend hooks: `src/widgets/mod-preview/hooks/usePreviewPanelState.ts`,
  `useIniEditorState.ts` (added), `iniEditorSnapshot.ts` (added),
  `iniEditorSnapshot.test.ts` (added).
- KeyViewer (`src-tauri/src/modules/automation/application/keyviewer/`):
  `matcher.rs`, `mod.rs`, `models.rs` (added), `generator/ini.rs`,
  `generator/keybind_text.rs`, `tests/matcher_tests.rs`, `tests/generator/mod.rs`,
  `tests/generator/ini_tests.rs`, `tests/generator/keybind_text_tests.rs`.
- App (`src-tauri/src/modules/system/application/app/`): `mod.rs`,
  `post_apply.rs`, `keyviewer_publication.rs` (added).
- Documentation/artifacts: `src-tauri/AGENT.md`, this history,
  `.bug-hunter/cleanup-implementation_plan.md`, `cleanup-validation.md` and
  recoverable `cleanup-baseline/` copies (added).

## Goal

Editor state has one owner; each generated observer carries its own bindings;
publication validation stays cohesive without weakening safety guards.

## Impact

No public IPC, dependency, migration or fingerprint-format change. Existing
unrelated workspace edits are preserved. No performance improvement is claimed.

## Validation

70 focused frontend tests and 114 Rust tests passed (two Rust tests ignored).
Another 14 related preview-data tests passed. Targeted lint/format, production
Clippy with warnings denied, TypeScript/Vite build and diff checks passed.
Test-inclusive Clippy remains blocked by unrelated warnings in `readonly_audit`
and `bulk_attributes_tests`. One fresh-build isolated native Windows INI E2E
passed: validation, canceled transition, discard and persisted save. Details and
earlier shared-workspace blockers are in the validation artifact.
