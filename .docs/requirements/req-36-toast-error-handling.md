# Epic 36: Toast & Error Handling Pipeline

## 1. Executive Summary

- **Problem Statement**: Without a consistent error surface, users see silent failures, white-screen crashes, or raw Rust panic messages — they cannot understand what went wrong or how to recover.
- **Proposed Solution**: A three-layer error system: (1) a `useToastStore` Zustand queue displaying non-intrusive notifications for success/failure of all background operations, (2) a React `ErrorBoundary` catching catastrophic render errors with a recovery UI, and (3) a rollback-aware `CommandError` enum in Rust that serializes to predictable JSON so the frontend can give users actionable messages.
- **Success Criteria**:
  - Success toasts appear in ≤ 100ms after an operation completes.
  - Error toasts include the specific `CommandError` type and human-readable message — never a raw Rust backtrace or `[object Object]`.
  - `ErrorBoundary` catches 100% of React render exceptions (by test suite coverage).
  - `get_log_lines` returns the last 200 log lines in ≤ 500ms.
  - Zero unhandled promise rejections from IPC calls — all `invoke()` calls have `onError` handlers.

---

## 2. User Experience & Functionality

### User Stories

#### US-36.1: Non-Intrusive Notifications

As a user, I want small pop-up messages when background operations succeed or fail, so that I know the app is working.

| ID        | Type        | Criteria                                                                                                                                                                          |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-36.1.1 | ✅ Positive | Given a successful operation (e.g., "Apply Collection"), then a green success toast appears in the bottom-right corner, showing the operation name and auto-dismisses after 3s    |
| AC-36.1.2 | ✅ Positive | Given a failed operation (e.g., "File Access Denied"), then a red error toast appears with the specific error message — it does NOT auto-dismiss; user must click ✕               |
| AC-36.1.3 | ✅ Positive | Given a long-running background operation (e.g., Scan), then an intermediate "in-progress" toast shows with a spinner — it is replaced by the success/failure toast when complete |
| AC-36.1.4 | ⚠️ Edge     | Given more than 5 toasts accumulate simultaneously, then older toasts are stacked/collapsed — the notification area never overflows the viewport                                  |

---

#### US-36.2: App Crash Recovery

As a user, I want the app to catch catastrophic rendering errors, so that I see a recovery screen instead of a blank white page.

| ID        | Type        | Criteria                                                                                                                                                                                                                |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-36.2.1 | ✅ Positive | Given a React component throws an unhandled exception during render, then the `ErrorBoundary` catches it and renders: "Oops, something went wrong" + component name (from `error.componentStack`) + "Reload App" button |
| AC-36.2.2 | ✅ Positive | Given the ErrorBoundary fires, then the error details are also written to the Tauri log file via `tauri_plugin_log` — accessible in the Logs tab                                                                        |
| AC-36.2.3 | ⚠️ Edge     | Given an error occurs in a sub-tree (e.g., FolderGrid crashes), then only that panel shows the ErrorBoundary fallback — the objectlist and top bar remain functional                                                       |

---

#### US-36.3: View System Logs

As a power user or developer, I want to read internal logs from within the UI, so that I can troubleshoot deeper issues.

| ID        | Type        | Criteria                                                                                                                                                                                 |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-36.3.1 | ✅ Positive | Given the Settings > Logs tab, when opened, then `get_log_lines(200)` fetches the last 200 lines from `{app_data_dir}/logs/emmm2.log` and renders them in a monospaced, scrollable panel |
| AC-36.3.2 | ✅ Positive | Given the Logs tab is open, when I click "Open Log Folder", then `open_log_folder()` opens Windows Explorer to `{app_data_dir}/logs` via `tauri::api::shell::open`                       |
| AC-36.3.3 | ❌ Negative | Given the log file doesn't exist yet (first launch), then the Logs panel shows "No logs yet" — no crash, no empty-path error                                                             |

---

### Non-Goals

- No remote error reporting or crash analytics (Sentry-style) — all logging is local.
- No log rotation management within the app — `tauri-plugin-log` handles rolling file rotation.
- Toast system does not queue more than 10 simultaneous toasts — excess are discarded (logged to console).

---

## 3. Technical Specifications

### Architecture Overview

```
CommandError (Rust enum, serialized to JSON):
  { error: "FileAccessDenied", message: "Access denied to path: /mods/Foo" }
  | { error: "DuplicateConflict", message: "..." }
  | { error: "OperationInProgress", message: "..." }
  | ... (all variants in src-tauri/src/types/errors.rs)

useToastStore (Zustand):
  state: { toasts: Toast[] }
  addToast({ type: 'success'|'error'|'info', message, action?, duration? })
  removeToast(id: string)
  → App.tsx maps toasts array to <Toast> portal components

React Query integration:
  all useMutation onError: (err) => addToast({ type: 'error', message: parseCommandError(err) })
  all useMutation onSuccess: (data) => addToast({ type: 'success', message: data.message })

ErrorBoundary (class component):
  componentDidCatch(error, info): log to Tauri log + set error state
  render: if hasError → fallback UI else children

get_log_lines(n: usize) → Vec<String>:
  tail last n lines from app_data_dir/logs/emmm2.log

open_log_folder() → ():
  shell::open(app_data_dir/logs)
```

### Integration Points

| Component      | Detail                                                                              |
| -------------- | ----------------------------------------------------------------------------------- |
| Toast Store    | `useToastStore` (Zustand) — shared across all hooks via `addToast` export           |
| Tauri Logs     | `tauri-plugin-log` (configured at bootstrap, Epic 01) → `emmm2.log` rolling file    |
| ErrorBoundary  | Wraps major UI regions: `FolderGrid`, `PreviewPanel`, full `App`                    |
| `CommandError` | All backend endpoints return `Err(CommandError)` → `serde` serializes to typed JSON |

### Security & Privacy

- **Log files contain path info but no user passwords or PIN hashes** — PIN is never logged.
- **`get_log_lines` path is hardcoded to `app_data_dir/logs/emmm2.log`** — no user-supplied path injection.
- **ErrorBoundary output includes component name only** — no user file content in error screens.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap — `tauri-plugin-log` initialization).
- **Blocks**: All epics — this is the universal error surface used by every command and mutation in the app.
