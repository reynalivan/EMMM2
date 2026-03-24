# Unified Thumbnail Pipeline Protocol

### Context

Thumbnail rendering was unreliable and broken on some Windows systems due to protocol mismatches and host normalization (e.g., `game=localhost` errors). This refactor unifies the system on the `emmm://` custom protocol.

### Changes

- **Backend Protocol**: Standardized `ThumbnailCache` and all thumbnail commands to return `emmm://` URIs instead of absolute paths or `http` URIs.
- **URI Stripping**: Fixed critical bug in `lib.rs` where WebView2's automatic host normalization (`localhost/`) caused PathGuard rejections.
- **Scanner Update**: `resolve_thumbnail` now generates `emmm://` URIs for Master DB assets, ensuring they load correctly after sync.
- **Frontend Utility**: Refactored `getFileUrl` to safely dispatch URIs and absolute paths, preventing broken image renders for relative strings.
- **Component Safety**: Added protocol guards to `ObjectRowItem` to ensure Master DB paths are correctly resolved as URIs.

### Impacted Files

- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/services/images/thumbnail_cache.rs` (modified)
- `src-tauri/src/services/scanner/sync/preview.rs` (modified)
- `src-tauri/src/commands/mods/mod_thumbnail_cmds.rs` (modified)
- `src/lib/utils.ts` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/preview/components/GallerySection.tsx` (modified)

### Goal

Restore 100% reliable thumbnail rendering across all platforms by using a single, robust protocol scheme.

### Impact

- Eliminates "Broken Image" placeholders in ObjectList and FolderGrid.
- Robust against Windows/WebView2 host prefixing.
- Simplified frontend asset resolution.
