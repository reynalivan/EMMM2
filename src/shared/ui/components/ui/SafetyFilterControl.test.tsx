import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import { SafetyFilterControl } from './SafetyFilterControl';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

describe('SafetyFilterControl', () => {
  it('uses a labelled dropdown trigger and marks the active safety scope', () => {
    render(<SafetyFilterControl value="unsafe" onChange={vi.fn()} compact />);

    expect(screen.getByRole('button', { name: 'Mod safety filter' })).toHaveTextContent('Unsafe');
    expect(screen.getByRole('button', { name: 'Unsafe' })).toHaveAttribute('aria-current', 'true');
  });

  it('selects a safety scope from the dropdown menu', () => {
    const onChange = vi.fn();
    render(<SafetyFilterControl value="all" onChange={onChange} />);

    fireEvent.click(screen.getByRole('button', { name: 'Safe' }));

    expect(onChange).toHaveBeenCalledWith('safe');
  });
});
