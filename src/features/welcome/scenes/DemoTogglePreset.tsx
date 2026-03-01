import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { Power } from 'lucide-react';
import { DEMO_MODS, usePrefersReducedMotion } from '../demoTypes';

export default function DemoTogglePreset({ onComplete }: { onComplete?: () => void }) {
  const prefersReduced = usePrefersReducedMotion();
  const [isOn, setIsOn] = useState(() => prefersReduced);

  useEffect(() => {
    let toggleTimer: ReturnType<typeof setTimeout>;
    let endTimer: ReturnType<typeof setTimeout>;

    if (prefersReduced) {
      if (onComplete) endTimer = setTimeout(onComplete, 3000);
    } else {
      // Toggle midway
      toggleTimer = setTimeout(() => setIsOn(true), 1800);
      if (onComplete) {
        endTimer = setTimeout(onComplete, 4000);
      }
    }

    return () => {
      clearTimeout(toggleTimer);
      clearTimeout(endTimer);
    };
  }, [prefersReduced, onComplete]);

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="w-full h-full p-6 flex flex-col justify-center items-center"
    >
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1, duration: 0.4 }}
        className="absolute top-6 left-0 right-0 text-center z-20"
      >
        <h3 className="text-lg font-bold">One-click Presets</h3>
        <p className="text-sm text-base-content/60">Applied instantly</p>
      </motion.div>

      <div className="flex w-full max-w-lg items-center justify-between gap-8 mt-12">
        {/* The Toggle Button */}
        <div className="flex flex-col items-center gap-2">
          <motion.div
            animate={{
              scale: isOn ? 1 : 0.95,
              boxShadow: isOn
                ? '0px 0px 20px 0px var(--color-primary)'
                : '0px 0px 0px 0px transparent',
            }}
            transition={{ type: 'spring', stiffness: 300, damping: 20 }}
            className={`w-16 h-16 rounded-full flex items-center justify-center border-2 transition-colors duration-300
              ${isOn ? 'border-primary bg-primary/20 text-primary' : 'border-base-content/20 bg-base-200 text-base-content/50'}`}
          >
            <Power className="w-8 h-8" />
          </motion.div>
          <span className="font-semibold text-xs tracking-wider uppercase opacity-70">
            {isOn ? 'Preset ON' : 'Preset OFF'}
          </span>
        </div>

        {/* The Mod Cards reacting */}
        <div className="flex-1 grid grid-cols-2 gap-3 relative">
          <AnimatePresence>
            {isOn && !prefersReduced && (
              <motion.div
                initial={{ opacity: 0, scale: 0.9 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 1.1 }}
                className="absolute inset-0 bg-primary/10 rounded-xl filter blur-xl -z-10"
              />
            )}
          </AnimatePresence>

          {DEMO_MODS.slice(0, 4).map((mod, i) => {
            const delay = prefersReduced ? 0 : i * 0.05;
            return (
              <motion.div
                key={mod.id}
                initial={false}
                animate={{
                  opacity: isOn ? 1 : 0.4,
                  scale: isOn ? 1 : 0.98,
                  filter: isOn ? 'grayscale(0%)' : 'grayscale(100%)',
                }}
                transition={{ duration: 0.3, delay }}
                className={`bg-base-200 border rounded-lg p-2 flex items-center gap-2 shadow-sm
                  ${isOn ? 'border-primary/50' : 'border-base-content/10'}`}
              >
                <div
                  className={`w-5 h-5 rounded shrink-0 transition-colors duration-300
                  ${isOn ? 'bg-primary' : 'bg-base-300'}`}
                />
                <span className="truncate text-xs font-medium">{mod.name}</span>
              </motion.div>
            );
          })}
        </div>
      </div>
    </motion.div>
  );
}
