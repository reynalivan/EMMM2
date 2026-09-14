# Collection runtime descriptor IPC handoff

## Context

Global collection controls only need bounded runtime status. The Collections page
still needs the full runtime snapshot for its tree and projected-state preview.

## Changes

- Registered `get_collection_runtime_descriptor` with Tauri and the app command
  permission allowlist.
- Regenerated the TypeScript bindings, including the disk-reconcile warning
  contract generated alongside this change.
- Added a descriptor-specific query key and hook, then migrated the top-bar
  collection control to it.
- Kept `useCollectionRuntime` as the full snapshot hook for `CollectionsPage`.

## Verification

- The command registry/permission audit passed, so the descriptor command is
  registered and allowed.
- Focused top-bar and collection hook tests passed.
- TypeScript initially reported three file-watcher test fixture errors
  caused by the concurrently generated required `pending_runtime_effects` field.

## Follow-up

- Updated the three disk-reconcile result fixture builders with empty pending
  effects and warnings. Their focused tests and the full TypeScript check pass.
