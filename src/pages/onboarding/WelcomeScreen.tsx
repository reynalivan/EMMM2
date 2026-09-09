import { formatAppError } from '../../shared/lib/appError';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../shared/api/tauri/bindings';
import { open } from '@tauri-apps/plugin-dialog';
import { Search, FolderOpen, ChevronRight, Loader2, AlertCircle, Globe } from 'lucide-react';
import { motion } from 'motion/react';
import type { GameConfig } from '@/entities/game';
import { pathsEqual } from '../../shared/lib/pathKey';
import { usePrefersReducedMotion } from '../../shared/lib/hooks/usePrefersReducedMotion';
import { ManualSetupForm } from './components/ManualSetupForm';
import { AutoDetectResult } from './components/AutoDetectResult';
import AuroraBackground from './components/welcome/AuroraBackground';
import SmartDemoStrip from './components/welcome/SmartDemoStrip';
import AnimatedLogo from './components/welcome/AnimatedLogo';
import { useOnboardingDiskProgress } from './hooks/useOnboardingDiskProgress';
import {
  estimatedRemainingMs,
  formatEstimatedDuration,
  type IndexingProgress,
} from './utils/indexingProgress';

type Screen = 'welcome' | 'auto-detect' | 'manual' | 'result';

const EASE_OUT: [number, number, number, number] = [0.22, 1, 0.36, 1];

export default function WelcomeScreen({
  onComplete,
}: {
  onComplete: (games: GameConfig[]) => void;
}) {
  const { t, i18n } = useTranslation(['welcome', 'onboarding']);
  const [view, setView] = useState<Screen>('welcome');
  const [isScanning, setIsScanning] = useState(false);
  const [isIndexing, setIsIndexing] = useState(false);
  const [indexingProgress, setIndexingProgress] = useState<IndexingProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [detectedGames, setDetectedGames] = useState<GameConfig[]>([]);
  const [isDemoPaused, setIsDemoPaused] = useState(false);
  const prefersReduced = usePrefersReducedMotion();
  const diskProgress = useOnboardingDiskProgress(isIndexing, detectedGames);

  // One shared entrance rhythm for every block on the welcome view.
  const fade = {
    hidden: { opacity: 0 },
    show: { opacity: 1, transition: { duration: prefersReduced ? 0.15 : 0.55, ease: EASE_OUT } },
  };
  // `rise` adds a transform, so it must never wrap a backdrop-filter element — a
  // transformed ancestor becomes the backdrop root and the blur samples nothing.
  const rise = prefersReduced
    ? fade
    : {
        hidden: { opacity: 0, y: 16 },
        show: { opacity: 1, y: 0, transition: { duration: 0.55, ease: EASE_OUT } },
      };

  const handleAutoDetect = async () => {
    setError(null);
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: t('onboarding:welcome.select_folder_title'),
      });

      if (!selectedPath) return;

      setIsScanning(true);
      setView('auto-detect');

      const games = await commands.autoDetectGames(selectedPath);

      setDetectedGames(games);
      setView('result');
    } catch (err) {
      setError(formatAppError(err));
      setView('welcome');
    } finally {
      setIsScanning(false);
    }
  };

  const handleManualComplete = (game: GameConfig) => {
    const duplicate = detectedGames.find((g) => pathsEqual(g.game_exe, game.game_exe));

    if (duplicate) {
      setError(t('onboarding:welcome.duplicate_error', { name: duplicate.name }));
      return; // Do NOT navigate away — stay on the manual form
    }

    setError(null);
    setDetectedGames((prev) => [...prev, game]);
    setView('result');
  };

  const handleRemoveGame = (gameId: string) => {
    setDetectedGames((prev) => {
      const remaining = prev.filter((g) => g.id !== gameId);
      if (remaining.length === 0) {
        setView('welcome');
      }
      return remaining;
    });
  };

  const handleFinalize = async (games: GameConfig[]) => {
    try {
      setError(null);
      setIsIndexing(true);
      const total = Math.max(1, games.length);
      const completedDurationsMs: number[] = [];
      setIndexingProgress({
        completed: 0,
        total,
        currentGame: games[0]?.name ?? null,
        completedDurationsMs,
      });

      // Save the games to DB — this is mandatory
      await commands.saveOnboardingGames(games);

      // Disk Reconcile only. Onboarding must not trigger Deep Match Scanner implicitly.
      for (const [index, game] of games.entries()) {
        setIndexingProgress({
          completed: index,
          total,
          currentGame: game.name,
          completedDurationsMs: [...completedDurationsMs],
        });
        const startedAt = performance.now();
        try {
          await commands.reconcileDiskStateCmd(game.id, 'OnboardingCompleted', null, true);
        } catch (refreshErr) {
          console.warn(
            `[onboarding] reconcileDiskState failed for "${game.name}", Disk Reconcile will retry on next entry:`,
            refreshErr,
          );
        }
        completedDurationsMs.push(performance.now() - startedAt);
        setIndexingProgress({
          completed: index + 1,
          total,
          currentGame: game.name,
          completedDurationsMs: [...completedDurationsMs],
        });
      }

      onComplete(games);
    } catch (err) {
      // Only the save_onboarding_games failure is a hard blocker
      setError(formatAppError(err));
      setIsIndexing(false);
      setIndexingProgress(null);
    }
  };

  // == Welcome View ==
  if (view === 'welcome') {
    return (
      <div className="h-screen w-full bg-transparent overflow-y-auto overflow-x-hidden relative flex flex-col items-center justify-center p-6 z-0">
        <div className="fixed inset-0 z-[-1]">
          <AuroraBackground />
        </div>

        {/* Language Selector */}
        <div className="absolute top-6 right-6 z-50">
          <div className="dropdown dropdown-end">
            <div
              tabIndex={0}
              role="button"
              className="btn btn-ghost btn-sm btn-circle text-base-content/70 hover:text-base-content"
              aria-label={t('onboarding:welcome.language.change')}
            >
              <Globe size={18} />
            </div>
            <ul
              tabIndex={0}
              className="dropdown-content menu bg-base-200/90 backdrop-blur-md rounded-box z-[1] w-32 p-2 shadow-xl border border-base-content/5 mt-2"
            >
              <li>
                <button
                  onClick={() => i18n.changeLanguage('en')}
                  className={i18n.language.startsWith('en') ? 'active' : ''}
                >
                  {t('onboarding:welcome.language.options.en')}
                </button>
              </li>
              <li>
                <button
                  onClick={() => i18n.changeLanguage('id')}
                  className={i18n.language.startsWith('id') ? 'active' : ''}
                >
                  {t('onboarding:welcome.language.options.id')}
                </button>
              </li>
              <li>
                <button
                  onClick={() => i18n.changeLanguage('zh')}
                  className={i18n.language.startsWith('zh') ? 'active' : ''}
                >
                  {t('onboarding:welcome.language.options.zh')}
                </button>
              </li>
            </ul>
          </div>
        </div>

        <motion.div
          initial="hidden"
          animate="show"
          variants={{
            show: {
              transition: { staggerChildren: prefersReduced ? 0 : 0.09, delayChildren: 0.1 },
            },
          }}
          className="max-w-4xl w-full text-center space-y-7 z-10 py-6 my-auto origin-center [@media(max-height:800px)]:scale-95 [@media(max-height:750px)]:scale-90 [@media(max-height:700px)]:scale-[0.85] [@media(max-height:650px)]:scale-[0.8] transition-transform duration-500 ease-out"
        >
          {/* Logo & Title */}
          <motion.div
            variants={rise}
            className="flex flex-col [@media(max-height:750px)]:flex-row items-center justify-center gap-3 [@media(max-height:750px)]:gap-5"
          >
            <div className="mx-auto [@media(max-height:750px)]:mx-0 w-16 h-16 sm:w-20 sm:h-20 [@media(max-height:750px)]:w-14 [@media(max-height:750px)]:h-14 flex items-center justify-center shrink-0 text-base-content hover:text-primary transition-colors duration-300">
              <AnimatedLogo />
            </div>
            <div className="[@media(max-height:750px)]:text-left flex flex-col justify-center">
              <h1 className="text-3xl sm:text-4xl md:text-5xl [@media(max-height:750px)]:text-2xl font-extrabold bg-linear-to-r from-primary to-secondary bg-clip-text text-transparent drop-shadow-sm pb-1">
                {t('onboarding:welcome.title')}
              </h1>
              <p className="text-base-content/60 text-base md:text-lg [@media(max-height:750px)]:text-xs font-medium tracking-wide mt-1">
                {t('onboarding:welcome.subtitle')}
              </p>
            </div>
          </motion.div>

          {/* fade, not rise — the strip is backdrop-blurred (see the `rise` note above) */}
          <motion.div variants={fade}>
            <SmartDemoStrip isPausedFromParent={isDemoPaused} />
          </motion.div>

          {/* Error Alert */}
          {error && (
            <div
              role="alert"
              className="alert alert-error alert-soft max-w-2xl mx-auto text-left text-sm"
            >
              <AlertCircle className="w-5 h-5 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {/* CTA Buttons */}
          <motion.div variants={rise} className="max-w-2xl mx-auto space-y-3">
            <div className="flex flex-col [@media(max-height:750px)]:flex-row max-sm:flex-col! [@media(max-height:750px)]:w-full gap-3">
              <div
                className="w-full [@media(max-height:750px)]:flex-1 max-sm:flex-none tooltip tooltip-bottom flex min-w-0"
                data-tip={t('onboarding:welcome.auto_detect_tip')}
              >
                <motion.button
                  whileHover="hover"
                  whileTap="tap"
                  variants={{ hover: { y: -2 }, tap: { scale: 0.985 } }}
                  onHoverStart={() => setIsDemoPaused(true)}
                  onHoverEnd={() => setIsDemoPaused(false)}
                  onFocus={() => setIsDemoPaused(true)}
                  onBlur={() => setIsDemoPaused(false)}
                  id="btn-auto-detect"
                  className="cta-shine btn btn-primary btn-lg w-full gap-2 sm:gap-3 overflow-hidden border-0 shadow-lg shadow-primary/20 hover:shadow-xl hover:shadow-primary/30 transition-shadow duration-300 [@media(max-height:750px)]:min-h-12 max-sm:min-h-14! [@media(max-height:750px)]:h-12 max-sm:h-14! [@media(max-height:750px)]:px-4"
                  onClick={handleAutoDetect}
                >
                  <span aria-hidden="true" className="cta-shine-bar" />
                  <Search className="w-5 h-5 shrink-0" />
                  <span className="flex-1 text-left truncate [@media(max-height:750px)]:text-sm">
                    {t('onboarding:welcome.auto_detect')}
                  </span>
                  <motion.div
                    variants={{ hover: { x: 5 } }}
                    transition={{ type: 'spring', stiffness: 400, damping: 28 }}
                    className="flex items-center shrink-0 opacity-70"
                  >
                    <ChevronRight className="w-5 h-5" />
                  </motion.div>
                </motion.button>
              </div>

              <motion.button
                whileHover="hover"
                whileTap="tap"
                variants={{ hover: { y: -2 }, tap: { scale: 0.985 } }}
                onHoverStart={() => setIsDemoPaused(true)}
                onHoverEnd={() => setIsDemoPaused(false)}
                onFocus={() => setIsDemoPaused(true)}
                onBlur={() => setIsDemoPaused(false)}
                id="btn-manual-setup"
                className="btn btn-ghost btn-lg w-full [@media(max-height:750px)]:flex-1 max-sm:flex-none! gap-2 sm:gap-3 border border-base-content/10 bg-base-100/50 hover:bg-base-content/8 hover:border-base-content/25 transition-colors duration-300 [@media(max-height:750px)]:min-h-12 max-sm:min-h-14! [@media(max-height:750px)]:h-12 max-sm:h-14! [@media(max-height:750px)]:px-4 min-w-0"
                onClick={() => {
                  setError(null);
                  setView('manual');
                }}
              >
                <FolderOpen className="w-5 h-5 shrink-0" />
                <span className="flex-1 text-left truncate [@media(max-height:750px)]:text-sm">
                  {t('onboarding:welcome.manual_setup')}
                </span>
                <motion.div
                  variants={{ hover: { x: 5 } }}
                  transition={{ type: 'spring', stiffness: 400, damping: 28 }}
                  className="flex items-center shrink-0 opacity-70"
                >
                  <ChevronRight className="w-5 h-5" />
                </motion.div>
              </motion.button>
            </div>

            <p className="text-base-content/45 text-sm font-medium">
              {t('onboarding:welcome.description')}
            </p>
          </motion.div>
        </motion.div>
      </div>
    );
  }

  // == Scanning State ==
  if (view === 'auto-detect' && isScanning) {
    return (
      <div className="min-h-screen bg-base-100 flex items-center justify-center">
        <div className="text-center space-y-6">
          <Loader2 className="w-16 h-16 text-primary animate-spin mx-auto" />
          <div>
            <h2 className="text-2xl font-semibold">{t('onboarding:scanning.title')}</h2>
            <p className="text-base-content/60 mt-2">{t('onboarding:scanning.subtitle')}</p>
          </div>
          {/* Shimmer placeholder cards (EC-1.07) */}
          <div className="w-80 mx-auto space-y-3">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-16 rounded-xl bg-base-200 animate-pulse" />
            ))}
          </div>
        </div>
      </div>
    );
  }

  // == Indexing State ==
  if (isIndexing) {
    const progress = indexingProgress ?? {
      completed: 0,
      total: 1,
      currentGame: null,
      completedDurationsMs: [],
    };
    const rootProgress = diskProgress?.total_units === null ? null : diskProgress;
    const completed = rootProgress?.completed_units ?? progress.completed;
    const total = rootProgress?.total_units ?? progress.total;
    const percent = Math.min(100, Math.max(0, Math.round((completed / total) * 100))) || 0;
    const remaining = rootProgress?.eta_ms ?? estimatedRemainingMs(progress);

    const phaseLabel = diskProgress?.phase
      ? {
          DiscoveringRoots: t('onboarding:indexing.phases.discovering'),
          ScanningRoots: t('onboarding:indexing.phases.scanning'),
          Projecting: t('onboarding:indexing.phases.projecting'),
          Finalizing: t('onboarding:indexing.phases.finalizing'),
          Completed: t('onboarding:indexing.phases.completed'),
          Failed: t('onboarding:indexing.phases.failed'),
        }[diskProgress.phase]
      : t('onboarding:indexing.phases.starting');

    return (
      <div className="min-h-screen bg-base-100 flex items-center justify-center">
        <div className="w-full max-w-md px-8 text-center">
          {/* Header */}
          <div className="mb-8">
            <div className="relative w-16 h-16 mx-auto mb-6">
              <div className="absolute inset-0 bg-primary/20 rounded-full animate-ping motion-reduce:animate-none" />
              <div className="relative bg-base-100 rounded-full w-full h-full flex items-center justify-center shadow-lg border border-base-content/10">
                <Loader2 className="w-8 h-8 text-primary animate-spin motion-reduce:animate-none" />
              </div>
            </div>
            <h2 className="text-2xl font-bold bg-linear-to-r from-primary to-secondary bg-clip-text text-transparent">
              {t('onboarding:indexing.title')}
            </h2>
            <p className="text-base-content/60 mt-2 text-sm font-medium">
              {progress.currentGame
                ? t('onboarding:indexing.processing', { game: progress.currentGame })
                : t('onboarding:indexing.subtitle')}
            </p>
          </div>

          {/* Progress Section */}
          <div
            className="bg-base-200/50 rounded-2xl p-6 shadow-sm border border-base-content/5 space-y-4"
            aria-live="polite"
          >
            {/* Phase & Stats */}
            <div className="flex items-end justify-between text-xs">
              <div className="text-left">
                <span className="font-bold text-primary uppercase tracking-wider text-[10px] block mb-1">
                  {phaseLabel}
                </span>
                <span className="text-base-content/60 font-mono">
                  {completed.toLocaleString()} / {total.toLocaleString()}
                </span>
              </div>
              <div className="text-right">
                <span className="text-2xl font-light text-base-content tracking-tighter">
                  {percent}
                  <span className="text-sm text-base-content/50 ml-0.5">%</span>
                </span>
              </div>
            </div>

            {/* Custom Animated Bar */}
            <div
              className="h-2 w-full bg-base-300/50 rounded-full overflow-hidden shadow-inner relative"
              role="progressbar"
              aria-label={t('onboarding:indexing.progress_label')}
              aria-valuemin={0}
              aria-valuemax={total}
              aria-valuenow={completed}
            >
              <div
                className="absolute top-0 bottom-0 left-0 bg-primary transition-all duration-300 ease-out"
                style={{ width: `${percent}%` }}
              >
                <div className="absolute inset-0 bg-white/20 animate-pulse motion-reduce:animate-none" />
              </div>
            </div>

            {/* Terminal Log View */}
            <div className="bg-neutral text-neutral-content rounded-lg p-2.5 text-left h-10 overflow-hidden relative shadow-inner">
              <div className="absolute top-0 left-0 bottom-0 w-1 bg-primary/80" />
              <div className="text-[10px] font-mono truncate pl-2 opacity-70 flex items-center h-full">
                <span className="mr-2 text-primary opacity-50">&gt;</span>
                {diskProgress?.current_root ?? t('onboarding:indexing.initializing')}
              </div>
            </div>

            {/* ETA */}
            <p className="text-center text-[11px] font-medium text-base-content/40 uppercase tracking-wide pt-1">
              {remaining === null
                ? t('onboarding:indexing.estimating')
                : remaining < 2000
                  ? t('onboarding:indexing.finishing_up')
                  : t('onboarding:indexing.estimated_remaining', {
                      duration: formatEstimatedDuration(remaining),
                    })}
            </p>
          </div>
        </div>
      </div>
    );
  }

  if (view === 'manual') {
    return (
      <ManualSetupForm
        onBack={() => {
          if (detectedGames.length > 0) {
            setView('result');
          } else {
            setView('welcome');
          }
        }}
        onSuccess={handleManualComplete}
      />
    );
  }

  if (view === 'result') {
    return (
      <AutoDetectResult
        games={detectedGames}
        onConfirm={() => handleFinalize(detectedGames)}
        onBack={() => {
          setDetectedGames([]);
          setError(null);
          setView('welcome');
        }}
        onAddMore={() => {
          setError(null);
          setView('manual');
        }}
        onRemoveGame={handleRemoveGame}
      />
    );
  }

  return null;
}
