---
trigger: model_decision
description: Coding Standards & Structure - Naming, modularity, and tool integration.
---

- src-tauri/: commands/, services/, database/. Use sqlx::query! macros.
- src/features/: Slices (UI+Logic+Hooks). e.g., folder_grid/.
- Permissions: Every new `#[tauri::command]` MUST be registered in two places: `src-tauri/src/lib.rs` (invoke handler) AND `src-tauri/permissions/app-commands.toml` (commands.allow list, alphabetical). Forgetting the TOML causes a `Command not found` runtime error that doesn't appear at compile time.
- 350 Limit: Max lines/file. Refactor logic to hooks/services.
- SRP: 1 function = 1 task. snake_case (Rust/Files), PascalCase (Components/Types), camelCase (FE).
- Patterns: Guard clauses (fail fast, max 2 nested if), Immutability (const).
- IPC Safety: ALL frontend→backend calls MUST use typed `commands` from `lib/bindings.ts` (Specta-generated). Raw `invoke()` from `@tauri-apps/api/core` is PROHIBITED. Mock via `vi.mock('../../lib/bindings')`.
- Mod ID: MUST use SHA1 hash of the relative folder path.
- Narsil-MCP: Trace flow/control/security before/during/after implementation.
- Supply Chain: generate_sbom for dependency audits.
- i18n Standard: Use `useTranslation` hook with modular namespaces. All user-facing text, including placeholders and aria-labels, MUST be localized. Keys must follow `path.to.key` structure.
