import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { motion, LayoutGroup } from 'motion/react';
import { Folder } from 'lucide-react';
import { usePrefersReducedMotion } from '../../../../hooks/usePrefersReducedMotion';
import { DEMO_MODS } from '../demoTypes';

export default function DemoAutoOrganize() {
  const { t } = useTranslation('welcome');
  const prefersReduced = usePrefersReducedMotion();
  const [isOrganized, setIsOrganized] = useState(() => prefersReduced);
  const [showSweep, setShowSweep] = useState(false);

  // Reduced motion just skips to the end state.
  useEffect(() => {
    if (prefersReduced) return;

    const sweepTimer = setTimeout(() => setShowSweep(true), 1200);
    const organizeTimer = setTimeout(() => {
      setIsOrganized(true);
      setShowSweep(false);
    }, 3000);

    return () => {
      clearTimeout(sweepTimer);
      clearTimeout(organizeTimer);
    };
  }, [prefersReduced]);

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
      className={`bg-base-200/80 border border-base-content/10 rounded-lg shadow-sm flex items-center gap-2 ${
        inFolder ? 'text-[11px] px-2 py-1' : 'text-sm w-full px-3 py-1.5'
      }`}
    >
      <div className={`rounded bg-base-300 shrink-0 ${inFolder ? 'w-4 h-4' : 'w-5 h-5'}`} />
      <span className="truncate flex-1 font-medium text-left">{t(mod.name)}</span>
      {!inFolder && (
        <motion.span
          initial={{ opacity: 0, scale: 0.6 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: 0.4, type: 'spring', stiffness: 400, damping: 30 }}
          className="shrink-0 rounded-full bg-base-content/10 px-2 py-0.5 text-[10px] font-medium text-base-content/55"
        >
          {t(`demo.tag_${mod.typeTag.toLowerCase()}`)}
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
        <h3 className="text-lg font-bold">{t('features.auto_organize')}</h3>
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
            <div className="flex w-full max-w-sm flex-col gap-1.5 mt-6">
              {DEMO_MODS.map((mod) => renderCard(mod, false))}
            </div>
          ) : (
            <div className="grid grid-cols-3 gap-3 w-full px-4 mt-6">
              {(['Character', 'Weapon', 'UI'] as const).map((cat) => (
                <div
                  key={cat}
                  className="flex flex-col bg-base-200/40 rounded-xl p-3 border border-base-content/5"
                >
                  <div className="flex items-center gap-1.5 mb-2 text-base-content/55 font-semibold text-[10px] uppercase tracking-wider">
                    <Folder className="w-3.5 h-3.5 text-primary shrink-0" />
                    <span className="truncate">{t(`demo.tag_${cat.toLowerCase()}`)}</span>
                  </div>
                  <div className="flex-1 space-y-1.5">
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
          {t('features.auto_organized')}
        </motion.div>
      )}
    </motion.div>
  );
}
