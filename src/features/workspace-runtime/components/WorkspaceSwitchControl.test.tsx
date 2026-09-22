import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { WorkspaceSwitchControl } from './WorkspaceSwitchControl';

describe('WorkspaceSwitchControl', () => {
  it('keeps a pending node switch interactive', () => {
    const onToggle = vi.fn();
    render(
      <WorkspaceSwitchControl
        node={{ node_kind: 'terminal_mod' } as never}
        policy={{ canToggle: true, checked: false, blocked: false, label: 'Disabled' }}
        isPending={false}
        isBusy
        size="xs"
        ariaLabel="Toggle mod"
        onToggle={onToggle}
      />,
    );

    const control = screen.getByRole('checkbox', { name: 'Toggle mod' });
    expect(control).toHaveAttribute('aria-busy', 'true');
    expect(control).toBeEnabled();
    expect(screen.getByRole('status', { name: 'Toggle mod pending' })).toBeInTheDocument();

    fireEvent.click(control);
    expect(onToggle).toHaveBeenCalledTimes(1);
  });
});
