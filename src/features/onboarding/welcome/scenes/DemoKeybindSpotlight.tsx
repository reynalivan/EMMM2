import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { motion, useMotionValue, useMotionTemplate, animate, AnimatePresence } from 'motion/react';
import { Gamepad2 } from 'lucide-react';
import { usePrefersReducedMotion } from '../../../../hooks/usePrefersReducedMotion';
import { DEMO_KEYBINDS } from '../demoTypes';

export default function DemoKeybindSpotlight() {
  const { t } = useTranslation('welcome');
  const prefersReduced = usePrefersReducedMotion();

  // X, Y coordinates as motion values (0-100% of container)
  const spotX = useMotionValue(80);
  const spotY = useMotionValue(20);

  // Combine into a CSS radial-gradient mask
  const maskImage = useMotionTemplate`radial-gradient(120px circle at ${spotX}% ${spotY}%, transparent 10%, black 80%)`;

  const [showKeys, setShowKeys] = useState(false);

  useEffect(() => {
    // Reduced motion skips the spotlight sweep entirely.
    if (prefersReduced) return;

    // Using unknown because explicit AnimationControls differs slightly across motion versions
    const controls: unknown = animate([
      // Linger on the KeyViewer overlay corner, then sweep down to where the
      // key pills land — the two things this scene is actually about.
      [spotX, 78, { duration: 1.0, ease: 'easeOut' }],
      [spotY, 25, { duration: 1.0, ease: 'easeOut', at: '<' }],
      [spotX, 50, { duration: 1.4, ease: 'easeInOut', at: '+0.5' }],
      [spotY, 58, { duration: 1.4, ease: 'easeInOut', at: '<' }],
    ]);

    const keyTimer = setTimeout(() => setShowKeys(true), 1800);

    return () => {
      const ctrl = controls as Record<string, unknown>;
      if (ctrl && typeof ctrl.stop === 'function') ctrl.stop();
      clearTimeout(keyTimer);
    };
  }, [spotX, spotY, prefersReduced]);

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
        <h3 className="text-lg font-bold">{t('demo.keybinds_instantly')}</h3>
        <p className="text-sm text-base-content/60">{t('demo.press_help')}</p>
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

      {/* The backdrop is the GAME, not app chrome: these hotkeys fire while
          the game has focus, so showing a toolbar here would imply shortcuts
          the app does not have. */}
      <div className="flex flex-col items-center w-full max-w-lg mt-12 opacity-60">
        <div className="w-full aspect-video bg-base-200 rounded-xl border border-base-content/10 relative overflow-hidden">
          <Gamepad2 className="w-10 h-10 opacity-30 absolute inset-0 m-auto" />

          {/* KeyViewer overlay corner, the thing F7 toggles */}
          <div className="absolute top-3 right-3 flex flex-col gap-1 items-end">
            <div className="h-1.5 w-14 bg-base-content/20 rounded" />
            <div className="h-1.5 w-10 bg-base-content/20 rounded" />
            <div className="h-1.5 w-12 bg-base-content/20 rounded" />
          </div>
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
                    {t(kb.action)}
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
