---
name: tdd
description: Use when implementing any feature or bugfix, before writing implementation code
---

# Test-Driven Development (TDD) Skill

## Overview

> **Core principle:** If you didn't watch the test fail, you don't know if it tests the right thing.

**Violating the letter of the rules is violating the spirit of the rules.**

## When to Use

**Always:**

- New features
- Bug fixes
- Refactoring
- Behavior changes

**Exceptions:**

- Throwaway prototypes
- Generated code
- Configuration files

Thinking "skip TDD just this once"? Stop. That's rationalization.

## The Iron Law

```
NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
```

Write code before the test? Delete it. Start over.

**No exceptions:**

- Don't keep it as "reference"
- Don't "adapt" it while writing tests
- Don't look at it
- Delete means delete

## Red-Green-Refactor

1.  **🔴 RED**: Write a short test that fails.
2.  **🟩 GREEN**: Write the _minimum_ code to make the test pass.
3.  **🟦 REFACTOR**: Clean up redundancy, improve names, optimize.

### RED - Write Failing Test

Write one minimal test showing what should happen.

**Rust Example:**

```rust
#[test]
fn retries_failed_operations() {
    let mut attempts = 0;
    let result = retry_operation(|| {
        attempts += 1;
        if attempts < 3 { Err("fail") } else { Ok("success") }
    });
    assert_eq!(result, Ok("success"));
    assert_eq!(attempts, 3);
}
```

**Requirements:**

- One behavior
- Clear name
- Real code (no mocks unless unavoidable)

### Verify RED - Watch It Fail

**MANDATORY. Never skip.**

- Confirm it fails (not errors).
- Failure message is expected.
- Fails because feature missing.

### GREEN - Minimal Code

Write simplest code to pass the test. Don't add features, refactor other code, or "improve" beyond the test.

### Verify GREEN - Watch It Pass

**MANDATORY.**

- Test passes.
- Other tests still pass.

### REFACTOR - Clean Up

After green only:

- Remove duplication.
- Improve names.
- Extract helpers.

**Keep tests green.**

## Why Order Matters

**"I'll write tests after to verify it works"**
Tests written after code pass immediately. Passing immediately proves nothing:

- Might test wrong thing.
- Might test implementation, not behavior.
- Might miss edge cases.

**"I already manually tested all the edge cases"**
Manual testing is ad-hoc. You can't re-run it when code changes.

## Common Rationalizations

| Excuse                         | Reality                                                       |
| ------------------------------ | ------------------------------------------------------------- |
| "Too simple to test"           | Simple code breaks. Test takes 30 seconds.                    |
| "I'll test after"              | Tests passing immediately prove nothing.                      |
| "Already manually tested"      | Ad-hoc ≠ systematic. No record, can't re-run.                 |
| "Deleting X hours is wasteful" | Sunk cost fallacy. Keeping unverified code is technical debt. |
| "TDD will slow me down"        | TDD faster than debugging. Pragmatic = test-first.            |

## Red Flags - STOP and Start Over

- Code before test
- Test after implementation
- Test passes immediately
- Can't explain why test failed
- "I already manually tested it"

**All of these mean: Delete code. Start over with TDD.**

## Verification Checklist

Before marking work complete:

- [ ] Every new function/method has a test
- [ ] Watched each test fail before implementing
- [ ] Each test failed for expected reason
- [ ] Wrote minimal code to pass each test
- [ ] All tests pass
- [ ] Tests use real code (mocks only if unavoidable)

## References

- [Rust TDD Patterns](references/rust_tdd.md)
- [React TDD Patterns](references/react_tdd.md)
- [Testing Anti-Patterns](references/testing_anti_patterns.md)
