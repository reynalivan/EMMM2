---
trigger: model_decision
description: File Management Rule - When manipulating the filesystem (rename, move, delete), toggling objects, or dealing with watcher suppression.
---

# 📁 File Management & Movement Standards

> **Context:** EMMM2 heavily manipulates the filesystem (renaming, moving, toggling mods) while simultaneously watching the filesystem for external changes (`notify` crate).
> **Goal:** Prevent infinite watcher loops, race conditions, "ghost" DB records, and UI desyncs during file operations.

## 1. Watcher Suppression (Crucial)

Any operation that renames, moves, or deletes files within the `mods_path` **MUST** suppress the file watcher to prevent it from triggering a recursive "auto-sync" storm.

### Backend (Rust)

Use the `SuppressionGuard` pattern (RAII) to temporarily pause watcher processing:

```rust
use crate::services::scanner::watcher::SuppressionGuard;

let _guard = SuppressionGuard::new(&watcher_state);
// .. perform fs::rename, fs::remove_dir_all, etc.
// Guard automatically drops and resumes the watcher on scope exit
```

### Frontend (TypeScript / React)

For bulk operations orchestrated from the frontend, use the suppression command with a `try...finally` block to guarantee it resumes even if there's an error:

```typescript
await invoke('set_watcher_suppression_cmd', { suppressed: true });
try {
  await invoke('bulk_rename_or_move', { ... });
} finally {
  await invoke('set_watcher_suppression_cmd', { suppressed: false });
}
```

## 2. Operation Locks (Concurrency)

File operations must not collide globally. The backend provides an `OperationLock`.
All commands that mutate the filesystem layout should acquire the lock first:

```rust
let _lock = op_lock.acquire().await.map_err(|e| e.to_string())?;
// ... perform safe file ops ...
```

## 3. Disconnected Folder Names (The "DISABLED" Prefix)

The application disables mods/objects by renaming their folders (adding a `DISABLED ` prefix).

- **Rule:** Never trust `subPath` or folder names alone for identification, especially when navigating objects.
- **Rule:** When tracking or toggling an entity (like an Object), prefer querying the backend via its `object_id` or `id` instead of its folder name. The DB knows the `folder_path` and `status` authoritatively.

## 4. Source of Truth (FS vs DB)

- The **Filesystem (FS)** is the ultimate source of truth.
- However, the **Database (DB)** must mirror the FS exactly.
- Every FS mutation command must be immediately followed by a DB `UPDATE` or `DELETE` in the same transaction/function scope before responding to the frontend.
- Frontend must use optimistic updates cautiously and **ALWAYS** `invalidateQueries` in the `finally` or `onSettled` block to re-sync with the authoritative DB state.

## 5. Directory Verification

Before moving a file or folder, **always** ensure the destination parent directory exists (`fs::create_dir_all`). Before deleting, tolerate "not found" errors cleanly (`std::io::ErrorKind::NotFound`).
