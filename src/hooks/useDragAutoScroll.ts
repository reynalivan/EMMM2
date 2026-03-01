import { useEffect, useRef } from 'react';

interface DragPosition {
  x: number;
  y: number;
}

interface UseDragAutoScrollProps {
  containerRef: React.RefObject<HTMLElement | null>;
  dragPosition: DragPosition | null;
  /** Speed multiplier for scrolling (default: 5) */
  speed?: number;
  /** Distance from edge in pixels to trigger scroll (default: 50) */
  threshold?: number;
}

/**
 * Automatically scrolls a container when a drag position is near its top or bottom edges.
 */
export function useDragAutoScroll({
  containerRef,
  dragPosition,
  speed = 8,
  threshold = 60,
}: UseDragAutoScrollProps) {
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (!el || !dragPosition) {
      if (rafRef.current) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
      return;
    }

    const { top, bottom } = el.getBoundingClientRect();
    const y = dragPosition.y;

    // Check if within bounds vertically, but allow some leniency horizontally
    // Usually, during drag and drop, you want scroll to happen even if mouse is slightly off-center
    // but definitely between top and bottom of the element.
    if (y < top || y > bottom) {
      if (rafRef.current) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
      return;
    }

    let scrollAmount = 0;

    // Near top edge (scroll up)
    if (y - top < threshold) {
      // Closer = faster
      const intensity = 1 - (y - top) / threshold;
      scrollAmount = -Math.max(1, intensity * speed);
    }
    // Near bottom edge (scroll down)
    else if (bottom - y < threshold) {
      const intensity = 1 - (bottom - y) / threshold;
      scrollAmount = Math.max(1, intensity * speed);
    }

    if (scrollAmount !== 0) {
      const scrollLoop = () => {
        if (el) {
          el.scrollTop += scrollAmount;
          rafRef.current = requestAnimationFrame(scrollLoop);
        }
      };

      if (!rafRef.current) {
        rafRef.current = requestAnimationFrame(scrollLoop);
      }
    } else {
      if (rafRef.current) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    }

    return () => {
      if (rafRef.current) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, [dragPosition, containerRef, speed, threshold]);
}
