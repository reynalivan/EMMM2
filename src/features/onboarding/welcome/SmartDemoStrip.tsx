import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { AnimatePresence, motion } from 'motion/react';
import { usePrefersReducedMotion } from '../../../hooks/usePrefersReducedMotion';
import { SCENE_DURATION_MS } from './demoTypes';

import DemoAutoOrganize from './scenes/DemoAutoOrganize';
import DemoTogglePreset from './scenes/DemoTogglePreset';
import DemoKeybindSpotlight from './scenes/DemoKeybindSpotlight';

const SCENES = [
  { Component: DemoAutoOrganize, duration: SCENE_DURATION_MS.A_AUTO_ORGANIZE },
  { Component: DemoTogglePreset, duration: SCENE_DURATION_MS.B_TOGGLE_PRESET },
  { Component: DemoKeybindSpotlight, duration: SCENE_DURATION_MS.C_KEYBIND_SPOTLIGHT },
];

export default function SmartDemoStrip({
  isPausedFromParent = false,
}: {
  isPausedFromParent?: boolean;
}) {
  const { t } = useTranslation('welcome');
  const [currentSceneIdx, setCurrentSceneIdx] = useState(0);
  const prefersReduced = usePrefersReducedMotion();

  // Track the timeout so we can pause/resume
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    // If paused, clear the timer and wait. (This freezes the loop, not the current scene's
    // internal entry animation — the sub-components manage that themselves.)
    if (isPausedFromParent) {
      if (timerRef.current) clearTimeout(timerRef.current);
      return;
    }

    // Reduced motion still loops, just without the internal heavy motion.
    const duration = prefersReduced ? 2500 : SCENES[currentSceneIdx].duration;

    timerRef.current = setTimeout(() => {
      setCurrentSceneIdx((prev) => (prev + 1) % SCENES.length);
    }, duration + 350); // pad for the crossfade exit

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [currentSceneIdx, isPausedFromParent, prefersReduced]);

  const drift = prefersReduced ? 0 : 10;
  const sceneVariants = {
    initial: { opacity: 0, y: drift },
    animate: {
      opacity: 1,
      y: 0,
      transition: { duration: prefersReduced ? 0.2 : 0.45, ease: 'easeOut' as const },
    },
    exit: { opacity: 0, y: -drift, transition: { duration: prefersReduced ? 0.15 : 0.28 } },
  };

  const { Component: Scene } = SCENES[currentSceneIdx];

  return (
    <div
      className="w-full max-w-3xl mx-auto h-75 [@media(max-height:750px)]:h-60 relative rounded-2xl bg-base-100/40 backdrop-blur-md border border-base-content/10 shadow-xl overflow-hidden focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
      tabIndex={0}
      aria-label={t('demo.aria_label')}
    >
      <div className="w-full h-75 absolute top-0 left-0 origin-top [@media(max-height:750px)]:scale-[0.8] transition-transform duration-500">
        <AnimatePresence mode="wait">
          <motion.div
            key={currentSceneIdx}
            variants={sceneVariants}
            initial="initial"
            animate="animate"
            exit="exit"
            className="w-full h-full absolute inset-0"
          >
            <Scene />
          </motion.div>
        </AnimatePresence>
      </div>

      {/* Progress indicators — the active one fills over the scene's own duration,
          and freezes in place while the strip is paused. */}
      <div className="absolute bottom-3 left-0 right-0 flex justify-center gap-2 z-30">
        {SCENES.map((scene, idx) => (
          <button
            key={idx}
            aria-label={t('demo.aria_go_to_scene', { count: idx + 1 })}
            onClick={() => setCurrentSceneIdx(idx)}
            className={`h-1.5 overflow-hidden rounded-full transition-all duration-300 ${
              idx === currentSceneIdx
                ? 'w-6 bg-primary/25'
                : 'w-2 bg-base-content/20 hover:bg-base-content/40'
            }`}
          >
            {idx === currentSceneIdx && (
              <span
                key={currentSceneIdx}
                className="demo-progress-fill block h-full w-full rounded-full bg-primary"
                style={{
                  animationDuration: `${scene.duration + 350}ms`,
                  animationPlayState: isPausedFromParent ? 'paused' : 'running',
                }}
              />
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
