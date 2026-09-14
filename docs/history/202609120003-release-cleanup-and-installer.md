# Release cleanup and installer

## Context

The release build reported dead-code warnings, and local frontend tooling could
silently use a pnpm version different from the lockfile's declared version.

## Changes

- Removed unused release-only Rust exports, repository helpers, task helpers,
  and stale sync types. Helpers needed only by tests remain behind `cfg(test)`;
  integration tests now import the application operations directly.
- Kept the live GameBanana API test explicitly ignored because it depends on
  mutable remote community data.
- Pinned project setup to Corepack-managed pnpm `10.24.0`, added Node version
  constraints, and allowed only the required native package build scripts.
- Rebuilt the Windows NSIS installer from the cleaned source.

## Validation

- `cargo rustc -p emmm --release --lib --crate-type cdylib -- -Dwarnings`
- `cargo test -p emmm` — 962 passed, 3 ignored
- `corepack pnpm exec vitest run src/features/import-batches/ImportBatchWizardHost.test.tsx src/features/import-batches/resume.test.ts` — 4 passed
- `corepack pnpm build`
- `corepack pnpm lint` — 0 errors; 50 pre-existing style and React Compiler warnings remain
- `corepack pnpm tauri build --bundles nsis`

## Artifact

- `src-tauri/target/release/bundle/nsis/EMMM_0.1.0_x64-setup.exe`
