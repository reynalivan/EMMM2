# Sync unsaved corridor labels to switch dialog

## Context

Unsaved collection naming was already unified in list/topbar/preview, but the safe-mode switch dialog still rendered raw corridor state names. That left unsaved switch previews inconsistent and unable to distinguish SAFE vs UNSAFE unsaved states.

## Changes

- Expanded the shared corridor label helper to resolve corridor-aware unsaved names:
  - `Unsaved SAFE Preset`
  - `Unsaved UNSAFE Preset`
- Updated topbar, collection list, and collection preview to use the corridor-aware label helper.
- Extended switch preview payloads with unsaved/mode metadata for both leaving and target states.
- Updated the safe-mode switch confirm modal to render unsaved labels from shared logic instead of raw state names.
- Added locale keys for safe/unsafe unsaved labels in English, Indonesian, and Chinese.
- Added focused frontend tests for helper logic, topbar, collection list, and switch dialog rendering.
- Added backend test coverage for switch preview unsaved metadata.

## Impacted Files

- `src/lib/corridorLabels.ts` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)
- `src/features/collections/components/CollectionList.tsx` (modified)
- `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
- `src/types/collection.ts` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.test.tsx` (added)
- `src/components/layout/top-bar/ContextControls.test.tsx` (modified)
- `src/features/collections/components/CollectionList.test.tsx` (modified)
- `src/lib/corridorLabels.test.ts` (modified)
- `src/locales/en/layout.json` (modified)
- `src/locales/id/layout.json` (modified)
- `src/locales/zh/layout.json` (modified)
- `src-tauri/src/domain/corridor.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/services/corridor_constants.rs` (modified)

## Goal

Every collections/corridor surface now follows the same unsaved naming policy, including the switch preview dialog, and safe vs unsafe unsaved states are visually distinct.

## Impact

- Raw timestamp names no longer leak into switch previews for unsaved states.
- SAFE and UNSAFE unsaved states are distinguishable without relying on context.
- No storage schema or DB migration changes were introduced.

## Notes

- Frontend typecheck and focused Vitest suites passed.
- Rust test binaries still only verified to compile in this environment; running them remains blocked by `STATUS_ENTRYPOINT_NOT_FOUND`.
