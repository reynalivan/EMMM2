# React Frontend Testing Templates (Vitest + RTL)

## 1. Component Rendering & Interaction
**Use for:** Buttons, Forms, Displays.

```tsx
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Button } from './Button';

describe('Button', () => {
  it('calls onClick when clicked', async () => {
    const handleClick = vi.fn();
    render(<Button onClick={handleClick}>Submit</Button>);

    const button = screen.getByRole('button', { name: /submit/i });
    await userEvent.click(button);

    expect(handleClick).toHaveBeenCalledTimes(1);
  });
});
```

## 2. Hook Testing
**Use for:** Custom hooks with complex state (`useSafeMode`).

```tsx
import { renderHook, act } from '@testing-library/react';
import { useSafeMode } from '../hooks/useSafeMode';

describe('useSafeMode', () => {
  it('toggles value', () => {
    const { result } = renderHook(() => useSafeMode());

    expect(result.current.isActive).toBe(false);

    act(() => {
      result.current.toggle();
    });

    expect(result.current.isActive).toBe(true);
  });
});
```

## 3. Mocking Tauri Commands
**Use for:** Components that call backend via `invoke`.

```tsx
// test/setup.ts or inside test file
import { mockIPC } from '@tauri-apps/api/mocks';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

it('fetches mods on mount', async () => {
  const invokeMock = vi.mocked(invoke);
  invokeMock.mockResolvedValue([{ id: 1, name: 'Mod A' }]);

  render(<ModList />);

  expect(await screen.findByText('Mod A')).toBeInTheDocument();
});
```
