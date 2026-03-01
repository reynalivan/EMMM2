import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act, fireEvent } from '@testing-library/react';
import { ToastContainer } from './Toast';
import { useToastStore, toast } from '../../stores/useToastStore';

describe('ToastContainer', () => {
  beforeEach(() => {
    useToastStore.setState({ toasts: [] });
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders a toast and automatically dismisses after duration (TC-36-001)', () => {
    render(<ToastContainer />);
    act(() => {
      toast.success('Auto dismiss me', 3000);
    });

    expect(screen.getByText('Auto dismiss me')).toBeInTheDocument();

    // Fast-forward 2900ms, should still be there
    act(() => {
      vi.advanceTimersByTime(2900);
    });
    expect(screen.getByText('Auto dismiss me')).toBeInTheDocument();

    // Fast-forward remaining 100ms
    act(() => {
      vi.advanceTimersByTime(100);
    });

    // Toast unmounts
    expect(screen.queryByText('Auto dismiss me')).not.toBeInTheDocument();
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it('does not auto-dismiss if duration is 0, requires manual dismiss (TC-36-002)', () => {
    render(<ToastContainer />);

    // A duration of 0 means indefinite
    act(() => {
      toast.error('Permission denied', 0);
    });

    expect(screen.getByText('Permission denied')).toBeInTheDocument();

    // Fast-forward 10 seconds, should still be there
    act(() => {
      vi.advanceTimersByTime(10000);
    });
    expect(screen.getByText('Permission denied')).toBeInTheDocument();

    // Click explicit dismiss 'X' button
    const alertBox = screen.getByText('Permission denied').closest('.alert');
    const dismissButton = alertBox?.querySelector('button');

    act(() => {
      if (dismissButton) fireEvent.click(dismissButton);
    });

    expect(screen.queryByText('Permission denied')).not.toBeInTheDocument();
  });
});
