import { renderHook, act } from '@testing-library/react';
import { useResponsive } from './useResponsive';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

describe('useResponsive', () => {
  const originalInnerWidth = window.innerWidth;

  beforeEach(() => {
    // Mock innerWidth property so we can change it
    Object.defineProperty(window, 'innerWidth', {
      writable: true,
      configurable: true,
      value: originalInnerWidth,
    });
  });

  afterEach(() => {
    window.innerWidth = originalInnerWidth;
    vi.restoreAllMocks();
  });

  const triggerResizeEvent = (width: number) => {
    window.innerWidth = width;
    window.dispatchEvent(new Event('resize'));
  };

  it('should initialize with isMobile = true if width is under 768', () => {
    window.innerWidth = 500;
    const { result } = renderHook(() => useResponsive());
    expect(result.current.isMobile).toBe(true);
  });

  it('should initialize with isMobile = false if width is 768 or greater', () => {
    window.innerWidth = 800;
    const { result } = renderHook(() => useResponsive());
    expect(result.current.isMobile).toBe(false);
  });

  it('should update isMobile on resize event', () => {
    window.innerWidth = 1000;
    const { result } = renderHook(() => useResponsive());
    expect(result.current.isMobile).toBe(false);

    act(() => {
      triggerResizeEvent(600);
    });

    expect(result.current.isMobile).toBe(true);

    act(() => {
      triggerResizeEvent(1200);
    });

    expect(result.current.isMobile).toBe(false);
  });
});
