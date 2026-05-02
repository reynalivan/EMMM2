# 2026-03-25 20:50 - Remove CollectionTreeView legacy members fallback

## Summary

- Removed `mapLegacyNodes` and the `members` fallback from `CollectionTreeView`.
- Confirmed all live callers already pass `nodes`, so the legacy compatibility path was dead code.
- Simplified the component to a single preview-tree input path.

## Validation

- `pnpm exec tsc --noEmit`
- `cargo check --manifest-path src-tauri/Cargo.toml`

## Notes

- `CollectionTreeView.test.tsx` still has one pre-existing i18n expectation mismatch (`Container` vs translation key output). This is a test debt, not a runtime regression from removing `mapLegacyNodes`.
