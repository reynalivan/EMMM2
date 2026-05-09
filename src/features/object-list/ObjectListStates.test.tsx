import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ObjectListStates from './ObjectListStates';

describe('ObjectListStates', () => {
  const defaultProps = {
    isLoading: false,
    isError: false,
    errorMessage: undefined,
    hasNoGame: false,
    isEmpty: false,
    sidebarSearchQuery: '',
    activeFilters: {},
    onClearFilters: vi.fn(),
    onClearSearch: vi.fn(),
    onCreateNew: vi.fn(),
    onAutoSetup: vi.fn(),
  };

  it('renders loading state', () => {
    render(<ObjectListStates {...defaultProps} isLoading={true} />);
    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  it('renders error state', () => {
    render(<ObjectListStates {...defaultProps} isError={true} errorMessage="Custom Error" />);
    expect(screen.getByText('Custom Error')).toBeInTheDocument();
  });

  it('renders no game state', () => {
    render(<ObjectListStates {...defaultProps} hasNoGame={true} />);
    expect(screen.getByText('states.select_game')).toBeInTheDocument();
  });

  it('renders empty state without filters', () => {
    render(<ObjectListStates {...defaultProps} isEmpty={true} />);
    expect(screen.getByText('states.empty_hint')).toBeInTheDocument();
    expect(screen.getByText('states.add_folder')).toBeInTheDocument();
    expect(screen.getByText('states.auto_setup')).toBeInTheDocument();
  });

  it('renders empty state with filters', () => {
    render(<ObjectListStates {...defaultProps} isEmpty={true} activeFilters={{ cat: ['1'] }} />);
    expect(screen.getByText('states.no_filter_results')).toBeInTheDocument();
    expect(screen.getByTestId('clear-filters-btn')).toBeInTheDocument();
  });
});
