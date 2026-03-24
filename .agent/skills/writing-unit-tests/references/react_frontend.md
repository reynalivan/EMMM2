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

## 3. Mocking Tauri Commands (Typed)

**Use for:** Components/hooks that call backend via typed `commands` from `bindings.ts`.

> **IMPORTANT:** Do NOT mock raw `invoke` from `@tauri-apps/api/core`. Always mock the typed `commands` object.

```tsx
import { commands } from '../../lib/bindings';

// Mock the entire bindings module
vi.mock('../../lib/bindings', () => ({
  commands: {
    listMods: vi.fn(),
    getVersion: vi.fn(),
  },
}));

it('fetches mods on mount', async () => {
  vi.mocked(commands.listMods).mockResolvedValue([{ id: 1, name: 'Mod A' }]);

  render(<ModList />);

  expect(await screen.findByText('Mod A')).toBeInTheDocument();
});
```
