## Parent-Disabled Folder Inheritance

### Context
Folders inside a disabled parent (prefixed with `DISABLED `) should automatically be treated as disabled and locked, preventing inconsistent states where a subfolder is "enabled" but its parent container is inactive.

### Changes
- **Backend (listing.rs)**: Added `find_disabled_ancestor` to scan path segments for the `DISABLED ` prefix in $O(\text{depth})$. Injected this state into `FolderGridResponse`.
- **Backend (types.rs)**: Added `ancestor_disabled_by` field to `FolderGridResponse`.
- **Frontend (useFolderGrid.ts)**: Added logic to detect ancestor lock, expose it to UI, and guard `handleToggleEnabled` to prevent interaction when locked.
- **Frontend (FolderGridBanners.tsx)**: Added a compact, context-aware notice bar. Hides "Active Mod Directory" banner when enabled to reduce clutter.
- **Frontend (EnableParentDialog.tsx)**: New dialog with impact preview (which mods will activate vs. stay disabled).
- **Frontend (FolderCard.tsx)**: Added `LOCKED` badge and warning-colored toggle for inherited disabled states.
- **i18n**: Added keys for English, Indonesian, and Chinese.

### Impacted Files
- `src-tauri/src/services/explorer/types.rs` (modified)
- `src-tauri/src/services/explorer/listing.rs` (modified)
- `src-tauri/src/commands/folder_grid/listing.rs` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/FolderGridBanners.tsx` (modified)
- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/folder-grid/EnableParentDialog.tsx` (added)
- `src/locales/en/grid.json` (modified)
- `src/locales/id/grid.json` (modified)
- `src/locales/zh/grid.json` (modified)

### Goal
Establish a reliable, path-based inheritance system for mod folder states that is performant (No DB I/O for detection) and provides clear UI feedback.

### Impact
- Improves UI/UX transparency for nested mods.
- Prevents "phantom enabled" mods that don't actually load because their parent is disabled.
- Zero database migration required.
