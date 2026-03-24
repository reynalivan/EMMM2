# Unsaved Collection Initialization

## Context
When setting up the app for the first time or entering an empty corridor, the collections workspace and context dropdown were completely empty. The user requested that the "Unsaved" collection be created from the beginning so the collection state is never entirely empty, ensuring at least an "Unsaved" preset exists.

## Changes
- Modified `corridor_service.rs` to auto-initialize the `Unsaved` collection by calling `handle_dirty_state` if no collections exist in the active corridor.
- Modified `collection_service.rs` to include `is_unsaved = 1` collections when returning lists to the frontend, removing the bypass that dropped the Unsaved collection.
- Removed the nonexistent `kind` property from `CollectionSummary` in TypeScript.
- Updated `CollectionList.tsx` to include unsaved collections in the main workspace list so they are synced with the user's active state natively, while still excluding `is_undo_target` records. 
  - Rendered unique badging and disabled direct inline renaming for the Unsaved preset.
  - Replaced the DB `unsaved_...` name with the localized 'Unsaved Preset' explicitly.
  - Active Normal collections no longer show 'Apply' as it is already active.
  - The 'Delete' button is now completely hidden for Unsaved collections to prevent accidental deletion of the dirty state context.

## Impacted Files
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src/types/collection.ts` (modified)
- `src/features/collections/components/CollectionList.tsx` (modified)

## Goal
To guarantee that the Unsaved collection is always initialized upon the first entry into any corridor, providing a baseline collection state.

## Impact
- Context Controls Dropdown will always display at least the "Unsaved *" target natively.
- No longer showing "No collections" when first launching if nothing was explicitly saved.
