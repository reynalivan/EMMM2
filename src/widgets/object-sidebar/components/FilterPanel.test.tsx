/**
 * Component Tests for Epic 3: FilterPanel
 * Covers:
 * - TC-3.1-02 (Category Filter - UI part)
 * - NC-3.4-01 (Filter Interaction)
 */

import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import type { ReactNode } from 'react';
import FilterPanel from './FilterPanel';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children, className }: { children: ReactNode; className?: string }) => (
    <div className={className}>{children}</div>
  ),
}));

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
    expect(screen.getByText('Pyro').closest('.w-56')).toHaveClass('w-56');

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

  it('uses quiet theme surfaces for active chips', () => {
    render(
      <FilterPanel
        filters={mockFilters}
        activeFilters={{ element: ['Pyro'] }}
        onFilterChange={mockOnFilterChange}
        onClearAll={mockOnClearAll}
        statusFilter="enabled"
        onStatusFilterChange={vi.fn()}
        {...defaultCategorySortProps}
        categories={[{ name: 'modpack', label: 'Mod Pack', icon: 'package', color: 'primary' }]}
        selectedCategory="modpack"
      />,
    );

    const enabledChip = screen.getByRole('button', { name: 'Enabled' });
    const modPackChip = screen.getByRole('button', { name: 'Mod Pack' });
    const elementChip = screen.getByRole('button', { name: 'Element1' });

    expect(enabledChip).toHaveClass('bg-base-200', 'text-success');
    expect(enabledChip).not.toHaveClass('btn-success');
    expect(modPackChip).toHaveClass('bg-base-200', 'text-primary');
    expect(modPackChip).not.toHaveClass('btn-primary');
    expect(elementChip).toHaveClass('bg-base-200', 'text-primary');
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
