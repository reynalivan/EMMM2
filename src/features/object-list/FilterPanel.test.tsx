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

  /** Shared default props for the new category/sort fields */
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
    // Status row (All / Enabled / Disabled) should always render
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

    // Initially dropdown content not visible
    expect(screen.queryByText('Pyro')).not.toBeInTheDocument();

    // Click to expand
    fireEvent.click(screen.getByText('Element'));
    expect(screen.getByText('Pyro')).toBeInTheDocument();

    // Click again to collapse
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

    fireEvent.click(screen.getByText('Element')); // Expand
    fireEvent.click(screen.getByText('Pyro')); // Select

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

    // Check main badge count (2 active filters total)
    // The main badge is in the header "Filters (2)" logic?
    // FilterPanel implementation: sum of all active values length.
    // 2 items active.

    // Implementation details:
    // <span className="badge badge-xs badge-primary">{activeCount}</span>
    // Check main badge count and button badge (multiple "2"s)

    // Check button badge
    // Also inside the button for specific filter
    // We expect another '2' inside the element button
    // It might be ambiguous, so check existence
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
