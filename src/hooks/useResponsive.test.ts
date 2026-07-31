import { renderHook, act } from '@testing-library/react';
import { useResponsive } from './useResponsive';
import { describe, it, expect, vi, afterEach } from 'vitest';

/** Minimal MediaQueryList stand-in that can flip and notify, like a real breakpoint crossing. */
function mockBreakpoint(initialMatches: boolean) {
  const listeners = new Set<() => void>();
  const media = {
    matches: initialMatches,
    media: '(max-width: 767px)',
    addEventListener: (_event: string, listener: () => void) => listeners.add(listener),
    removeEventListener: (_event: string, listener: () => void) => listeners.delete(listener),
  };

  vi.mocked(window.matchMedia).mockReturnValue(media as unknown as MediaQueryList);

  return {
    media,
    cross(matches: boolean) {
      media.matches = matches;
      listeners.forEach((listener) => listener());
    },
    get listenerCount() {
      return listeners.size;
    },
  };
}

describe('useResponsive', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should initialize with isMobile = true when the mobile breakpoint matches', () => {
    mockBreakpoint(true);
    const { result } = renderHook(() => useResponsive());
    expect(result.current.isMobile).toBe(true);
  });

  it('should initialize with isMobile = false when the breakpoint does not match', () => {
    mockBreakpoint(false);
    const { result } = renderHook(() => useResponsive());
    expect(result.current.isMobile).toBe(false);
  });

  it('should update isMobile when the breakpoint is crossed', () => {
    const breakpoint = mockBreakpoint(false);
    const { result } = renderHook(() => useResponsive());
    expect(result.current.isMobile).toBe(false);

    act(() => breakpoint.cross(true));
    expect(result.current.isMobile).toBe(true);

    act(() => breakpoint.cross(false));
    expect(result.current.isMobile).toBe(false);
  });

  it('should detach the listener on unmount', () => {
    const breakpoint = mockBreakpoint(false);
    const { unmount } = renderHook(() => useResponsive());
    expect(breakpoint.listenerCount).toBe(1);

    unmount();
    expect(breakpoint.listenerCount).toBe(0);
  });
});
