import { render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { Power, PowerOff } from 'lucide-react';
import BulkActionBar from './BulkActionBar';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({
    children,
    liquidRole,
    className,
  }: {
    children: ReactNode;
    liquidRole: string;
    className?: string;
  }) => (
    <div data-liquid-role={liquidRole} className={className}>
      {children}
    </div>
  ),
}));

describe('BulkActionBar', () => {
  it('uses restrained status controls and keeps the more menu above adjacent panes', () => {
    render(
      <BulkActionBar
        variant="floating"
        count={1}
        onClear={vi.fn()}
        onMarkSafe={vi.fn()}
        labels={{
          clear: 'Clear selection',
          count: 'Selected',
          safe: 'Mark safe',
          unsafe: 'Mark unsafe',
          more: 'More actions',
        }}
        toggleGroup={{
          tooltip: 'Toggle status',
          enableLabel: 'Enable',
          disableLabel: 'Disable',
          onToggle: vi.fn(),
        }}
        iconActions={[]}
        dropdownActions={[]}
      />,
    );

    const primaryAction = (name: string) =>
      screen
        .getAllByRole('button', { name })
        .find((button) => !button.classList.contains('bulk-action-bar__compact-only'));

    expect(primaryAction('Mark safe')).toHaveClass('btn-ghost');
    expect(primaryAction('Mark unsafe')).toHaveClass('btn-ghost');
    expect(primaryAction('Enable')).toHaveClass('btn-ghost');
    expect(primaryAction('Disable')).toHaveClass('btn-ghost');

    const actionBar = screen.getByRole('button', { name: 'Clear selection' }).closest('.absolute');
    expect(actionBar).toHaveClass('z-[var(--workspace-layer-overlay)]');
    expect(actionBar).toHaveClass('max-w-[calc(100cqw-2rem)]');

    const moreMenu = screen.getByRole('button', { name: 'More actions' }).parentElement;
    expect(moreMenu).toHaveClass('z-[calc(var(--workspace-layer-overlay)+1)]');
    expect(document.querySelectorAll('[data-liquid-role="overlay"]')).toHaveLength(2);
  });

  it('keeps primary actions available from More when the bar is compact', () => {
    render(
      <BulkActionBar
        variant="floating"
        count={2}
        onClear={vi.fn()}
        onMarkSafe={vi.fn()}
        labels={{
          clear: 'Clear selection',
          count: 'Selected',
          safe: 'Mark safe',
          unsafe: 'Mark unsafe',
          more: 'More actions',
        }}
        toggleGroup={{
          tooltip: 'Toggle status',
          enableLabel: 'Enable',
          disableLabel: 'Disable',
          onToggle: vi.fn(),
        }}
        iconActions={[]}
        dropdownActions={[]}
      />,
    );

    const compactActions = document.querySelectorAll('button.bulk-action-bar__compact-only');
    expect(compactActions).toHaveLength(4);
    expect([...compactActions].map((button) => button.textContent)).toEqual([
      'Enable',
      'Disable',
      'Mark safe',
      'Mark unsafe',
    ]);
  });

  it('keeps every floating menu action on one line within a viewport-safe overlay', () => {
    render(
      <BulkActionBar
        variant="floating"
        count={2}
        onClear={vi.fn()}
        onMarkSafe={vi.fn()}
        labels={{
          clear: 'Clear selection',
          count: 'Selected',
          safe: 'Mark as Safe',
          unsafe: 'Mark as Unsafe',
          more: 'More actions',
          menuTitle: 'Bulk Operations',
        }}
        toggleGroup={{
          tooltip: 'Toggle status',
          enableLabel: 'Enable',
          disableLabel: 'Disable',
          onToggle: vi.fn(),
        }}
        iconActions={[]}
        dropdownActions={[
          { icon: Power, label: 'Move to Object...', onClick: vi.fn() },
          { icon: PowerOff, label: 'Remove Favorite', onClick: vi.fn() },
        ]}
      />,
    );

    expect(screen.getByText('Move to Object...').closest('button')).toHaveClass(
      'whitespace-nowrap',
    );
    expect(screen.getByText('Remove Favorite').closest('button')).toHaveClass('whitespace-nowrap');
    expect(document.querySelector('[data-liquid-role="overlay"].w-60')).toHaveClass(
      'max-w-[calc(100cqw-2rem)]',
    );
  });

  it('uses a contrast-bearing one-line menu in the inline object-list bar', () => {
    render(
      <BulkActionBar
        variant="inline"
        count={2}
        onClear={vi.fn()}
        onMarkSafe={vi.fn()}
        labels={{
          clear: 'Clear selection',
          count: '2 selected',
          safe: 'Mark safe',
          unsafe: 'Mark unsafe',
          more: 'More actions',
        }}
        iconActions={[]}
        dropdownActions={[{ icon: Power, label: 'Add tags', onClick: vi.fn() }]}
      />,
    );

    expect(
      document.querySelector('[data-liquid-role="overlay"].object-bulk-action-menu'),
    ).toBeTruthy();
    expect(screen.getByText('Mark safe').closest('button')).toHaveClass(
      'bulk-action-bar__menu-item',
    );
  });
});
