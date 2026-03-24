# Fix Blank Thumbnails in ObjectList & FolderGrid

## Context

Semua asset image thumbnail di ObjectList dan FolderGrid muncul blank (tidak fungsional). Investigasi menemukan mismatch pada Tauri v2 protocol handler API dan masalah encoding path Windows di URI.

## Changes

- **Backend**: Update `lib.rs` agar compatible dengan Tauri v2 `UriSchemeContext`.
- **Backend**: Simplify URI format dari `{game_id}/{abs_path}` menjadi `thumbnails/{hash}.webp`.
- **Frontend**: Fix path construction di `ObjectRowItem` (gunakan `/` bukan `\`).
- **Frontend**: Update watcher hooks agar trigger `refetchType: 'active'` untuk thumbnail.
- **Backend**: Align `get_thumbnail` cmd dengan format URI baru.

## Impacted Files

- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/services/images/thumbnail_cache.rs` (modified)
- `src-tauri/src/commands/mods/mod_thumbnail_cmds.rs` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src-tauri/src/services/images/tests/thumbnail_cache_tests.rs` (modified)
- `src/hooks/useThumbnail.test.tsx` (modified)

## Goal

Restore functional thumbnail rendering across the entire app with stable, simplified URI protocol.

## Impact

- Thumbnail loading lebih stabil (bebas encoding issues).
- Performa lebih baik (less complexity di protocol handler).
- Real-time update thumbnail saat file di disk berubah (via watcher).
