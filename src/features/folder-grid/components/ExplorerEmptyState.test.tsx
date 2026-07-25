import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import ExplorerEmptyState from './ExplorerEmptyState';

describe('ExplorerEmptyState', () => {
  it('renders empty state correctly', () => {
    render(<ExplorerEmptyState />);
    expect(screen.getByText('No Object Selected')).toBeInTheDocument();
    expect(screen.getByText(/Select a category/)).toBeInTheDocument();
  });
});
