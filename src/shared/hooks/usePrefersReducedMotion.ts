import { useReducedMotion } from 'motion/react';

/**
 * A wrapper hook around Motion's useReducedMotion.
 * Returns true if the user has requested to minimize the amount of non-essential motion.
 * Falls back to false if the API is not supported.
 */
export function usePrefersReducedMotion() {
  const prefersReducedMotion = useReducedMotion();
  return prefersReducedMotion ?? false;
}
