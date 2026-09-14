# Phase A2 and B FSD Restructuring

### Context
Continuing the FSD restructuring to align with \docs/relocation.md\, completely dropping backward-compatible layers as requested by the user.

### Changes
- Migrated \src-tauri/src/commands/\ implementations directly into \modules/.../adapters/inbound/tauri.rs\.
- Removed \src-tauri/src/commands/\ proxy directories.
- Moved \src/core/lib\ to \src/shared/lib\ and completely removed \src/core/\.
- Configured TS and Vite to support \@/\ paths (baseUrl and paths).
- Repointed all TypeScript files to use \@/\ FSD aliases to resolve over 400 broken relative imports caused by prior directory restructuring.
- Removed \src/stores\ proxy directory entirely in favor of \src/app/store/\.

### Impacted Files
- \src-tauri/src/commands/*\ (removed)
- \src-tauri/src/modules/*/adapters/inbound/tauri.rs\ (modified/added)
- \src-tauri/src/lib.rs\ (modified)
- \src/core/*\ (removed/moved)
- \src/shared/lib/*\ (added/modified)
- \src/stores/*\ (removed)
- \	sconfig.json\, \ite.config.ts\ (modified)
- 120+ Frontend \.ts/\.tsx\ files (modified imports)

### Goal
Enforce direct dependency imports without backward compatibility proxies, while resolving cascading import breakages from directory moves.

### Impact
- \cargo test\ passes 100%.
- Frontend \	sc\ builds successfully without broken module resolution.
- All code correctly adheres to the FSD boundary structure defined in the relocation blueprint.

### Notes
- Extensive alias path rewriting (\@/...\) was required because previous moving of \src/features/*\ to \src/pages/*\ broke relative paths.
