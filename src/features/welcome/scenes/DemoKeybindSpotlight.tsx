import { useState, useEffect } from 'react';
import { motion, useMotionValue, useMotionTemplate, animate, AnimatePresence } from 'motion/react';
import { Search, Power, HelpCircle } from 'lucide-react';
import { DEMO_KEYBINDS, usePrefersReducedMotion } from '../demoTypes';

export default function DemoKeybindSpotlight({ onComplete }: { onComplete?: () => void }) {
  const prefersReduced = usePrefersReducedMotion();

  // X, Y coordinates as motion values (0-100% of container)
  const spotX = useMotionValue(80);
  const spotY = useMotionValue(20);

  // Combine into a CSS radial-gradient mask
  const maskImage = useMotionTemplate`radial-gradient(120px circle at ${spotX}% ${spotY}%, transparent 10%, black 80%)`;

  const [showKeys, setShowKeys] = useState(false);

  useEffect(() => {
    let endTimer: ReturnType<typeof setTimeout>;
    let keyTimer: ReturnType<typeof setTimeout>;
    let controls: unknown; // Using unknown because explicit AnimationControls differs slightly across motion versions

    if (prefersReduced) {
      if (onComplete) endTimer = setTimeout(onComplete, 3000);
    } else {
      // Animate spotlight moving to 4 points slower
      controls = animate([
        [spotX, 85, { duration: 1.2, ease: 'easeOut' }],
        [spotY, 80, { duration: 1.2, ease: 'easeOut', at: '<' }], // move to Help
        [spotX, 50, { duration: 1.2, ease: 'easeInOut', at: '+0.4' }],
        [spotY, 50, { duration: 1.2, ease: 'easeInOut', at: '<' }], // move to middle
        [spotX, 15, { duration: 1.2, ease: 'easeOut', at: '+0.5' }],
        [spotY, 80, { duration: 1.2, ease: 'easeOut', at: '<' }], // move to Toggle
      ]);

      keyTimer = setTimeout(() => setShowKeys(true), 1800);

      if (onComplete) {
        endTimer = setTimeout(onComplete, 4500);
      }

      return () => {
        const ctrl = controls as Record<string, unknown>;
        if (ctrl && typeof ctrl.stop === 'function') {
          ctrl.stop();
        }
        clearTimeout(keyTimer);
        clearTimeout(endTimer);
      };
    }

    return () => clearTimeout(endTimer);
  }, [spotX, spotY, prefersReduced, onComplete]);

  const containerVariants = {
    hidden: { opacity: 0 },
    show: {
      opacity: 1,
      transition: { staggerChildren: 0.15 },
    },
  };

  const keyVariants = {
    hidden: { opacity: 0, scale: 0.8, y: 10 },
    show: { opacity: 1, scale: 1, y: 0, transition: { type: 'spring' as const } },
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="w-full h-full relative overflow-hidden flex flex-col items-center justify-center p-6"
    >
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1, duration: 0.4 }}
        className="absolute top-6 left-0 right-0 text-center z-20"
      >
        <h3 className="text-lg font-bold">Keybinds, instantly</h3>
        <p className="text-sm text-base-content/60">Press ? to view keybinds</p>
      </motion.div>

      {/* Dim overlay with spotlight cutout */}
      {!prefersReduced && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 0.8 }}
          className="absolute inset-0 bg-base-100 z-10 pointer-events-none"
          style={{ maskImage, WebkitMaskImage: maskImage }}
        />
      )}

      {/* Background UI pattern to show spotlight effect */}
      <div className="flex justify-between items-end w-full max-w-lg mt-12 opacity-60">
        <div className="flex flex-col items-center gap-2 px-4">
          <Power className="w-8 h-8 opacity-50" />
          <div className="h-2 w-16 bg-base-300 rounded" />
        </div>

        <div className="flex flex-col items-center justify-center gap-4 flex-1">
          <div className="w-48 h-12 bg-base-200 rounded-xl border border-base-content/10 flex items-center px-4">
            <Search className="w-5 h-5 opacity-40 mr-2" />
            <div className="h-3 w-24 bg-base-300 rounded" />
          </div>
        </div>

        <div className="flex flex-col items-center gap-2 px-4">
          <HelpCircle className="w-8 h-8 opacity-50" />
          <div className="h-2 w-16 bg-base-300 rounded" />
        </div>
      </div>

      {/* Keybind Pills */}
      <div className="absolute inset-0 z-20 flex items-center justify-center pointer-events-none mt-20">
        <AnimatePresence>
          {showKeys && (
            <motion.div
              variants={containerVariants}
              initial="hidden"
              animate="show"
              className="flex items-center gap-6"
            >
              {DEMO_KEYBINDS.map((kb) => (
                <motion.div
                  key={kb.keys}
                  variants={keyVariants}
                  className="flex flex-col items-center gap-1"
                >
                  <kbd className="kbd kbd-md shadow-md bg-base-100 border-base-300 font-mono font-bold text-primary">
                    {kb.keys}
                  </kbd>
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-base-content/70 bg-base-100/80 px-1 rounded">
                    {kb.action}
                  </span>
                </motion.div>
              ))}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </motion.div>
  );
}
