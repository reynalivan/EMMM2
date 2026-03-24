# Fix Preview is_enabled Filter

## Context

The Apply Collection modal and Mode Switch modal displayed ALL members (enabled + disabled) from the collection snapshot. The Collection Preview Panel already had the correct filter.

## Changes

- Added `.filter((m) => m.is_enabled)` before `buildGroupedCollectionMembers` in both `ApplyCollectionModal` and `ModeSwitchConfirmModal`
- Matches existing pattern in `CollectionPreviewPanel` which already filtered correctly

## Impacted Files

- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)

## Goal

All three preview surfaces (Collection Panel, Apply Modal, Mode Switch Modal) now consistently filter by `is_enabled` before grouping.

## Impact

- Disabled mods no longer appear in Apply Collection or Mode Switch previews
- No backend changes needed — the filter operates on frontendData
