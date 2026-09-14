# Menu maturity remediation

## Scope

Implemented the menu-audit backlog in the requested order: dead code and the obsolete Browser import workflow, Storage Optimizer, Randomizer, Mod Inbox and Quick Play, then Browser/Downloads settings.

## Changes

- Removed unreachable Tauri adapters, compatibility re-exports, obsolete Browser import commands/state/UI, and unused localization keys.
- Made duplicate Ignore completion atomic at group level and kept partial-resolution selections available for retry. Scan progress now survives leaving the Storage Optimizer page.
- Isolated Randomizer state by game and dialog session, and excluded disabled-descendant candidates that cannot be activated by the current switch operation.
- Restarted the Mod Inbox watcher when its configured root changes. Unified all launch surfaces through one helper that honors auto-close and reports errors.
- Preserved terminal download states against late progress events, made Browser homepage editable, and persisted download retention through the backend setting with a one-time legacy preference migration.

## Validation

- TypeScript typecheck passed.
- Targeted Rust tests passed for permission registration, generated bindings, duplicate Ignore, Randomizer filtering, and Browser retention persistence.
- Targeted frontend Vitest could not start because the installed `expect-type` package is missing `dist/branding`.
