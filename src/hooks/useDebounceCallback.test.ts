import { renderHook, act } from '@testing-library/react';
import { useDebounceCallback } from './useDebounceCallback';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';

describe('useDebounceCallback', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it('should call the callback after the specified delay', () => {
    const callback = vi.fn();
    const { result } = renderHook(() => useDebounceCallback(callback, 500));

    act(() => {
      result.current('test');
    });

    // Should not be called immediately
    expect(callback).not.toHaveBeenCalled();

    // Advance by half the delay
    act(() => {
      vi.advanceTimersByTime(250);
    });
    expect(callback).not.toHaveBeenCalled();

    // Advance by the rest of the delay
    act(() => {
      vi.advanceTimersByTime(250);
    });
    expect(callback).toHaveBeenCalledWith('test');
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it('should debounce multiple rapid calls', () => {
    const callback = vi.fn();
    const { result } = renderHook(() => useDebounceCallback(callback, 300));

    // Call it multiple times rapidly
    act(() => {
      result.current(1);
      result.current(2);
      result.current(3);
    });

    expect(callback).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(300);
    });

    // Should only be called once with the latest arguments
    expect(callback).toHaveBeenCalledWith(3);
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it('should use default delay of 300ms if not specified', () => {
    const callback = vi.fn();
    const { result } = renderHook(() => useDebounceCallback(callback));

    act(() => {
      result.current('default');
    });

    act(() => {
      vi.advanceTimersByTime(299);
    });
    expect(callback).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(callback).toHaveBeenCalledWith('default');
  });

  it('should clean up the timeout on unmount', () => {
    const callback = vi.fn();
    const { result, unmount } = renderHook(() => useDebounceCallback(callback, 300));

    act(() => {
      result.current('unmount-test');
    });

    unmount();

    act(() => {
      vi.advanceTimersByTime(300);
    });

    expect(callback).not.toHaveBeenCalled();
  });
});
