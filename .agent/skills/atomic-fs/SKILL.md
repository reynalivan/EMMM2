---
name: atomic-fs
description: Use this skill for ALL file modifications to ensure data safety. Triggers when: (1) Moving/Renaming files (e.g., enabling mods), (2) Deleting files (MUST use Trash), (3) writing configuration files, or (4) handling file IO errors.
---

# Atomic FS Skill

Implements **Zero Data Loss** policy for file operations.

## Core Principles

1.  **Check Before Write:** Never overwrite without verifying the destination state.
2.  **Transactional Moves:** When moving multiple files, safeguard against partial failures.
3.  **Soft Delete Only:** Hard deletion (`std::fs::remove_file`) is **FORBIDDEN** for user content. Use `trash` crate.

## Usage Patterns

### 1. Safe Rename / Move
**Goal:** Move file `A` to `B` without losing `A` if `B` fails.
> **Code:** See [safe_ops.rs](examples/safe_ops.rs) for `safe_rename`.

### 2. Soft Delete
**Goal:** User deletes a mod.
**Action:** Move to System Trash (Recycle Bin).
> **Code:** See [safe_ops.rs](examples/safe_ops.rs) for `safe_trash`.

### 3. Atomic Write
**Goal:** Write config `config.json`.
**Action:** Write to `config.json.tmp` -> Rename to `config.json`.
> **Strategy:** See [atomic_write.md](references/atomic_write.md).

## Error Handling
Map all IO errors to user-understandable context.
-   `PermissionDenied` -> "File is open in another app (Game/Explorer)?"
-   `AlreadyExists` -> "Destination file already exists."
