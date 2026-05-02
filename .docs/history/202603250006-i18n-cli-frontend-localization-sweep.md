# Frontend i18n CLI and coverage sweep

## Context

Frontend masih punya key i18n yang belum tertutup penuh, warning editor untuk namespace locale, dan beberapa string FE masih hardcoded atau masih memakai `defaultValue` sebagai fallback produksi.

## Changes

- Menambahkan `i18next-cli` dan konfigurasi extractor/sync untuk struktur locale `src/locales/<lang>/<namespace>.json`.
- Menyetel editor i18n agar namespace nested key dikenali dengan benar.
- Melokalisasi sisa string FE di dashboard, folder grid, preview, object list, collections, conflict modal, scanner, top bar, downloads, settings, onboarding, dan safe mode.
- Menghapus fallback `defaultValue` user-facing di hooks/settings flow dan memindahkannya ke key locale eksplisit.
- Menambahkan key baru ke `en`, lalu menyinkronkan dan mengisi `id` serta `zh` untuk namespace yang disentuh dalam perubahan ini.

## Impacted Files

- Tooling:
  - `.vscode/settings.json` (modified)
  - `i18next.config.ts` (added)
  - `package.json` (modified)
  - `pnpm-lock.yaml` (modified)
- Frontend components/hooks:
  - `src/components/dialogs/FileInUseDialog.tsx` (modified)
  - `src/components/layout/top-bar/ContextControls.tsx` (modified)
  - `src/components/layout/top-bar/index.tsx` (modified)
  - `src/hooks/useSettings.ts` (modified)
  - `src/features/collections/components/ApplyCollectionModal.tsx` (modified)
  - `src/features/collections/components/CollectionModRow.tsx` (modified)
  - `src/features/collections/components/CollectionTreeView.tsx` (modified)
  - `src/features/collections/components/RecoveryDialog.tsx` (modified)
  - `src/features/conflict-report/ConflictModal.tsx` (modified)
  - `src/features/dashboard/Dashboard.tsx` (modified)
  - `src/features/downloads/DownloadsPage.tsx` (modified)
  - `src/features/folder-grid/ActiveModContextDialog.tsx` (modified)
  - `src/features/folder-grid/Breadcrumbs.tsx` (modified)
  - `src/features/folder-grid/ConflictResolveDialog.tsx` (modified)
  - `src/features/folder-grid/DragOverlay.tsx` (modified)
  - `src/features/folder-grid/DuplicateWarningModal.tsx` (modified)
  - `src/features/folder-grid/FolderCard.tsx` (modified)
  - `src/features/folder-grid/FolderListRow.tsx` (modified)
  - `src/features/folder-grid/IgnoreManagementModal.tsx` (modified)
  - `src/features/object-list/AutoSetupModal.tsx` (modified)
  - `src/features/object-list/BulkTagModal.tsx` (modified)
  - `src/features/object-list/DropConfirmModal.tsx` (modified)
  - `src/features/object-list/EditObjectTabThumbnail.tsx` (modified)
  - `src/features/object-list/FolderTooltip.tsx` (modified)
  - `src/features/object-list/useObjHandlersBulk.ts` (modified)
  - `src/features/onboarding/AutoDetectResult.tsx` (modified)
  - `src/features/preview/PreviewPanel.tsx` (modified)
  - `src/features/preview/components/AdvancedKeybindModal.tsx` (modified)
  - `src/features/preview/components/GallerySection.tsx` (modified)
  - `src/features/preview/components/PreviewPanelContextMenu.tsx` (modified)
  - `src/features/preview/components/PreviewPanelModals.tsx` (modified)
  - `src/features/preview/components/UnsavedIniChangesModal.tsx` (modified)
  - `src/features/randomizer/RandomizerModal.tsx` (modified)
  - `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
  - `src/features/scanner/components/ArchiveModal.tsx` (modified)
  - `src/features/scanner/components/useReviewTable.tsx` (modified)
  - `src/features/settings/tabs/GeneralTab.tsx` (modified)
  - `src/features/settings/tabs/HotkeyTab.tsx` (modified)
  - `src/features/settings/tabs/MaintenanceTab.tsx` (modified)
- Locale files:
  - `src/locales/en/browser.json` (modified)
  - `src/locales/en/collections.json` (modified)
  - `src/locales/en/common.json` (modified)
  - `src/locales/en/dashboard.json` (modified)
  - `src/locales/en/folder_grid.json` (modified)
  - `src/locales/en/grid.json` (modified)
  - `src/locales/en/layout.json` (modified)
  - `src/locales/en/objects.json` (modified)
  - `src/locales/en/preview.json` (modified)
  - `src/locales/en/safe_mode.json` (modified)
  - `src/locales/en/scanner.json` (modified)
  - `src/locales/en/settings.json` (modified)
  - `src/locales/id/browser.json` (modified)
  - `src/locales/id/collections.json` (modified)
  - `src/locales/id/common.json` (modified)
  - `src/locales/id/dashboard.json` (modified)
  - `src/locales/id/folder_grid.json` (modified)
  - `src/locales/id/grid.json` (modified)
  - `src/locales/id/layout.json` (modified)
  - `src/locales/id/objects.json` (modified)
  - `src/locales/id/preview.json` (modified)
  - `src/locales/id/safe_mode.json` (modified)
  - `src/locales/id/scanner.json` (modified)
  - `src/locales/id/settings.json` (modified)
  - `src/locales/zh/browser.json` (modified)
  - `src/locales/zh/collections.json` (modified)
  - `src/locales/zh/common.json` (modified)
  - `src/locales/zh/dashboard.json` (modified)
  - `src/locales/zh/folder_grid.json` (modified)
  - `src/locales/zh/grid.json` (modified)
  - `src/locales/zh/layout.json` (modified)
  - `src/locales/zh/objects.json` (modified)
  - `src/locales/zh/preview.json` (modified)
  - `src/locales/zh/safe_mode.json` (modified)
  - `src/locales/zh/scanner.json` (modified)
  - `src/locales/zh/settings.json` (modified)

## Goal

Frontend sekarang memakai key locale untuk copy yang disentuh perubahan ini, editor dapat membaca namespace locale repo, dan `i18next-cli` bisa dipakai untuk audit, lint, dan sync antar bahasa.

## Impact

- Warning hardcoded string dari `i18next-cli lint` turun menjadi bersih.
- Build frontend tetap lolos.
- `i18next-cli status` masih belum 100% karena masih ada gap translasi lama di namespace lain yang tidak menjadi bagian sweep ini.

## Notes

- `i18next-cli sync` mengisi struktur key baru ke `id` dan `zh`, tetapi value kosong tetap perlu diisi manual; perubahan ini sudah mengisi key baru yang diperkenalkan sweep ini.
