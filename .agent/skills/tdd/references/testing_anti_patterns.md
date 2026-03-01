# Testing Anti-Patterns

**Load this reference when:** writing or changing tests, adding mocks, or tempted to add test-only methods to production code.

## Overview

Tests must verify real behavior, not mock behavior. Mocks are a means to isolate, not the thing being tested.

**Core principle:** Test what the code does, not what the mocks do.

**Following strict TDD prevents these anti-patterns.**

## The Iron Laws

```
1. NEVER test mock behavior
2. NEVER add test-only methods to production classes
3. NEVER mock without understanding dependencies
```

## Anti-Pattern 1: Testing Mock Behavior

**The violation:**

```tsx
// ❌ BAD: Testing that the mock exists
test('renders sidebar', () => {
  render(<Page />);
  expect(screen.getByTestId('sidebar-mock')).toBeInTheDocument();
});
```

**Why this is wrong:**

- You're verifying the mock works, not that the component works.
- Test passes when mock is present, fails when it's not.
- Tells you nothing about real behavior.

**The Fix:**

```tsx
// ✅ GOOD: Test real component or don't mock it
test('renders sidebar', () => {
  render(<Page />); // Don't mock sidebar if possible
  expect(screen.getByRole('navigation')).toBeInTheDocument();
});
```

### Gate Function

```
BEFORE asserting on any mock element:
  Ask: "Am I testing real component behavior or just mock existence?"
  IF testing mock existence: STOP. Delete assertion or unmock.
```

## Anti-Pattern 2: Test-Only Methods in Production

**The violation:**

```typescript
// ❌ BAD: destroy() only used in tests
class Session {
  async destroy() {
    // Looks like production API!
    await this.workspaceManager?.destroy(this.id);
  }
}

// In tests
afterEach(() => session.destroy());
```

**Why this is wrong:**

- Production class polluted with test-only code.
- Dangerous if accidentally called in production.
- Violates YAGNI.

**The Fix:**

```typescript
// ✅ GOOD: Test utilities handle test cleanup
// In test-utils/
export async function cleanupSession(session: Session) {
  // Use public API or specific test-only helper module
  // Session stays clean
}

// In tests
afterEach(() => cleanupSession(session));
```

### Gate Function

```
BEFORE adding any method to production class:
  Ask: "Is this only used by tests?"
  IF yes: STOP. Put it in test utilities instead.
```

## Anti-Pattern 3: Mocking Without Understanding

**The violation:**

```typescript
// ❌ BAD: Mock breaks test logic
test('detects duplicate server', async () => {
  // Mock prevents config write that test depends on!
  vi.mock('ConfigManager', () => ({
    write: vi.fn().mockResolvedValue(undefined),
  }));

  await addServer(config);
  await addServer(config); // Should throw - but won't because 'write' was mocked out!
});
```

**Why this is wrong:**

- Mocked method had side effect test depended on.
- Test passes for wrong reason or fails mysteriously.

**The Fix:**

```typescript
// ✅ GOOD: Mock at correct level
test('detects duplicate server', () => {
  // Mock the slow part (File System), preserve behavior (Config Logic)
  // Or use a real in-memory adapter

  await addServer(config);
  await addServer(config); // Duplicate detected ✓
});
```

### Gate Function

```
BEFORE mocking any method:
  1. Ask: "What side effects does the real method have?"
  2. Ask: "Does this test depend on any of those side effects?"

  IF depends on side effects:
    Mock at lower level (e.g. IO) or use Test Doubles.
```

## Anti-Pattern 4: Incomplete Mocks

**The violation:**

```typescript
// ❌ BAD: Partial mock - only fields you think you need
const mockUser = {
  id: '123',
  name: 'Alice',
  // Missing: preferences, roles, etc.
};
// Later: Code crashes when accessing user.preferences.theme
```

**Why this is wrong:**

- **Partial mocks hide structural assumptions.**
- **Silent failures** in downstream code.

**The Fix:**

```typescript
// ✅ GOOD: Mirror real API completeness
const mockUser: User = {
  id: '123',
  name: 'Alice',
  preferences: { theme: 'dark' },
  roles: ['admin'],
  // Matches interface completely
};
```

### Gate Function

```
BEFORE creating mock responses:
  Check: "What fields does the real API response contain?"
  Action: Include ALL fields system might consume.
```

## Anti-Pattern 5: Integration Tests as Afterthought

**The violation:**

```
✅ Implementation complete
❌ No tests written
"Ready for testing"
```

**The fix:**

1.  **RED**: Write failing integration test (e.g., `sqlx::test`).
2.  **GREEN**: Implement.
3.  **REFACTOR**.

## Red Flags

- Classes polluted with `#[cfg(test)]` methods that leak into logic.
- Mock setup is >50% of test.
- Test fails when you remove a mock.
- "I'll mock this just to be safe." (Safety comes from testing real code).

## The Bottom Line

**Mocks are tools to isolate external volatility (Network, Disk), not to skip testing logic.**

If TDD reveals you're testing mock behavior, you've gone wrong.
