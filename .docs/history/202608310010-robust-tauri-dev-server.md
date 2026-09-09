# Robust Tauri Development Server Startup

## Context

Running `pnpm tauri dev` could fail with `Port 1420 is already in use` when an existing EMMM Vite child was still alive or when two development sessions started close together. On Windows, starting the Vite child through `pnpm.cmd` could also fail with `spawn EINVAL` under the workspace's Node runtime.

## Changes

- Route Tauri development through a dedicated `dev:tauri` launcher.
- Reuse a healthy EMMM Vite server already listening on port `1420`.
- Serialize startup per workspace with a temporary lock and re-check the port after acquiring it.
- Detect unrelated services on port `1420` and return an actionable error instead of hiding the conflict.
- Start Windows Vite children through `cmd.exe` and clean up the complete child tree when the launcher exits.
- Validate the EMMM title and entry point before reusing an existing server.

## Impacted Files

- `scripts/tauri-dev-server.mjs` (added)
- `package.json` (modified)
- `src-tauri/tauri.conf.json` (modified)

## Goal

Make `pnpm tauri dev` resilient to stale Vite children, duplicate launches, and Windows process-spawn behavior while preserving Tauri's fixed development URL.

## Impact

Repeated launches no longer create a second Vite server when an EMMM server is already ready. Concurrent startups are serialized, stale lock files can be recovered when their owner process is gone, and normal shutdown releases the child process tree and temporary lock.

## Validation

- `pnpm run dev:tauri` started Vite successfully from a free port.
- A second `pnpm run dev:tauri` reused the running EMMM server with exit code 0.
- Stopping the launcher released port `1420` and removed the temporary lock.
- Node syntax check, ESLint, Prettier, TypeScript, i18n lint, 15 focused tests, and production build passed.
