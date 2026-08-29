import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import ObjectListToolbar from './ObjectListToolbar';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('./FilterPanel', () => ({ default: () => null }));
vi.mock('./ObjectBulkActionBar', () => ({ default: () => null }));

describe('ObjectListToolbar', () => {
  it('does not expose a local refresh action because disk sync is global', () => {
    render(
      <ObjectListToolbar
        sidebarSearchQuery=""
        onSearchChange={vi.fn()}
        schema={undefined}
        selectedObjectType={null}
        onSelectObjectType={vi.fn()}
        sortBy="name"
        onSortChange={vi.fn()}
        isSyncing={false}
        onSync={vi.fn()}
        onCreateNew={vi.fn()}
        categoryFilters={[]}
        activeFilters={{}}
        onFilterChange={vi.fn()}
        onClearFilters={vi.fn()}
        statusFilter="all"
        onStatusFilterChange={vi.fn()}
        showFilterPanel={false}
      />,
    );

    expect(screen.queryByTitle('toolbar.refresh_list')).not.toBeInTheDocument();
  });
});
