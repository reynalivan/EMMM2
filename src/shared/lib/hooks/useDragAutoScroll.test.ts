import { renderHook } from '@testing-library/react';
import { useDragAutoScroll } from './useDragAutoScroll';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';

describe('useDragAutoScroll', () => {
  let mockContainer: HTMLElement;
  let rafSpy: ReturnType<typeof vi.spyOn>;
  let cancelRafSpy: ReturnType<typeof vi.spyOn>;
  let rafCallback: FrameRequestCallback | null = null;
  let rafId = 0;

  beforeEach(() => {
    mockContainer = document.createElement('div');
    // Mock getBoundingClientRect
    mockContainer.getBoundingClientRect = vi.fn().mockReturnValue({
      top: 100,
      bottom: 500,
      height: 400,
      width: 400,
      left: 0,
      right: 400,
      x: 0,
      y: 100,
      toJSON: () => {},
    });
    mockContainer.scrollTop = 0;

    rafSpy = vi.spyOn(window, 'requestAnimationFrame').mockImplementation((cb) => {
      rafCallback = cb;
      return ++rafId;
    });
    cancelRafSpy = vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => {
      rafCallback = null;
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    rafCallback = null;
  });

  it('should do nothing if containerRef or dragPosition is null', () => {
    renderHook(() =>
      useDragAutoScroll({
        containerRef: { current: mockContainer },
        dragPosition: null,
      }),
    );
    expect(rafSpy).not.toHaveBeenCalled();

    renderHook(() =>
      useDragAutoScroll({
        containerRef: { current: null },
        dragPosition: { x: 100, y: 100 },
      }),
    );
    expect(rafSpy).not.toHaveBeenCalled();
  });

  it('should scroll up when dragPosition is near the top edge', () => {
    const { rerender } = renderHook(
      (props: { dragPosition: { x: number; y: number } | null }) =>
        useDragAutoScroll({
          containerRef: { current: mockContainer },
          dragPosition: props.dragPosition,
          threshold: 50,
          speed: 10,
        }),
      { initialProps: { dragPosition: null as { x: number; y: number } | null } },
    );

    // Initial state: scrollTop is 100 (let's set it so we can observe scroll UP)
    mockContainer.scrollTop = 100;

    // Y = 110: 10px below top (100). Threshold 50.
    rerender({ dragPosition: { x: 200, y: 110 } });

    expect(rafSpy).toHaveBeenCalled();

    // Trigger the animation frame manually
    if (rafCallback) rafCallback(0);

    // scrollTop should decrease (scroll up)
    expect(mockContainer.scrollTop).toBeLessThan(100);
  });

  it('should scroll down when dragPosition is near the bottom edge', () => {
    const { rerender } = renderHook(
      (props: { dragPosition: { x: number; y: number } | null }) =>
        useDragAutoScroll({
          containerRef: { current: mockContainer },
          dragPosition: props.dragPosition,
          threshold: 50,
          speed: 10,
        }),
      { initialProps: { dragPosition: null as { x: number; y: number } | null } },
    );

    mockContainer.scrollTop = 0;

    // Y = 490: 10px above bottom (500). Threshold 50.
    rerender({ dragPosition: { x: 200, y: 490 } });

    expect(rafSpy).toHaveBeenCalled();

    // Trigger the animation frame manually
    if (rafCallback) rafCallback(0);

    // scrollTop should increase (scroll down)
    expect(mockContainer.scrollTop).toBeGreaterThan(0);
  });

  it('should cancel raf if dragged outside the vertical bounds', () => {
    const { rerender } = renderHook(
      (props: { dragPosition: { x: number; y: number } | null }) =>
        useDragAutoScroll({
          containerRef: { current: mockContainer },
          dragPosition: props.dragPosition,
          threshold: 50,
          speed: 10,
        }),
      { initialProps: { dragPosition: null as { x: number; y: number } | null } },
    );

    // Initially inside
    rerender({ dragPosition: { x: 200, y: 490 } });
    expect(rafSpy).toHaveBeenCalled();

    // Now outside top bound (< 100)
    rerender({ dragPosition: { x: 200, y: 50 } });
    expect(cancelRafSpy).toHaveBeenCalled();
  });
});
