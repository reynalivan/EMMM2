# React TDD Patterns (Vitest + RTL)

## 1. Component Tests

Focus on **User Behavior** (Click, Type, Read), not implementation details (State, Props).

### The Test (Red)

```tsx
// Button.test.tsx
test('calls onClick when clicked', async () => {
  const handleClick = vi.fn();
  render(<Button onClick={handleClick}>Click Me</Button>);

  // Fails here because Button might not handle click yet
  await userEvent.click(screen.getByText(/click me/i));
  expect(handleClick).toHaveBeenCalledTimes(1);
});
```

### The Implementation (Green)

```tsx
export function Button({ onClick, children }) {
  return <button onClick={onClick}>{children}</button>; // Minimum to pass
}
```

## 2. Mocking Tauri (Typed Commands)

Mock the typed `commands` object from `bindings.ts`. Do NOT mock raw `invoke`.

```ts
// setups/test-setup.ts
vi.mock('../../lib/bindings', () => ({
  commands: {
    getVersion: vi.fn(() => Promise.resolve('1.0.0')),
    listGames: vi.fn(() => Promise.resolve([])),
    // Add mocks for commands used in the test
  },
}));
```

### Per-test override

```ts
import { commands } from '../../lib/bindings';

vi.mocked(commands.getVersion).mockResolvedValue('2.0.0');
```

## 3. Hook Testing

Use `renderHook` for complex logic hooks (`useSafeMode`).

```ts
const { result } = renderHook(() => useCounter());
act(() => {
  result.current.increment();
});
expect(result.current.count).toBe(1);
```
