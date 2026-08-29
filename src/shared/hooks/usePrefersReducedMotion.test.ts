import { renderHook } from '@testing-library/react';
import { usePrefersReducedMotion } from './usePrefersReducedMotion';
import { vi, describe, it, expect } from 'vitest';
import * as motionReact from 'motion/react';

vi.mock('motion/react', () => ({
  useReducedMotion: vi.fn(),
}));

describe('usePrefersReducedMotion', () => {
  it('should return true when motion/react returns true', () => {
    vi.mocked(motionReact.useReducedMotion).mockReturnValue(true);
    const { result } = renderHook(() => usePrefersReducedMotion());
    expect(result.current).toBe(true);
  });

  it('should return false when motion/react returns false', () => {
    vi.mocked(motionReact.useReducedMotion).mockReturnValue(false);
    const { result } = renderHook(() => usePrefersReducedMotion());
    expect(result.current).toBe(false);
  });

  it('should return false when motion/react returns null (unsupported)', () => {
    vi.mocked(motionReact.useReducedMotion).mockReturnValue(null);
    const { result } = renderHook(() => usePrefersReducedMotion());
    expect(result.current).toBe(false);
  });
});
