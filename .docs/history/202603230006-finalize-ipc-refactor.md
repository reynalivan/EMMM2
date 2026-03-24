# 100% IPC Refactor Completion

### Context

Final audit and refactoring of remaining legacy `invoke` calls to achieve 100% end-to-end type safety for Tauri commands.

### Changes

- Migrated legacy `invoke` to typed `commands` for Collections, PIN security, and Preview data management.
- Resolved type mismatches in optional parameters (e.g., `recoveryCode` transitioned from `null` to `undefined`).
- Eliminated all raw IPC strings from the `src` directory.

### Impacted Files

- `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
- `src/features/collections/hooks/usePin.ts` (modified)
- `src/features/preview/hooks/usePreviewData.ts` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)

### Goal

Achieve 100% type safety for the React-Rust bridge, ensuring all communication is validated at compile time.

### Impact

- **Security**: Hardened PIN and corridor switching operations with robust typing.
- **Reliability**: Zero risk of runtime "Command not found" or signature mismatch errors for refactored modules.
- **Maintainability**: Standardized IPC patterns across the entire codebase.
