---
trigger: model_decision
description: Testing Standards Rule - When writing unit tests, writing E2E tests, verifying code, or handling test cases.
---

# 🧪 Testing & TDD Standards

> **Goal:** Ensure stability through Red-Green-Refactor logic and verifiable automated or manual test cases.

## 1. TDD Workflow (Red-Green-Refactor)

You MUST NOT write implementation code without a failing test or a clear verification plan derived from `@/.docs/.testcase/`.

1. **🔴 RED:** Write a failing test case or define **Manual Verification Steps** in `implementation_plan.md`.
2. **🟩 GREEN:** Write the minimal code needed to pass the test/verification. Ensure strict typing.
3. **🟦 REFACTOR:** Clean code, optimize, match DaisyUI design.

## 2. Backend Testing (Rust)

- **Unit Tests (`#[test]`):** Inside source files (`src/**/*.rs`) in `mod tests { ... }`.
- **Async Tests (`#[tokio::test]`):** For I/O services. Inside `src/**/mod.rs` or `tests/api_*.rs`.
- **Database (`#[sqlx::test]`):** ALL DB interactions must use `#[sqlx::test]`. Provide fresh isolated transactions. NEVER use prod DB for tests.

## 3. Frontend Testing (React)

- **Component Tests:** Vitest + React Testing Library (RTL). Test Behavior, not Implementation (e.g., test roles, not `onClick` props).
- **Mocking:** Mock all `invoke` calls (`vi.mock()`).

## 4. Test Case Compliance

All unit/integration tests must link to Test Cases from `@/.docs/.testcase/`.

```rust
// Covers: TC-1.1-01
#[test]
fn test_feature() { ... }
```
