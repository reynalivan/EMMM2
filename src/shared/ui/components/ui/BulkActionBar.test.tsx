import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import BulkActionBar from './BulkActionBar';

describe('BulkActionBar', () => {
  it('keeps status actions readable against the action bar in every theme', () => {
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

    expect(screen.getByRole('button', { name: 'Mark safe' })).toHaveClass(
      'btn-success',
      'text-success-content',
    );
    expect(screen.getByRole('button', { name: 'Mark unsafe' })).toHaveClass(
      'btn-warning',
      'text-warning-content',
    );
    expect(screen.getByRole('button', { name: 'Enable' })).toHaveClass(
      'btn-success',
      'text-success-content',
    );
    expect(screen.getByRole('button', { name: 'Disable' })).toHaveClass(
      'btn-warning',
      'text-warning-content',
    );
    expect(screen.getByTitle('More actions')).toBeInTheDocument();
  });
});
