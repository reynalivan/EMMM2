# Tauri Command Registration Checklist

Every command exposed to the frontend must be registered in all IPC layers in the same change. Missing the permission entry causes the runtime error `command not allowed`, even when the Rust command compiles.

## Required steps

1. Add `#[tauri::command]` and `#[specta::specta]` to the Rust command.
2. Register the command in the `collect_commands!` list in `src-tauri/src/lib.rs`. This list is used both by Tauri's invoke handler and Specta's binding export.
3. Add the exact command name to `commands.allow` in `src-tauri/permissions/app-commands.toml`.
4. Regenerate `src/lib/bindings.gen.ts` with `cargo test specta_tests::export_bindings` from `src-tauri`. Do not hand-write a duplicate wrapper in `bindings.ts`.
5. Call the generated typed command from the frontend and handle its structured `AppError`.

## Verification gate

- Run the relevant Rust test or `cargo check` to validate command registration.
- Run `cargo test every_registered_command_is_allowed_by_the_app_permission`; it compares the invoke registry with the permission allowlist and prevents the common `command not allowed` regression.
- Regenerate bindings and confirm the new command and DTO types exist in `src/lib/bindings.gen.ts`.
- Run the related frontend test with the generated command mocked.
- Exercise the command through the Tauri runtime or E2E test when available; a unit test that calls the Rust function directly cannot detect a missing permission.

When a command is renamed or removed, update the command list, permission allowlist, generated bindings, frontend callers, and mocks together. Keep command names identical across all layers.

## Troubleshooting `command not allowed`

Treat this toast as an IPC registration mismatch, not as a command implementation failure:

1. Copy the exact command name from the error. Check spelling first (for example, `get_folder_conflict_details`, not `get_folder_conlfict_details`).
2. Confirm that exact name exists in both `src-tauri/src/lib.rs` and `src-tauri/permissions/app-commands.toml`.
3. Regenerate bindings and replace handwritten `invoke("...")` calls with the generated typed command whenever possible.
4. Run `cargo test every_registered_command_is_allowed_by_the_app_permission` before considering the fix complete.

Do not fix this by broadening Tauri permissions or adding wildcards. Commands remain explicitly allowlisted so the desktop IPC surface stays reviewable.
