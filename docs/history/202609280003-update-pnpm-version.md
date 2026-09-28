# Update project pnpm version

## Context

The project pinned pnpm 10.24.0, but the available toolchain uses pnpm 11.19.0 with Node 24.18.0, preventing project scripts from running through pnpm.

## Changes

- Align `packageManager` and `engines.pnpm` with pnpm 11.19.0.
- Align the setup script and README with the same project pin.
- Carry the existing dependency build-script allowlist into pnpm 11's `allowBuilds` configuration.

## Impacted Files

- `package.json` (modified)
- `setup.ps1` (modified)
- `README.md` (modified)
- `pnpm-workspace.yaml` (modified)
- `docs/history/202609280003-update-pnpm-version.md` (added)

## Goal

Project package commands run with the available pnpm toolchain.

## Impact

- Contributors and CI need pnpm 11.19.0 and a supported Node version.
- No application runtime behavior or dependency versions changed.
