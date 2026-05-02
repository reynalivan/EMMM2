## [2026-03-24] - Splash Screen Implementation

### Title
Implementing Professional Tauri Splash Screen

### Context
The application lacked visual feedback during the initial 1-2 seconds of startup, which felt unpolished. The goal was to provide an instant, theme-aware splash screen.

### Changes
- Created static `public/splash.html` with CSS Aurora background and SVG logo.
- Configured Tauri for multi-window startup (`splashscreen` visible, `main` hidden).
- Added `close_splashscreen` Rust command to manage window transition.
- Integrated closure trigger in React `AppRouter` startup lifecycle.
- Resolved multiple unrelated Rust backend test suite errors (signature mismatches, duplicate fields) to restore build stability.
- Fixed malformed JSX in `Dashboard.tsx` charts.

### Impacted Files
- `src-tauri/tauri.conf.json` (modified)
- `src-tauri/src/commands/app/app_cmds.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src/App.tsx` (modified)
- `public/splash.html` (added)
- `src/features/dashboard/Dashboard.tsx` (modified)
- `src-tauri/src/commands/folder_grid/tests/listing_tests.rs` (modified)
- `src-tauri/src/services/explorer/tests/helpers_tests.rs` (modified)

### Goal
A professional startup experience with near-instant visual feedback.

### Impact
- Improved perceived performance on launch.
- Backend stability restored by fixing test suite errors.
- No breaking changes to existing features.
