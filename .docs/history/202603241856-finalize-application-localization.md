# finalize application localization (preview, onboarding)

## Context

Refactoring remaining hardcoded UI strings in the `preview`, `onboarding`, and `welcome` modules to ensure full multi-language support (English, Indonesian, Chinese).

## Changes

- Registered `preview` namespace in `i18n.ts`.
- Localized `PreviewPanel.tsx` and all sub-components (`MetadataSection`, `GallerySection`, `IniEditorSection`).
- Localized `PreviewPanelModals.tsx` and `AdvancedKeybindModal.tsx`.
- Localized `WelcomeScreen.tsx`, `ManualSetupForm.tsx`, and `AutoDetectResult.tsx`.
- Replaced hardcoded strings with `t()` calls using `preview`, `welcome`, `onboarding`, and `common` namespaces.
- Fixed React Compiler memoization conflicts and TypeScript type errors in the modified components.

## Impacted Files

- `src/lib/i18n.ts` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/preview/components/MetadataSection.tsx` (modified)
- `src/features/preview/components/GallerySection.tsx` (modified)
- `src/features/preview/components/IniEditorSection.tsx` (modified)
- `src/features/preview/components/PreviewPanelModals.tsx` (modified)
- `src/features/preview/components/AdvancedKeybindModal.tsx` (modified)
- `src/features/onboarding/WelcomeScreen.tsx` (modified)
- `src/features/onboarding/ManualSetupForm.tsx` (modified)
- `src/features/onboarding/AutoDetectResult.tsx` (modified)

## Goal

The system now achieves full internationalization for the preview and onboarding modules, providing a consistent multi-language experience for all users.

## Impact

- Seamless language transitions for the main preview functionality and first-run experience.
- Improved code quality through type fixes and lint resolution.

## Notes

- Standardized the use of `useTranslation` across all modules.
- Ensured consistent behavior of modals and form validation after localization.
