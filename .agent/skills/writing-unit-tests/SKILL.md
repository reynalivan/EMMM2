---
name: writing-unit-tests
description: Guide for writing Rust (Backend) and React (Frontend) unit tests aligned with Project TRD.
---

# Writing Unit Tests

## Overview

This skill provides the standard templates and rules for writing tests in EMMM2.
**Goal:** Ensure stability at every layer (Rust Core, Database, React Components).

## The Iron Laws of Testing

1.  **Isolation:** Tests must not depend on each other (Random Order Execution).
2.  **Determinism:** No `sleep()`, no flaky network calls. Mock external side effects.
3.  **Speed:** Unit tests must run in milliseconds. DB tests using `sqlx::test` are the exception but must still be fast.
4.  **Scope:** Test _behavior_, not implementation details (e.g., test "User can click button", not "Button has class 'btn-primary'").

## Test Stack Strategy (TRD 1.3)

| Layer            | Tool             | Use Case                            | Template                |
| :--------------- | :--------------- | :---------------------------------- | :---------------------- |
| **Logic (Rust)** | `cargo test`     | Pure functions, Parsers, Algorithms | `#[test]`               |
| **Async (Rust)** | `tokio::test`    | Services, File I/O, Commands        | `#[tokio::test]`        |
| **Database**     | `sqlx::test`     | SQL Queries, Repositories           | `#[sqlx::test]`         |
| **UI (React)**   | `Vitest` + `RTL` | Components, Hooks                   | `render(<Component />)` |

## References

- [Backend (Rust) Templates](references/rust_backend.md)
- [Frontend (React) Templates](references/react_frontend.md)
