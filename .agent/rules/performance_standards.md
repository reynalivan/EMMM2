---
trigger: model_decision
description: When optimizing code speed, memory usage, or handling large datasets.
---

# ⚡ Performance Standards

> **Goal:** < 100ms Interaction Latency. Native feel.

## 1. Time Budgets

- **Startup:** < 800ms.
- **Frame:** 16ms (60fps). Use CSS transforms.
- **Interaction:** < 100ms (Optimistic UI).
- **Scan:** < 5s / 1GB content.

## 2. Frontend Constraints

- **Offload Heavy Computation:** Move heavy client-side operations (e.g., DP-based fuzzy matching in `useMasterDbSync.ts`) to a Rust Tauri Command to keep the UI thread unblocked and leverage compiled speed.
- **Virtualization:** MANDATORY for lists > 50 items (`@tanstack/react-virtual`).
- **Re-renders:** Use `React.memo` and Split Contexts.
- **Images:** WebP cache + Lazy Load.

## 3. Backend Constraints

- **Async:** ALL I/O must be `async` (Tokio).
- **Blocking:** Compute-heavy tasks -> `rayon::spawn` / dedicated thread.
- **Memory:** Stream files (Buffer Reader), explicit drop for large buffers.

## 4. Database

- **Schema & Query Optimization:** Enforce strict indexing on frequently queried columns (`game_id`, `is_safe`). Avoid `SELECT *`.
- **SQLx:** Use Connection Pooling & WAL Mode.
