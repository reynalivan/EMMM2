import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import ObjectListToolbar from './ObjectListToolbar';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => (key === 'toolbar.search_placeholder' ? 'Search objects...' : key),
  }),
}));

vi.mock('./FilterPanel', () => ({ default: () => null }));
vi.mock('./ObjectBulkActionBar', () => ({ default: () => null }));
vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children, className }: { children: ReactNode; className?: string }) => (
    <div className={className}>{children}</div>
  ),
}));

describe('ObjectListToolbar', () => {
  const defaultProps = {
    sidebarSearchQuery: '',
    onSearchChange: vi.fn(),
    schema: undefined,
    selectedObjectType: null,
    onSelectObjectType: vi.fn(),
    sortBy: 'name' as const,
    onSortChange: vi.fn(),
    isSyncing: false,
    onSync: vi.fn(),
    onCreateNew: vi.fn(),
    categoryFilters: [],
    activeFilters: {},
    onFilterChange: vi.fn(),
    onClearFilters: vi.fn(),
    statusFilter: 'all' as const,
    onStatusFilterChange: vi.fn(),
    showFilterPanel: false,
  };

  it('does not expose a local refresh action because disk sync is global', () => {
    render(<ObjectListToolbar {...defaultProps} />);

    expect(screen.queryByTitle('toolbar.refresh_list')).not.toBeInTheDocument();
  });

  it('keeps search compact until the field is active', () => {
    render(<ObjectListToolbar {...defaultProps} />);

    expect(screen.getByTestId('icon-search')).toBeInTheDocument();
    const search = screen.getByRole('searchbox', { name: 'Search objects...' });
    expect(search).toHaveClass('pl-9', 'opacity-0');
    expect(screen.getByTestId('object-list-search')).toHaveClass('w-8');

    fireEvent.focus(search);

    expect(search).toHaveClass('opacity-100');
    expect(screen.getByTestId('object-list-search')).toHaveClass('w-56');
  });
});
