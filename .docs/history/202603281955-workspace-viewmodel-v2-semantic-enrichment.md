# Workspace ViewModel V2 Semantic Enrichment

## Context

`WorkspaceViewModel` sebelumnya masih membawa `FolderGridResponse` dan `ModFolder` yang relatif mentah, sehingga `FolderGrid` dan `PreviewPanel` masih menebak role node, chip type, inactive ancestry, dan title preview dari query/query-helper terpisah.

## Changes

- `WorkspaceViewModel` diperkaya dengan kontrak runtime baru:
  - `WorkspaceExplorer`
  - `WorkspaceExplorerNode`
  - `WorkspacePreview` summary
  - warning/type/display enums untuk node workspace
- Backend `workspace_service` sekarang memetakan semantic final untuk node explorer:
  - `node_kind`
  - `display_mode`
  - `type_chip`
  - `is_effectively_active`
  - `ancestor_disabled`
  - `inactive_reason`
  - `warning_state`
  - `primary_warning`
  - `can_navigate`
- Preview slice workspace sekarang mengirim descriptor final:
  - `display_title`
  - `display_subtitle`
  - `mod_info_summary`
  - `ini_summary`
  - `image_summary`
  - `warning_summary`
- `PreviewPanel` dan `usePreviewPanelState` dipindah ke summary workspace untuk title/status/metadata source; `useModInfo` tidak lagi jadi source semantic utama preview.
- `FolderCard` dan `FolderListRow` sekarang merender chip/status/navigation dari metadata workspace final, bukan inferensi `node_type` lokal.
- Cache patch di `useFolders` diperbarui supaya optimistic update tetap mempertahankan field semantic `WorkspaceExplorerNode`.
- Locale `grid` ditambah label `Flat Mod` dan badge corrupt agar chip/warning baru punya label konsisten.

## Impacted Files

- `src-tauri/src/domain/workspace.rs` (modified)
- `src-tauri/src/services/workspace_service.rs` (modified)
- `src/types/workspace.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)
- `src/features/preview/hooks/usePreviewPanelState.ts` (modified)
- `src/features/preview/hooks/usePreviewData.ts` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/locales/en/grid.json` (modified)
- `src/locales/id/grid.json` (modified)
- `src/locales/zh/grid.json` (modified)
- `src/features/folder-grid/FolderCard.test.tsx` (modified)
- `src/features/folder-grid/FolderListRow.test.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.test.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelState.test.ts` (modified)

## Goal

Layar `mods` sekarang punya semantic runtime yang lebih backend-first: grid dan preview merender dari contract workspace yang sama, sehingga title/status/chip/warning tidak drift antar panel.

## Impact

- Query overlap berkurang karena preview metadata dasar tidak lagi bergantung pada `useModInfo`.
- Renderer frontend menjadi lebih declarative untuk role/type/inactive state node explorer.
- Rust runtime test masih belum bisa dieksekusi penuh di environment ini karena `STATUS_ENTRYPOINT_NOT_FOUND`, tetapi compile contract test berhasil.

## Notes

- `selected_node` lama tetap dipertahankan sebagai bagian dari preview slice agar optimistic cache patch yang sudah ada tidak perlu ditulis ulang total pada fase ini.
