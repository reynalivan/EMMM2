import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../lib/bindings';
import { open } from '@tauri-apps/plugin-dialog';
import { Search, FolderOpen, ChevronRight, Loader2, AlertCircle } from 'lucide-react';
import { motion } from 'motion/react';
import type { GameConfig } from '../../types/game';
import { pathsEqual } from '../../lib/pathKey';
import { scanService } from '../../lib/services/scanService';
import { ManualSetupForm } from './ManualSetupForm';
import { AutoDetectResult } from './AutoDetectResult';
import AuroraBackground from '../welcome/AuroraBackground';
import SmartDemoStrip from '../welcome/SmartDemoStrip';
import AnimatedLogo from '../welcome/AnimatedLogo';

type Screen = 'welcome' | 'auto-detect' | 'manual' | 'result';

export default function WelcomeScreen({
  onComplete,
}: {
  onComplete: (games: GameConfig[]) => void;
}) {
  const { t } = useTranslation(['welcome', 'onboarding']);
  const [view, setView] = useState<Screen>('welcome');
  const [isScanning, setIsScanning] = useState(false);
  const [isIndexing, setIsIndexing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [detectedGames, setDetectedGames] = useState<GameConfig[]>([]);
  const [isDemoPaused, setIsDemoPaused] = useState(false);

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

      const games = await commands.autoDetectGames({
        rootPath: selectedPath,
      });

      setDetectedGames(games);
      setView('result');
    } catch (err) {
      setError(String(err));
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

      // Save the games to DB — this is mandatory
      await commands.saveOnboardingGames({ games });

      // Best-effort sync: try to import mods from each game folder.
      for (const game of games) {
        try {
          await scanService.syncDatabase(game.id, game.name, game.game_type, game.mod_path);
        } catch (scanErr) {
          console.warn(
            `[onboarding] syncDatabase failed for "${game.name}", will retry via watcher:`,
            scanErr,
          );
        }
      }

      onComplete(games);
    } catch (err) {
      // Only the save_onboarding_games failure is a hard blocker
      setError(String(err));
      setIsIndexing(false);
    }
  };

  // == Welcome View ==
  if (view === 'welcome') {
    return (
      <div className="h-screen w-full bg-transparent overflow-y-auto overflow-x-hidden relative flex flex-col items-center justify-center p-6 z-0">
        <div className="fixed inset-0 z-[-1]">
          <AuroraBackground />
        </div>

        <div className="max-w-4xl w-full text-center space-y-6 z-10 py-6 my-auto origin-center [@media(max-height:800px)]:scale-95 [@media(max-height:750px)]:scale-90 [@media(max-height:700px)]:scale-[0.85] [@media(max-height:650px)]:scale-[0.8] transition-transform duration-500 ease-out">
          {/* Logo & Title */}
          <div className="flex flex-col [@media(max-height:750px)]:flex-row items-center justify-center gap-3 [@media(max-height:750px)]:gap-5">
            <div className="mx-auto [@media(max-height:750px)]:mx-0 w-16 h-16 sm:w-20 sm:h-20 [@media(max-height:750px)]:w-14 [@media(max-height:750px)]:h-14 flex items-center justify-center shrink-0 text-base-content hover:text-primary transition-colors duration-300">
              <AnimatedLogo />
            </div>
            <motion.div
              initial={{ opacity: 0, y: -20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.7, ease: 'easeOut' }}
              className="[@media(max-height:750px)]:text-left flex flex-col justify-center"
            >
              <h1 className="text-3xl sm:text-4xl md:text-5xl [@media(max-height:750px)]:text-2xl font-extrabold bg-linear-to-r from-primary to-secondary bg-clip-text text-transparent drop-shadow-sm pb-1">
                {t('onboarding:welcome.title')}
              </h1>
              <p className="text-base-content/60 text-base md:text-lg [@media(max-height:750px)]:text-xs font-medium tracking-wide mt-1">
                {t('onboarding:welcome.subtitle')}
              </p>
            </motion.div>
          </div>

          <SmartDemoStrip isPausedFromParent={isDemoPaused} />

          {/* Error Alert */}
          {error && (
            <div role="alert" className="alert alert-error alert-soft">
              <AlertCircle className="w-5 h-5" />
              <span>{error}</span>
            </div>
          )}

          {/* CTA Buttons */}
          <div className="max-w-2xl mx-auto flex flex-col [@media(max-height:750px)]:flex-row max-sm:flex-col! [@media(max-height:750px)]:w-full gap-4">
            <div
              className="w-full [@media(max-height:750px)]:flex-1 max-sm:flex-none tooltip tooltip-bottom flex min-w-0"
              data-tip={t('onboarding:welcome.auto_detect_tip')}
            >
              <motion.button
                whileHover="hover"
                whileTap="tap"
                variants={{
                  hover: { y: -2, boxShadow: '0 8px 30px -5px var(--color-primary)' },
                  tap: { scale: 0.98 },
                }}
                onHoverStart={() => setIsDemoPaused(true)}
                onHoverEnd={() => setIsDemoPaused(false)}
                onFocus={() => setIsDemoPaused(true)}
                onBlur={() => setIsDemoPaused(false)}
                id="btn-auto-detect"
                className="btn btn-primary btn-lg w-full gap-2 sm:gap-3 overflow-hidden outline-offset-2 [@media(max-height:750px)]:min-h-12 max-sm:min-h-14! [@media(max-height:750px)]:h-12 max-sm:h-14! [@media(max-height:750px)]:px-4"
                onClick={handleAutoDetect}
              >
                <Search className="w-5 h-5 shrink-0" />
                <span className="flex-1 text-left truncate [@media(max-height:750px)]:text-sm">
                  {t('onboarding:welcome.auto_detect')}
                </span>
                <motion.div
                  variants={{ hover: { x: 6 } }}
                  transition={{ type: 'spring' }}
                  className="flex items-center shrink-0"
                >
                  <ChevronRight className="w-5 h-5" />
                </motion.div>
              </motion.button>
            </div>

            <motion.button
              whileHover="hover"
              whileTap="tap"
              variants={{
                hover: {
                  y: -2,
                  boxShadow:
                    '0 8px 30px -5px color-mix(in srgb, var(--color-base-content), transparent 50%)',
                },
                tap: { scale: 0.98 },
              }}
              onHoverStart={() => setIsDemoPaused(true)}
              onHoverEnd={() => setIsDemoPaused(false)}
              onFocus={() => setIsDemoPaused(true)}
              onBlur={() => setIsDemoPaused(false)}
              id="btn-manual-setup"
              className="btn btn-outline btn-lg w-full [@media(max-height:750px)]:flex-1 max-sm:flex-none! gap-2 sm:gap-3 outline-offset-2 border-base-content/20 hover:bg-base-content hover:text-base-100 [@media(max-height:750px)]:min-h-12 max-sm:min-h-14! [@media(max-height:750px)]:h-12 max-sm:h-14! [@media(max-height:750px)]:px-4 min-w-0"
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
                variants={{ hover: { x: 6 } }}
                transition={{ type: 'spring' }}
                className="flex items-center shrink-0"
              >
                <ChevronRight className="w-5 h-5" />
              </motion.div>
            </motion.button>
          </div>

          <p className="text-base-content/50 text-sm font-medium">
            {t('onboarding:welcome.description')}
          </p>
        </div>
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
    return (
      <div className="min-h-screen bg-base-100 flex items-center justify-center">
        <div className="text-center space-y-6">
          <Loader2 className="w-16 h-16 text-primary animate-spin mx-auto" />
          <div>
            <h2 className="text-2xl font-semibold">{t('onboarding:indexing.title')}</h2>
            <p className="text-base-content/60 mt-2">{t('onboarding:indexing.subtitle')}</p>
          </div>
        </div>
      </div>
    );
  }

  // == Manual Setup View ==
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

  // == Result View ==
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
        onRemoveGame={handleRemoveGame}
      />
    );
  }

  return null;
}
