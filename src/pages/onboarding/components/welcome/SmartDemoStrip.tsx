import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { AnimatePresence, motion } from 'motion/react';
import { usePrefersReducedMotion } from '../../../../shared/lib/hooks/usePrefersReducedMotion';
import { LiquidSurface } from '../../../../shared/ui/liquid';
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
    if (isPausedFromParent || prefersReduced) {
      if (timerRef.current) clearTimeout(timerRef.current);
      return;
    }

    const duration = SCENES[currentSceneIdx].duration;

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
    <LiquidSurface
      liquidRole="control"
      className="mx-auto h-75 w-full max-w-3xl rounded-2xl [@media(max-height:750px)]:h-60"
      contentClassName="h-full"
    >
      <div
        className="relative h-full w-full overflow-hidden rounded-[inherit] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
        tabIndex={0}
        aria-label={t('demo.aria_label')}
      >
        <div className="absolute top-0 left-0 h-75 w-full origin-top transition-transform duration-500 [@media(max-height:750px)]:scale-[0.8]">
          <AnimatePresence mode="wait">
            <motion.div
              key={currentSceneIdx}
              variants={sceneVariants}
              initial="initial"
              animate="animate"
              exit="exit"
              className="absolute inset-0 h-full w-full"
            >
              <Scene />
            </motion.div>
          </AnimatePresence>
        </div>

        <div className="absolute right-0 bottom-3 left-0 z-30 flex justify-center gap-2">
          {SCENES.map((scene, idx) => (
            <button
              key={idx}
              aria-label={t('demo.aria_go_to_scene', { count: idx + 1 })}
              onClick={() => setCurrentSceneIdx(idx)}
              className="workspace-interactive grid h-8 w-8 place-items-center rounded-full focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-2"
            >
              <span
                className={`block h-1.5 overflow-hidden rounded-full transition-[width,background-color] duration-150 ${
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
                      animationPlayState:
                        isPausedFromParent || prefersReduced ? 'paused' : 'running',
                    }}
                  />
                )}
              </span>
            </button>
          ))}
        </div>
      </div>
    </LiquidSurface>
  );
}
