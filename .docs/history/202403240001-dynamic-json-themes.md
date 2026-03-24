## Dynamic JSON Theme System Implementation

### Context

The application needed a way for users to create, import, and share custom themes beyond the built-in dark/light options. This allows for community-driven customization and high contrast accessibility options.

### Changes

- **Backend**:
  - Implemented `theme_cmds.rs` with `list_custom_themes`, `load_custom_theme`, `save_custom_theme`, and `delete_custom_theme`.
  - Added automatic `themes` directory initialization in `lib.rs`.
- **Frontend State**:
  - Created `useCustomThemes` hook for managing user-defined themes via IPC.
  - Updated `themeOptions.ts` to support arbitrary theme IDs.
- **Dynamic Injection**:
  - Implemented `DynamicThemeInjector.tsx` to inject runtime-resolved CSS variables into the document head.
- **UI & Settings**:
  - Added "Import Theme" (file picker) to Settings -> General.
  - Added "Export" and "Remove" actions for active custom themes.
  - Enhanced theme dropdown to group Built-in vs Custom themes.

### Impacted Files

- `src-tauri/src/commands/app/theme_cmds.rs` (added)
- `src-tauri/src/commands/app/mod.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src/lib/bindings.ts` (modified)
- `src/features/settings/theme/DynamicThemeInjector.tsx` (added)
- `src/features/settings/theme/useCustomThemes.ts` (added)
- `src/features/settings/theme/themeOptions.ts` (modified)
- `src/features/settings/tabs/GeneralTab.tsx` (modified)
- `src/App.tsx` (modified)

### Goal

Users can now fully customize the application's appearance by importing JSON theme files, which are applied dynamically at runtime without a restart.

### Impact

- **Performance**: Negligible impact; themes are loaded once on startup or when changed.
- **Persistence**: Custom themes are stored in the app data directory and survive updates/restarts.
- **Fallback**: Automatically reverts to 'onyx' (dark) if a custom theme is deleted while active.
