# Tauri local signing key discovery

## Decision

The `pnpm tauri` wrapper automatically loads the encrypted local updater key from
`%USERPROFILE%\.tauri\emmm.key` when `TAURI_SIGNING_PRIVATE_KEY` is not already
provided by the process environment.

The key contents are never logged or committed. CI continues to provide the key
through `TAURI_SIGNING_PRIVATE_KEY` and keeps its existing secret configuration.

## Validation

- Confirmed the local key exists outside the repository.
- The wrapper only reports the key path, never the key contents.
