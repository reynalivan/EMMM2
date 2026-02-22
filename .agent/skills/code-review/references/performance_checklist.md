# Performance Checklist

## Backend (Rust)
-   **Async Blocking**: Are CPU-intensive tasks inside `async fn` blocking the runtime? (Use `spawn_blocking`).
-   **Cloning**: Excessive `.clone()` on hot paths?
-   **DB Queries**: N+1 queries? (Fetching items in a loop instead of a single `IN` query).
-   **IO**: File I/O inside the main thread? (Must be async or threaded).

## Frontend (React)
-   **Re-renders**: Are Context Providers optimizing children?
-   **Memoization**: Is `useMemo`/`useCallback` used for expensive operations/props?
-   **Virtualization**: Are Lists > 50 items using `TanStack Virtual`?
-   **Bundle Size**: Are large libs imported entirely? (e.g., `import * as _ from 'lodash'`).
