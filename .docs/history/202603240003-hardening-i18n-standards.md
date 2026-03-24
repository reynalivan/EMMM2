## Hardening i18n Standards

### Context

To ensure EMM2 remains 100% localized as more features are added, we needed to move from a manual audit process to an automated, policy-driven approach. This follows the successful extraction of all remaining hardcoded UI strings.

### Changes

- **AGENT.md & Rules**: Established the **Zero-Hardcode i18n Policy**. ALL user-facing strings (labels, placeholders, tooltips, etc.) must now be localized via `react-i18next`.
- **Workflows**: Added mandatory i18n verification steps to `new-feature.md` and `refactor.md` workflows.
- **Component Refactoring**: Finished localization for `ScanReviewModal.tsx`, `EditObjectModal.tsx`, `CreateObjectModal.tsx`, and `AdvancedKeybindModal.tsx`.
- **Localization Parity**: Updated `en`, `id`, and `zh` translation files to ensure full coverage of new UI keys.

### Impacted Files

- `AGENT.md` (modified)
- `.agent/rules/code_standards.md` (modified)
- `.agent/rules/ui_ux.md` (modified)
- `.agent/workflows/new-feature.md` (modified)
- `.agent/workflows/refactor.md` (modified)
- `src/features/object-list/ScanReviewModal.tsx` (modified)
- `src/features/object-list/EditObjectModal.tsx` (modified)
- `src/features/object-list/CreateObjectModal.tsx` (modified)
- `src/features/preview/components/AdvancedKeybindModal.tsx` (modified)
- `src/locales/*/preview.json` (modified)

### Goal

Prevent the re-introduction of hardcoded text literals by making localization a core axiom of the agent's execution logic.

### Impact

- Standardized multi-language maintenance.
- Atom-level localization enforcement.
- Professionalized developer workflows.
