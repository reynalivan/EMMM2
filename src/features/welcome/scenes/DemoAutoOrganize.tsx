import { useState, useEffect } from 'react';
import { motion, LayoutGroup } from 'motion/react';
import { Folder } from 'lucide-react';
import { DEMO_MODS, usePrefersReducedMotion } from '../demoTypes';

export default function DemoAutoOrganize({ onComplete }: { onComplete?: () => void }) {
  const prefersReduced = usePrefersReducedMotion();
  const [isOrganized, setIsOrganized] = useState(() => prefersReduced);
  const [showSweep, setShowSweep] = useState(false);

  // Reduced motion just skips to the end state.
  useEffect(() => {
    let sweepTimer: ReturnType<typeof setTimeout>;
    let organizeTimer: ReturnType<typeof setTimeout>;
    let endTimer: ReturnType<typeof setTimeout>;

    if (prefersReduced) {
      if (onComplete) endTimer = setTimeout(onComplete, 3500);
    } else {
      // 1. Enter and sweep at 800ms
      sweepTimer = setTimeout(() => setShowSweep(true), 1200);
      // 2. Snap to folders at 3000ms
      organizeTimer = setTimeout(() => {
        setIsOrganized(true);
        setShowSweep(false);
      }, 3000);
      // 3. Complete at 3600ms
      if (onComplete) {
        endTimer = setTimeout(onComplete, 3600);
      }
    }

    return () => {
      clearTimeout(sweepTimer);
      clearTimeout(organizeTimer);
      clearTimeout(endTimer);
    };
  }, [prefersReduced, onComplete]);

  const containerVariants = {
    hidden: { opacity: 0 },
    show: {
      opacity: 1,
      transition: { staggerChildren: 0.1 },
    },
    exit: { opacity: 0, transition: { duration: 0.3 } },
  };

  const cardVariants = {
    hidden: { opacity: 0, scale: 0.8, y: 10 },
    show: { opacity: 1, scale: 1, y: 0, transition: { type: 'spring' as const, bounce: 0.4 } },
  };

  const renderCard = (mod: (typeof DEMO_MODS)[0], inFolder: boolean) => (
    <motion.div
      key={mod.id}
      layoutId={`mod-card-${mod.id}`}
      variants={inFolder ? undefined : cardVariants}
      initial={inFolder ? false : undefined}
      className={`bg-base-200 border border-base-content/10 rounded-lg p-2 
                  shadow-sm flex items-center gap-2 ${inFolder ? 'text-xs my-1 py-1' : 'text-sm max-w-xs mx-auto'}`}
    >
      <div className="w-6 h-6 rounded bg-base-300 shrink-0" />
      <span className="truncate flex-1 font-medium">{mod.name}</span>
      {!inFolder && (
        <motion.span
          initial={{ opacity: 0, scale: 0 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: 0.5, type: 'spring' }}
          className="badge badge-accent badge-sm text-[10px]"
        >
          {mod.typeTag}
        </motion.span>
      )}
    </motion.div>
  );

  return (
    <motion.div
      variants={containerVariants}
      initial="hidden"
      animate="show"
      exit="exit"
      className="relative w-full h-full p-6 flex flex-col justify-center items-center"
    >
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1, duration: 0.4 }}
        className="absolute top-6 left-0 right-0 text-center z-20"
      >
        <h3 className="text-lg font-bold">Smart Auto-Organize</h3>
      </motion.div>

      <LayoutGroup id="organize-scene">
        <div className="w-full max-w-2xl relative h-56 flex items-center justify-center">
          {/* Shimmer sweep */}
          {showSweep && !prefersReduced && (
            <motion.div
              initial={{ x: '-100%', opacity: 0 }}
              animate={{ x: '100%', opacity: [0, 1, 0] }}
              transition={{ duration: 1.5, ease: 'linear' }}
              className="absolute inset-y-0 left-0 w-1/3 bg-linear-to-r from-transparent via-primary/30 to-transparent z-10 blur-md pointer-events-none"
            />
          )}

          {!isOrganized ? (
            <div className="flex flex-wrap justify-center gap-x-4 gap-y-2 px-8 mt-12">
              {DEMO_MODS.map((mod) => renderCard(mod, false))}
            </div>
          ) : (
            <div className="grid grid-cols-3 gap-6 w-full px-4 mt-6">
              {['Character', 'Weapon', 'UI'].map((cat) => (
                <div
                  key={cat}
                  className="flex flex-col bg-base-200/50 rounded-xl p-3 border border-base-content/5"
                >
                  <div className="flex items-center gap-2 mb-2 text-base-content/60 font-semibold text-xs uppercase tracking-wider">
                    <Folder className="w-4 h-4 text-primary" />
                    {cat}
                  </div>
                  <div className="flex-1 space-y-1">
                    {DEMO_MODS.filter((m) => m.typeTag === cat).map((mod) => renderCard(mod, true))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </LayoutGroup>

      {isOrganized && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="absolute bottom-10 badge badge-success gap-1 shadow-sm"
        >
          Auto-organized
        </motion.div>
      )}
    </motion.div>
  );
}
