// ponytail: the drift is three CSS keyframe animations (see App.css), so the
// welcome path renders without the motion runtime. `prefers-reduced-motion` is
// handled by the media query next to those keyframes.
export default function AuroraBackground() {
  return (
    <div
      data-testid="aurora-bg"
      className="fixed inset-0 overflow-hidden pointer-events-none -z-10 bg-base-100"
    >
      {/* Background base */}
      <div className="absolute inset-0 bg-base-100" />

      {/* Aurora Blobs */}
      <div
        className="absolute inset-0 overflow-hidden filter blur-[100px]"
        style={{
          mixBlendMode: 'var(--aurora-blend)' as React.CSSProperties['mixBlendMode'],
          opacity: 'var(--aurora-opacity)',
        }}
      >
        {/* Blob 1: Primary color */}
        <div
          className="aurora-blob-1 absolute -top-1/4 -left-1/4 w-[150vw] h-[150vh] rounded-full opacity-50
                     bg-[radial-gradient(circle_at_center,var(--color-primary)_0%,transparent_50%)]"
        />

        {/* Blob 2: Secondary color */}
        <div
          className="aurora-blob-2 absolute -bottom-1/4 -right-1/4 w-[150vw] h-[150vh] rounded-full opacity-50
                     bg-[radial-gradient(circle_at_center,var(--color-secondary)_0%,transparent_50%)]"
        />

        {/* Blob 3: Accent/Tertiary color to blend */}
        <div
          className="aurora-blob-3 absolute top-1/4 left-1/4 w-screen h-screen rounded-full opacity-30
                     bg-[radial-gradient(circle_at_center,var(--color-accent)_0%,transparent_50%)]"
        />
      </div>

      {/* Subtle Noise Overlay for premium texture */}
      <div
        className="absolute inset-0 opacity-[0.03] pointer-events-none mix-blend-overlay"
        style={{
          backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E")`,
          backgroundRepeat: 'repeat',
        }}
      />
    </div>
  );
}
