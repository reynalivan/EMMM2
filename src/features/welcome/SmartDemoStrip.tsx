import { useState, useEffect, useRef } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import { SCENE_DURATION_MS, usePrefersReducedMotion } from './demoTypes';

import DemoAutoOrganize from './scenes/DemoAutoOrganize';
import DemoTogglePreset from './scenes/DemoTogglePreset';
import DemoKeybindSpotlight from './scenes/DemoKeybindSpotlight';

type SceneKey = 'A' | 'B' | 'C';
const SCENES: SceneKey[] = ['A', 'B', 'C'];

export default function SmartDemoStrip({
  isPausedFromParent = false,
}: {
  isPausedFromParent?: boolean;
}) {
  const [currentSceneIdx, setCurrentSceneIdx] = useState(0);
  const prefersReduced = usePrefersReducedMotion();

  const currentScene = SCENES[currentSceneIdx];
  const isActuallyPaused = isPausedFromParent;

  // Track the timeout so we can pause/resume
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const goToNextScene = () => {
    setCurrentSceneIdx((prev) => (prev + 1) % SCENES.length);
  };

  useEffect(() => {
    // If paused, clear the timer and wait. (This will freeze the current scene's internal state
    // if it relies on time. Note: the sub-components manage their own internal entry animations.
    // We just handle the overall loop duration here).
    if (isActuallyPaused) {
      if (timerRef.current) clearTimeout(timerRef.current);
      return;
    }

    let duration = 2000;
    if (currentScene === 'A') duration = SCENE_DURATION_MS.A_AUTO_ORGANIZE;
    if (currentScene === 'B') duration = SCENE_DURATION_MS.B_TOGGLE_PRESET;
    if (currentScene === 'C') duration = SCENE_DURATION_MS.C_KEYBIND_SPOTLIGHT;

    // In reduced motion, we still loop but maybe faster or same speed, just without the internal heavy motion
    if (prefersReduced) duration = 2500;

    timerRef.current = setTimeout(() => {
      goToNextScene();
    }, duration + 500); // 500ms pad for crossfade exit

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [currentScene, currentSceneIdx, isActuallyPaused, prefersReduced]);

  const sceneVariants = {
    initial: { opacity: 0, scale: 1.02 },
    animate: { opacity: 1, scale: 1, transition: { duration: 0.8, ease: 'easeOut' as const } },
    exit: { opacity: 0, scale: 0.98, transition: { duration: 0.6 } },
  };

  return (
    <div
      className="w-full max-w-3xl mx-auto h-[300px] [@media(max-height:750px)]:h-[240px] relative rounded-2xl bg-base-100/40 backdrop-blur-md border border-base-content/10 shadow-xl overflow-hidden group flex items-start justify-center"
      tabIndex={0}
      aria-label="Smart Features Demonstration"
    >
      <div className="w-full h-[300px] absolute top-0 left-0 origin-top [@media(max-height:750px)]:scale-[0.8] transition-transform duration-500">
        <AnimatePresence mode="wait">
          {currentScene === 'A' && (
            <motion.div
              key="scene-a"
              variants={sceneVariants}
              initial="initial"
              animate="animate"
              exit="exit"
              className="w-full h-full absolute inset-0"
            >
              <DemoAutoOrganize />
            </motion.div>
          )}

          {currentScene === 'B' && (
            <motion.div
              key="scene-b"
              variants={sceneVariants}
              initial="initial"
              animate="animate"
              exit="exit"
              className="w-full h-full absolute inset-0"
            >
              <DemoTogglePreset />
            </motion.div>
          )}

          {currentScene === 'C' && (
            <motion.div
              key="scene-c"
              variants={sceneVariants}
              initial="initial"
              animate="animate"
              exit="exit"
              className="w-full h-full absolute inset-0"
            >
              <DemoKeybindSpotlight />
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Progress Indicators */}
      <div className="absolute bottom-3 left-0 right-0 flex justify-center gap-2 z-30">
        {SCENES.map((s, idx) => (
          <button
            key={s}
            aria-label={`Go to scene ${idx + 1}`}
            onClick={() => setCurrentSceneIdx(idx)}
            className={`h-1.5 rounded-full transition-all duration-300 ${
              idx === currentSceneIdx
                ? 'w-6 bg-primary'
                : 'w-2 bg-base-content/20 hover:bg-base-content/40'
            }`}
          />
        ))}
      </div>
    </div>
  );
}
