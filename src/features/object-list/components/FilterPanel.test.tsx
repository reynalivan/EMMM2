/**
 * Component Tests for Epic 3: FilterPanel
 * Covers:
 * - TC-3.1-02 (Category Filter - UI part)
 * - NC-3.4-01 (Filter Interaction)
 */

import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import FilterPanel from './FilterPanel';

describe('FilterPanel Component', () => {
  const mockFilters = [
    { key: 'element', label: 'Element', options: ['Pyro', 'Hydro'] },
    { key: 'rarity', label: 'Rarity', options: ['5-Star', '4-Star'] },
  ];

  const mockOnFilterChange = vi.fn();
  const mockOnClearAll = vi.fn();

  const defaultCategorySortProps = {
    selectedCategory: null as string | null,
    onSelectCategory: vi.fn(),
    sortBy: 'name' as const,
    onSortChange: vi.fn(),
  };

  it('renders status filter row even if no metadata filters provided', () => {
    render(
      <FilterPanel
        filters={[]}
        activeFilters={{}}
        onFilterChange={mockOnFilterChange}
        onClearAll={mockOnClearAll}
        statusFilter="all"
        onStatusFilterChange={vi.fn()}
        {...defaultCategorySortProps}
      />,
    );
    expect(screen.getByText('All')).toBeInTheDocument();
    expect(screen.getByText('Enabled')).toBeInTheDocument();
    expect(screen.getByText('Disabled')).toBeInTheDocument();
  });

  it('renders filter buttons correctly', () => {
    render(
      <FilterPanel
        filters={mockFilters}
        activeFilters={{}}
        onFilterChange={mockOnFilterChange}
        onClearAll={mockOnClearAll}
        statusFilter="all"
        onStatusFilterChange={vi.fn()}
        {...defaultCategorySortProps}
      />,
    );
    expect(screen.getByText('Element')).toBeInTheDocument();
    expect(screen.getByText('Rarity')).toBeInTheDocument();
  });

  it('toggles dropdown on click', () => {
    render(
      <FilterPanel
        filters={mockFilters}
        activeFilters={{}}
        onFilterChange={mockOnFilterChange}
        onClearAll={mockOnClearAll}
        statusFilter="all"
        onStatusFilterChange={vi.fn()}
        {...defaultCategorySortProps}
      />,
    );

    expect(screen.queryByText('Pyro')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('Element'));
    expect(screen.getByText('Pyro')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Element'));
    expect(screen.queryByText('Pyro')).not.toBeInTheDocument();
  });

  it('calls onFilterChange when option is selected', () => {
    render(
      <FilterPanel
        filters={mockFilters}
        activeFilters={{}}
        onFilterChange={mockOnFilterChange}
        onClearAll={mockOnClearAll}
        statusFilter="all"
        onStatusFilterChange={vi.fn()}
        {...defaultCategorySortProps}
      />,
    );

    fireEvent.click(screen.getByText('Element'));
    fireEvent.click(screen.getByText('Pyro'));

    expect(mockOnFilterChange).toHaveBeenCalledWith('element', ['Pyro']);
  });

  it('shows badge count for active filters', () => {
    render(
      <FilterPanel
        filters={mockFilters}
        activeFilters={{ element: ['Pyro', 'Hydro'] }}
        onFilterChange={mockOnFilterChange}
        onClearAll={mockOnClearAll}
        statusFilter="all"
        onStatusFilterChange={vi.fn()}
        {...defaultCategorySortProps}
      />,
    );

    const badges = screen.getAllByText('2');
    expect(badges.length).toBeGreaterThanOrEqual(1);
  });

  it('shows Clear All button when active filters exist', () => {
    const { rerender } = render(
      <FilterPanel
        filters={mockFilters}
        activeFilters={{}}
        onFilterChange={mockOnFilterChange}
        onClearAll={mockOnClearAll}
        statusFilter="all"
        onStatusFilterChange={vi.fn()}
        {...defaultCategorySortProps}
      />,
    );
    expect(screen.queryByText(/clear/i)).not.toBeInTheDocument();

    rerender(
      <FilterPanel
        filters={mockFilters}
        activeFilters={{ element: ['Pyro'] }}
        onFilterChange={mockOnFilterChange}
        onClearAll={mockOnClearAll}
        statusFilter="all"
        onStatusFilterChange={vi.fn()}
        {...defaultCategorySortProps}
      />,
    );
    const clearBtn = screen.getByText(/clear/i);
    expect(clearBtn).toBeInTheDocument();

    fireEvent.click(clearBtn);
    expect(mockOnClearAll).toHaveBeenCalled();
  });
});
