import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ErrorBoundary } from './ErrorBoundary';

const ThrowError = ({ shouldThrow }: { shouldThrow?: boolean }) => {
  if (shouldThrow) {
    throw new Error('Test Crash');
  }
  return <div>Safe Component</div>;
};

describe('ErrorBoundary', () => {
  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {}); // Suppress React error logging in test output
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should render children normally if no error occurs', () => {
    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>,
    );

    expect(screen.getByText('Safe Component')).toBeInTheDocument();
  });

  it('should render fallback UI when an error is thrown (TC-36-005)', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>,
    );

    // Verify fallback UI is rendered
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('Test Crash')).toBeInTheDocument();
  });

  it('should reload the page when reload button is clicked', () => {
    const originalReload = window.location.reload;

    // Cannot directly override window.location.reload in standard JSDOM cleanly without deleting location
    Object.defineProperty(window, 'location', {
      writable: true,
      value: { reload: vi.fn() },
    });

    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>,
    );

    const button = screen.getByRole('button', { name: /reload application/i });
    fireEvent.click(button);

    expect(window.location.reload).toHaveBeenCalled();

    // Restore
    Object.defineProperty(window, 'location', {
      writable: true,
      value: { reload: originalReload },
    });
  });
});
