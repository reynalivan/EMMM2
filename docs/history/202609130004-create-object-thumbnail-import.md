# Create object thumbnail import

## Context

New objects could not receive a user thumbnail during creation, and subcategories were shown as unrestricted input for every category.

## Changes

- Added file, clipboard, and URL thumbnail sources to Create New Object; each is normalized and stored locally as `preview_custom.png` through a compact, low-copy control.
- Reused the existing thumbnail size validation, image decoding, atomic write, and cache invalidation path.
- Restricted URL thumbnails to HTTP(S), rejected local/private targets and redirects, limited downloads to 10 MB, and validated downloaded bytes as an image.
- Rendered subcategory only when the selected game schema provides allowed values.

## Impacted Files

- `src/widgets/object-sidebar/modals/CreateObjectModal.tsx` (modified)
- `src/widgets/object-sidebar/modals/CreateObjectModal.test.tsx` (modified)
- `src/shared/i18n/locales/en/objects.json` (modified)
- `src/shared/i18n/locales/id/objects.json` (modified)
- `src/shared/i18n/locales/zh/objects.json` (modified)
- `src-tauri/src/modules/catalog/domain/objects.rs` (modified)
- `src-tauri/src/modules/catalog/application/objects/mutate.rs` (modified)
- `src-tauri/src/modules/catalog/application/objects/tests/mutate_tests.rs` (modified)
- `src-tauri/src/modules/library/application/mods/preview_ops.rs` (modified)
- `src-tauri/src/modules/library/adapters/tauri/mod_thumbnail_cmds.rs` (modified)
- `src/shared/api/tauri/bindings.gen.ts` (regenerated)

## Goal

Create New Object can import a persistent local thumbnail from all requested sources while respecting schema-defined classification choices.

## Impact

Asset-pack thumbnails remain supported as the existing fallback. The full Rust suite is currently blocked by an unrelated compile error in `mod_health/service.rs`.
