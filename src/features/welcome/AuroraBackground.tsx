import { motion, useTime, useTransform } from 'motion/react';
import { usePrefersReducedMotion } from '../../hooks/usePrefersReducedMotion';

export default function AuroraBackground() {
  const prefersReducedMotion = usePrefersReducedMotion();
  const time = useTime();

  // If reduced motion is preferred, we use a static value (e.g., 0)
  // Otherwise, we do a slow drift.
  // 60000ms = 1 rotation per minute
  const rotate1 = useTransform(time, [0, 60000], [0, 360], { clamp: false });
  const rotate2 = useTransform(time, [0, 75000], [0, -360], { clamp: false });
  const rotate3 = useTransform(time, [0, 90000], [90, 450], { clamp: false });

  return (
    <div
      data-testid="aurora-bg"
      className="fixed inset-0 overflow-hidden pointer-events-none -z-10 bg-base-100"
    >
      {/* Background base */}
      <div className="absolute inset-0 bg-base-100" />

      {/* Aurora Blobs */}
      <div className="absolute inset-0 opacity-30 dark:opacity-20 mix-blend-screen overflow-hidden filter blur-[100px]">
        {/* Blob 1: Primary color */}
        <motion.div
          className="absolute -top-1/4 -left-1/4 w-[150vw] h-[150vh] rounded-full opacity-50 
                     bg-[radial-gradient(circle_at_center,var(--color-primary)_0%,transparent_50%)]"
          style={{ rotate: prefersReducedMotion ? 0 : rotate1 }}
        />

        {/* Blob 2: Secondary color */}
        <motion.div
          className="absolute -bottom-1/4 -right-1/4 w-[150vw] h-[150vh] rounded-full opacity-50 
                     bg-[radial-gradient(circle_at_center,var(--color-secondary)_0%,transparent_50%)]"
          style={{ rotate: prefersReducedMotion ? 0 : rotate2 }}
        />

        {/* Blob 3: Accent/Tertiary color to blend */}
        <motion.div
          className="absolute top-1/4 left-1/4 w-screen h-screen rounded-full opacity-30 
                     bg-[radial-gradient(circle_at_center,var(--color-accent)_0%,transparent_50%)]"
          style={{ rotate: prefersReducedMotion ? 0 : rotate3 }}
        />
      </div>

      {/* Subtle Noise Overlay for premium texture */}
      <div
        className="absolute inset-0 opacity-[0.03] dark:opacity-[0.05] pointer-events-none mix-blend-overlay"
        style={{
          backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E")`,
          backgroundRepeat: 'repeat',
        }}
      />
    </div>
  );
}
