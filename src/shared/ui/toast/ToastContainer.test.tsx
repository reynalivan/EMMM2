import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act, fireEvent } from '@testing-library/react';
import { ToastContainer, useToastStore, toast } from './index';

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

    act(() => {
      vi.advanceTimersByTime(2900);
    });
    expect(screen.getByText('Auto dismiss me')).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(100);
    });

    expect(screen.queryByText('Auto dismiss me')).not.toBeInTheDocument();
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it('does not auto-dismiss if duration is 0, requires manual dismiss (TC-36-002)', () => {
    render(<ToastContainer />);
    act(() => {
      toast.error('Permission denied', 0);
    });

    expect(screen.getByText('Permission denied')).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(10000);
    });
    expect(screen.getByText('Permission denied')).toBeInTheDocument();

    const alertBox = screen.getByText('Permission denied').closest('.alert');
    const dismissButton = alertBox?.querySelector('button');

    act(() => {
      if (dismissButton) fireEvent.click(dismissButton);
    });

    expect(screen.queryByText('Permission denied')).not.toBeInTheDocument();
  });
});
