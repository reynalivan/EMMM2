# Workspace UI consolidation

Consolidated the primary workspace pages around a shared page frame and restrained visual language.

## Changed

- Added `WorkspacePageFrame`, `WorkspacePageContent`, and `WorkspaceContextBar` for consistent wide and form page layouts.
- Applied the frame and compact context controls to Storage Optimizer, Collections, Settings, Downloads, and Mod Inbox.
- Reduced visual noise across workspace surfaces: neutral borders, one primary blue action color, and status colors only where they convey state or destructive intent.
- Kept Mods Manager and Discover purpose-built while aligning their toolbar, focus/touch affordances, list surfaces, and motion behavior.
- Replaced browser download overlay z-index literals with named workspace layers.

## Validation

- Targeted Vitest: 31 tests passed across the changed workspace UI suites.
- Targeted ESLint passed for the changed files.
- `npm run build` passed.

