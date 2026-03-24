## Finalized Dynamic Theme Migration

### Context

Standardizing all application UI colors and backdrops to ensure perfect compatibility with light and dark themes, eliminating all remaining hardcoded color literals.

### Changes

- **Backdrop Standardization**: Refactored all modal and overlay backdrops to use `bg-overlay-mask` and `backdrop-blur-sm` semantic tokens.
- **Semantic Migration**: Replaced `bg-black/x`, `bg-white/x`, and `text-white` with theme-aware tokens like `base-content`, `base-200/300`, and `primary/secondary/accent` variants.
- **Component Polish**: Updated `MetadataSection`, `IniEditorSection`, and `AdvancedKeybindModal` with refined theme-aware micro-interactions.
- **Layout Repair**: Fixed `dialog` implementation in `PinModal` and `GameFormModal` for standard modal behavior.

### Impacted Files

- `src/App.css` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)
- `src/features/collections/components/SaveCollectionModal.tsx` (modified)
- `src/features/safe-mode/PinEntryModal.tsx` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
- `src/features/settings/modals/PinModal.tsx` (modified)
- `src/features/settings/modals/GameFormModal.tsx` (modified)
- `src/features/browser/components/BrowserPage.tsx` (modified)
- `src/features/scanner/components/ResolutionModal.tsx` (modified)
- `src/features/scanner/components/ArchiveModal.tsx` (modified)
- `src/features/conflict-report/ConflictModal.tsx` (modified)
- `src/features/preview/components/MetadataSection.tsx` (modified)
- `src/features/preview/components/PreviewPanelContextMenu.tsx` (modified)
- `src/features/preview/components/AdvancedKeybindModal.tsx` (modified)

### Goal

The system now fully supports hot-swappable light and dark themes with 100% visual consistency and improved accessibility across all modules.

### Impact

- Improved readability in Light Mode across all feature panels.
- Consistent modal interaction experience using standard backdrops.
- Reduced UI technical debt by consolidating theme logic in CSS tokens.
