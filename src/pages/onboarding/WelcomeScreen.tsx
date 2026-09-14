import { formatAppError } from '../../shared/lib/appError';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../shared/api/tauri/bindings';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { Search, FolderOpen, ChevronRight, Loader2, AlertCircle, Globe } from 'lucide-react';
import { motion } from 'motion/react';
import type { GameConfig } from '@/entities/game';
import { pathsEqual } from '../../shared/lib/pathKey';
import { setFrontendTelemetryEnabled } from '../../shared/lib/telemetry';
import { usePrefersReducedMotion } from '../../shared/lib/hooks/usePrefersReducedMotion';
import { ManualSetupForm } from './components/ManualSetupForm';
import { AutoDetectResult } from './components/AutoDetectResult';
import AuroraBackground from './components/welcome/AuroraBackground';
import SmartDemoStrip from './components/welcome/SmartDemoStrip';
import AnimatedLogo from './components/welcome/AnimatedLogo';
import { useOnboardingDiskProgress } from './hooks/useOnboardingDiskProgress';
import {
  calculateOverallIndexingProgress,
  estimatedRemainingMs,
  formatEstimatedDuration,
  INDEXING_STEP_COUNT,
  type IndexingWorkPlan,
  type IndexingProgress,
} from './utils/indexingProgress';
import type {
  DiskReconcilePhase,
  OnboardingIndexingSnapshotProgress,
  OnboardingIndexingWorkPlanUpdate,
} from '../../shared/api/tauri/bindings';
import { LiquidSurface } from '@/shared/ui/liquid';

type Screen = 'welcome' | 'auto-detect' | 'manual' | 'result';

const EASE_OUT: [number, number, number, number] = [0.22, 1, 0.36, 1];

function activityTranslationKey(
  phase: DiskReconcilePhase | undefined,
  folderName: string | null | undefined,
): string {
  switch (phase) {
    case 'ScanningRoots':
      return folderName ? 'scanning' : 'scanning_without_folder';
    case 'Projecting':
      return 'projecting';
    case 'Finalizing':
      return 'finalizing';
    case 'Completed':
      return 'completed';
    case 'Failed':
      return 'failed';
    case 'DiscoveringRoots':
    default:
      return 'discovering';
  }
}

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
  const [indexingWorkPlan, setIndexingWorkPlan] = useState<IndexingWorkPlan[] | null>(null);
  const [snapshotProgress, setSnapshotProgress] =
    useState<OnboardingIndexingSnapshotProgress | null>(null);
  const indexingSessionRef = useRef<string | null>(null);
  const indexingInFlightRef = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [detectedGames, setDetectedGames] = useState<GameConfig[]>([]);
  const [isDemoPaused, setIsDemoPaused] = useState(false);
  const [shareDiagnostics, setShareDiagnostics] = useState(true);
  const prefersReduced = usePrefersReducedMotion();
  const diskProgress = useOnboardingDiskProgress(isIndexing, detectedGames);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;

    void listen<OnboardingIndexingWorkPlanUpdate>(
      'onboarding_indexing:work_plan',
      ({ payload }) => {
        if (!mounted || payload.session_id !== indexingSessionRef.current) return;
        setIndexingWorkPlan(
          (current) =>
            current?.map((plan) =>
              plan.game_id === payload.work_plan.game_id ? payload.work_plan : plan,
            ) ?? current,
        );
      },
    ).then((stop) => {
      if (mounted) unlisten = stop;
      else stop();
    });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (diskProgress) setSnapshotProgress(null);
  }, [diskProgress]);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;

    void listen<OnboardingIndexingSnapshotProgress>(
      'onboarding_indexing:snapshot_progress',
      ({ payload }) => {
        if (!mounted || !indexingInFlightRef.current) return;
        if (indexingSessionRef.current && indexingSessionRef.current !== payload.session_id) return;
        indexingSessionRef.current = payload.session_id;
        setSnapshotProgress(payload);
      },
    ).then((stop) => {
      if (mounted) unlisten = stop;
      else stop();
    });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    return () => {
      const sessionId = indexingSessionRef.current;
      indexingInFlightRef.current = false;
      indexingSessionRef.current = null;
      if (sessionId) {
        void commands.cancelOnboardingIndexing(sessionId).catch((cancelError) => {
          console.warn(
            '[onboarding] failed to cancel indexing session during unmount:',
            cancelError,
          );
        });
      }
    };
  }, []);

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
    const duplicate = detectedGames.find((g) =>
      pathsEqual(g.instance_path || g.mod_path, game.instance_path || game.mod_path),
    );

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
    let sessionId: string | null = null;
    try {
      setError(null);
      setIsIndexing(true);
      setIndexingWorkPlan(null);
      setSnapshotProgress(null);
      indexingInFlightRef.current = true;
      const total = Math.max(1, games.length);
      const completedDurationsMs: number[] = [];
      setIndexingProgress({
        completed: 0,
        total,
        currentGame: games[0]?.name ?? null,
        completedDurationsMs,
      });

      if (shareDiagnostics) {
        await commands.setTelemetryEnabled(true);
        setFrontendTelemetryEnabled(true);
      }

      // Save the games to DB — this is mandatory
      await commands.saveOnboardingGames(games);
      const session = await commands.beginOnboardingIndexing(games.map((game) => game.id));
      sessionId = session.session_id;
      indexingSessionRef.current = sessionId;
      setIndexingWorkPlan(session.work_plans);
      setSnapshotProgress(null);

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
          await commands.reconcileOnboardingIndexingGame(sessionId, game.id);
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
      setIndexingWorkPlan(null);
    } finally {
      indexingInFlightRef.current = false;
      setSnapshotProgress(null);
      const activeSessionId = sessionId ?? indexingSessionRef.current;
      indexingSessionRef.current = null;
      if (activeSessionId) {
        try {
          await commands.cancelOnboardingIndexing(activeSessionId);
        } catch (cancelError) {
          console.warn('[onboarding] failed to cancel indexing session:', cancelError);
        }
      }
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
            <LiquidSurface
              liquidRole="overlay"
              className="dropdown-content z-[var(--workspace-layer-popover)] mt-2 w-32 rounded-box shadow-xl"
            >
              <ul tabIndex={0} className="menu w-full p-2">
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
            </LiquidSurface>
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
              <h1 className="pb-1 text-3xl font-extrabold text-base-content sm:text-4xl md:text-5xl [@media(max-height:750px)]:text-2xl">
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
          <motion.label
            variants={fade}
            className="mx-auto flex max-w-2xl cursor-pointer items-start gap-3 rounded-xl border border-base-content/10 bg-base-100/45 px-4 py-3 text-left backdrop-blur-sm"
            htmlFor="onboarding-anonymous-diagnostics"
          >
            <input
              id="onboarding-anonymous-diagnostics"
              type="checkbox"
              className="toggle toggle-sm toggle-primary mt-0.5"
              checked={shareDiagnostics}
              onChange={(event) => setShareDiagnostics(event.target.checked)}
            />
            <span>
              <span className="block text-sm font-medium">
                {t('onboarding:welcome.diagnostics.title')}
              </span>
              <span className="mt-0.5 block text-xs leading-5 text-base-content/65">
                {t('onboarding:welcome.diagnostics.description')}
              </span>
            </span>
          </motion.label>
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
                  className="btn btn-primary btn-lg w-full gap-2 border-0 shadow-lg shadow-primary/20 transition-shadow duration-300 hover:shadow-xl hover:shadow-primary/30 sm:gap-3 [@media(max-height:750px)]:min-h-12 max-sm:min-h-14! [@media(max-height:750px)]:h-12 max-sm:h-14! [@media(max-height:750px)]:px-4"
                  onClick={handleAutoDetect}
                >
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
          <Loader2 className="w-16 h-16 text-primary animate-spin motion-reduce:animate-none mx-auto" />
          <div>
            <h2 className="text-2xl font-semibold">{t('onboarding:scanning.title')}</h2>
            <p className="text-base-content/60 mt-2">{t('onboarding:scanning.subtitle')}</p>
          </div>
          {/* Shimmer placeholder cards (EC-1.07) */}
          <div className="w-80 mx-auto space-y-3">
            {[1, 2, 3].map((i) => (
              <div
                key={i}
                className="h-16 rounded-xl bg-base-200 animate-pulse motion-reduce:animate-none"
              />
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
    const snapshotGameIndex = snapshotProgress
      ? detectedGames.findIndex((game) => game.id === snapshotProgress.game_id)
      : -1;
    const activeGameIndex =
      snapshotGameIndex >= 0
        ? snapshotGameIndex
        : Math.min(progress.completed, Math.max(0, progress.total - 1));
    const activeGame = detectedGames[activeGameIndex];
    const activeDiskProgress =
      diskProgress?.current.game_id === activeGame?.id ? diskProgress : null;
    const overallProgress = activeDiskProgress
      ? calculateOverallIndexingProgress(
          activeDiskProgress.current,
          detectedGames.map((game) => game.id),
          indexingWorkPlan ?? [],
          activeDiskProgress.completedRootsByGame,
        )
      : null;
    // Snapshot creation precedes every game projection, so its per-game
    // completion cannot be added to the sequential reconcile percentage
    // without making the global bar move backwards. Keep the global bar at
    // zero while reporting the active snapshot game and stage below.
    const snapshotPercent = snapshotProgress ? 0 : null;
    const percent =
      snapshotPercent ??
      overallProgress?.percent ??
      Math.round((progress.completed / progress.total) * 100);
    const remaining = snapshotProgress
      ? null
      : estimatedRemainingMs(progress, activeDiskProgress?.current.eta_ms);
    const gameNumber = (overallProgress?.gameIndex ?? activeGameIndex) + 1;
    const gameName = activeGame?.name ?? progress.currentGame ?? '';
    const step = overallProgress?.step ?? 1;
    const activityKey = snapshotProgress
      ? snapshotProgress.phase === 'Scanning'
        ? 'scanning'
        : 'discovering'
      : activityTranslationKey(activeDiskProgress?.current.phase, overallProgress?.folderName);

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
            <h2 className="text-2xl font-bold text-base-content">
              {t('onboarding:indexing.title')}
            </h2>
          </div>

          {/* Progress Section */}
          <div
            className="bg-base-200/50 rounded-2xl p-6 shadow-sm border border-base-content/5 space-y-5"
            aria-live="polite"
          >
            <div className="flex items-baseline justify-between">
              <span className="text-sm font-medium text-base-content/70">
                {t('onboarding:indexing.overall_progress')}
              </span>
              <span className="text-2xl font-semibold tracking-tight text-base-content">
                {percent}
                <span className="ml-0.5 text-sm font-medium text-base-content/50">%</span>
              </span>
            </div>

            {/* Custom Animated Bar */}
            <div
              className="h-2 w-full bg-base-300/50 rounded-full overflow-hidden shadow-inner relative"
              role="progressbar"
              aria-label={t('onboarding:indexing.progress_label')}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={percent}
            >
              <div
                className="absolute bottom-0 left-0 top-0 bg-primary transition-[width] duration-150 ease-out motion-reduce:transition-none"
                style={{ width: `${percent}%` }}
              >
                <div className="absolute inset-0 bg-white/10" />
              </div>
            </div>

            {/* ETA */}
            <p className="text-left text-xs font-medium text-base-content/50">
              {remaining === null
                ? t('onboarding:indexing.estimating')
                : remaining < 2000
                  ? t('onboarding:indexing.finishing_up')
                  : t('onboarding:indexing.estimated_remaining', {
                      duration: formatEstimatedDuration(remaining),
                    })}
            </p>

            <div className="border-t border-base-content/10 pt-5 text-left space-y-1.5">
              <p className="text-sm font-semibold text-base-content">
                {t('onboarding:indexing.game_progress', {
                  current: gameNumber,
                  total: progress.total,
                  game: gameName,
                })}
              </p>
              <p className="text-sm text-base-content/65">
                {t(`onboarding:indexing.activity.${activityKey}`, {
                  folder: overallProgress?.folderName,
                  step,
                  total: INDEXING_STEP_COUNT,
                })}
              </p>
            </div>
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
