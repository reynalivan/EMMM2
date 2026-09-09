# Remove Duplicate Mod Inbox Header

## Context
The user noted that the Mod Inbox page had a duplicate header underneath the global application TopBar. The TopBar was also showing irrelevant actions (e.g. Collections dropdown, global play button) while viewing the Mod Inbox.

## Changes
- Updated `TopBar` to act as a contextual "Portal" Toolbar (Option 1).
- Center of `TopBar` now renders the active page title dynamically (e.g., "MOD MANAGER", "MOD INBOX").
- Right side of `TopBar` renders a portal target `#topbar-actions-portal`.
- Irrelevant global controls (`ContextControls`, `GlobalActions`) are hidden when `workspaceView === 'mod-inbox'`.
- Replaced the large duplicate header in `ModInboxChrome.tsx` with a React Portal that teleports its controls (Change Location, Open Inbox, Refresh, and Root Path) directly into the `TopBar`.

## Impacted Files
- `src/shared/ui/components/layout/top-bar/index.tsx` (modified)
- `src/pages/mod-inbox/ModInboxChrome.tsx` (modified)

## Goal
To eliminate double headers, save vertical screen space, and make the UI strictly contextual per page without rewriting the entire state tree.

## Impact
Cleaner, simpler UI. Global actions no longer leak into the Inbox view. No breaking changes.
- Removed the global Recycle Bin/Trash shortcut from GlobalActions.tsx as requested to keep the TopBar clean.
- Moved SafetyFilterControl (All/Safe/Unsafe) from FolderGridToolbar to the TopBar right-actions portal. Switched it to compact mode to save space.
