# Portal Folder Conflict Details

## Context

The folder-conflict item detail overlay was rendered inside the scrollable conflict list. When opened near the bottom, the manager footer clipped the overlay and the info trigger could also bubble into the keep-folder action.

## Changes

- Render folder details through a `document.body` portal with a top-layer z-index.
- Position the panel beside the trigger and move it above the trigger when there is not enough room below it.
- Replace the hover-only role button with an accessible click trigger that supports outside-click and Escape dismissal.
- Keep the existing translated detail content and prevent info interactions from selecting a different keep folder.
- Add regression coverage for top-layer rendering and keep-selection stability.

## Impacted Files

- `src/widgets/mod-explorer/modals/FolderConflictCandidateCard.tsx` (modified)
- `src/widgets/mod-explorer/modals/FolderConflictManager.test.tsx` (modified)

## Goal

Keep folder conflict details readable and interactive regardless of the item position or footer boundary.

## Impact

The detail panel is no longer clipped by modal scroll containers. It remains available until dismissed by clicking outside, pressing Escape, or toggling the info trigger.

## Notes

Validation completed with 15 focused conflict-manager tests, TypeScript, ESLint, Prettier, and production build.
