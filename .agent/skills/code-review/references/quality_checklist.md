# Quality Checklist

## General
-   **DRY (Don't Repeat Yourself)**: Is logic duplicated > 3 times?
-   **SRP (Single Responsibility)**: Do functions do one thing?
-   **Naming**: Are variables self-describing? (`userList` vs `u`).
-   **Magic Numbers**: Are constants used instead of raw numbers/strings?

## Rust
-   **Clippy**: Does `cargo clippy` pass?
-   **Unwrap**: Is `.unwrap()` used? (Prefer `?` or `expect()`).

## React
-   **Hooks**: Are Custom Hooks used for complex logic?
-   **Types**: Is `any` used? (Forbidden).
