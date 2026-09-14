# Mods Manager and reconcile follow-ups

Date: 2026-09-15

## Changes

- Internal metadata and thumbnail reconcile progress no longer presents the blocking “editing unavailable” banner. Watcher refreshes remain visible as non-blocking refreshes, and repeated runtime warnings are bounded and deduplicated.
- Watcher batches decide full versus scoped reconcile from normalized root coverage. Thumbnail fast-path is disabled for forced repair and repair completion still requires a completed full scan.
- Review matching keeps incomplete target comparisons as explicit review information. Proceed opens manual destination selection; GameBanana evidence remains optional, and incomplete comparisons never silently merge.
- Destination/canonical selectors render above dialogs with vertical, thumbnail-backed options and metadata chips. Duplicate exact-match member cards are keyboard- and pointer-selectable.
- Gallery navigation is bounded for large image sets and fullscreen preview is handled in-app with Escape and arrow-key navigation.
- Collection preview rows expose the path inline without an overlay, and active keybinding lists use a measured viewport for large sets. Dashboard INI harvesting runs off the async executor with one section scan per document.
- Theme template export uses a native save dialog, atomic write, success/error feedback, and file-manager reveal. Onboarding defaults anonymous diagnostics sharing to enabled. Browser tabs no longer render a globe behind a valid favicon.
- Object-list custom skin decoding now matches the persisted array written by classification.

## Validation

- TypeScript no-emit check and Vite production build passed.
- Full Vitest passed: 185 files, 948 tests (one intentionally skipped). Focused regression suites also cover GallerySection, DuplicateTable, ImportBatchWizard, CollectionTreeView, BrowserTabBar, settings, scanner, CanonicalObjectCombobox, file-watcher, committed mutation warning, and FolderGridSyncToast.
- Full Rust library tests passed: 1,120 tests, 8 intentionally ignored. This includes onboarding/object classification, theme template serialization, hotkey validation, dashboard keybindings, generation cleanup, watcher root coverage, thumbnail fast-path, and blanket-repair recovery.
- Vite production build passed. Full Rust formatting remains blocked by pre-existing unrelated formatting differences in browser/catalog modules; changed Rust files were checked and formatted individually.
