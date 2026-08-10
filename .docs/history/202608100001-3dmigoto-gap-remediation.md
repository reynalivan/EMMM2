# 3DMigoto gap remediation

## Context

The runtime knowledge audit found mismatches between EMMM, upstream 3DMigoto package behavior, INI persistence, KeyViewer artifacts, conflict evidence, and switch lifecycle reporting.

## Changes

- Unified recursive INI/runtime-disabled discovery and lossless encoding/newline handling.
- Added stale-safe, recoverable INI writes with source fingerprints and backup rotation.
- Replaced legacy reload/API assumptions with real Hunting bindings and package text APIs.
- Made KeyViewer matching deterministic, sentinel-safe, staged, and recoverable.
- Expanded archive classification and added typed conflict evidence, certainty, and ShaderFixes replacement detection.
- Changed conflicting default hotkeys, surfaced reserved-key collisions, reconciled incomplete rollback, and labeled app mutations as reload-required.

## Impacted Files

Backend:

- `src-tauri/src/common/{normalizer.rs,tests/normalizer_tests.rs}` (modified)
- `src-tauri/src/commands/app/{hotkey_cmds.rs,settings_cmds.rs}` (modified)
- `src-tauri/src/commands/mods/{mod_bulk_cmds.rs,preview_cmds.rs}` (modified)
- `src-tauri/src/pipeline/{apply_pipeline.rs,steps/batch_rename.rs}` (modified)
- `src-tauri/src/repo/{game_repo.rs,mod_repo/listing.rs}` (modified)
- `src-tauri/src/repo/mod_repo/tests/listing_tests.rs` (added)
- `src-tauri/src/services/app/post_apply.rs` (modified)
- `src-tauri/src/services/hotkeys/{mod.rs,reload.rs,tests/hotkey_tests.rs}` (modified)
- `src-tauri/src/services/ini/{document.rs,mod.rs,write.rs,tests/document_tests.rs,tests/write_tests.rs}` (modified)
- `src-tauri/src/services/ini/encoding.rs` (added)
- `src-tauri/src/services/keyviewer/{harvester.rs,matcher.rs}` (modified)
- `src-tauri/src/services/keyviewer/generator/{atomic.rs,ini.rs,keybind_text.rs,mod.rs,reload_key.rs}` (modified)
- `src-tauri/src/services/keyviewer/tests/{harvester_tests.rs,matcher_tests.rs}` (modified)
- `src-tauri/src/services/keyviewer/tests/generator/{atomic_tests.rs,ini_tests.rs,keybind_text_tests.rs,mod.rs,reload_key_tests.rs}` (modified)
- `src-tauri/src/services/mods/{metadata.rs,preview_ops.rs}` (modified)
- `src-tauri/src/services/mods/archive/classify.rs` (modified)
- `src-tauri/src/services/mods/tests/{metadata_conflict_tests.rs,preview_ops_tests.rs}` (modified)
- `src-tauri/src/services/runtime_mutation_engine.rs` (modified)
- `src-tauri/src/services/tests/runtime_mutation_engine_tests.rs` (modified)
- `src-tauri/src/services/scanner/conflict/{hash_scan.rs,tests/conflict_tests.rs}` (modified)

Frontend and generated contract:

- `src/lib/{bindings.gen.ts,disabledPrefix.ts,disabledPrefix.test.ts,pathKey.ts,pathKey.test.ts}` (modified)
- `src/features/preview/components/IniEditorSection.test.tsx` (modified)
- `src/features/preview/hooks/{usePreviewData.ts,usePreviewData.test.ts,usePreviewPanelState.ts}` (modified)
- `src/features/preview/{previewPanelUtils.ts,previewPanelUtils.test.ts}` (modified)
- `src/features/conflict-report/ConflictModal.tsx` (modified)
- `src/features/settings/tabs/{HotkeyTab.tsx,HotkeyTab.test.ts,hotkeyConflicts.ts}` (added/modified)
- `src/features/workspace-runtime/actions/{useWorkspaceSwitchActions.ts,workspaceSwitchOps.ts,workspaceSwitchOps.test.ts}` (modified)
- `src/locales/{en,id,zh}/{objects.json,scanner.json,settings.json}` (modified)

Documentation and tracking:

- `.docs/3dmigoto_context_knowledge.md` (added/updated)
- `implementation_plan.md`, `tasks/plan.md`, `tasks/todo.md` (added)

## Goal

EMMM now preserves INI bytes safely, follows package/runtime loading rules, generates portable KeyViewer artifacts, reports evidence-backed conflicts, recovers from incomplete mutations, and never equates a disk change with an in-game reload.

## Impact

- Existing saved hotkeys remain unchanged; only new/reset defaults use `Ctrl+F6` and `Ctrl+F8`.
- Conflict DTOs gain evidence/kind/certainty fields while retaining existing hash/path fields.
- EFMI KeyViewer generation fails explicitly because no verified official text API was found.
- Full Rust test audit still reports the unrelated pre-existing DAL ratchet in `collection_service/crud.rs`; library tests and production builds pass.
