import { useEffect, useId, useRef, useState } from 'react';
import { AlertCircle, Gamepad2, Loader2, Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { GAME_OPTIONS, useActiveGame, type GameConfig } from '@/entities/game';
import { useGameSwitch } from '@/features/workspace-runtime';
import { useAppStore } from '@/app/store';
import { LiquidSurface } from '@/shared/ui/liquid';
import { useDialogSync } from '@/shared/lib/hooks/useDialogSync';
import {
  useBackgroundIndexingStatus,
  type BackgroundIndexingStatusState,
} from '@/pages/onboarding/hooks/useBackgroundIndexingStatus';

interface GameSelectorProps {
  compact?: boolean;
  backgroundIndexingStatus?: BackgroundIndexingStatusState;
}

export default function GameSelector(props: GameSelectorProps) {
  if (props.backgroundIndexingStatus) {
    return <GameSelectorContent {...props} backgroundIndexingStatus={props.backgroundIndexingStatus} />;
  }
  return <GameSelectorWithOwnBackgroundStatus {...props} />;
}

function GameSelectorWithOwnBackgroundStatus({ compact = false }: GameSelectorProps) {
  const backgroundIndexingStatus = useBackgroundIndexingStatus();
  return <GameSelectorContent compact={compact} backgroundIndexingStatus={backgroundIndexingStatus} />;
}

function GameSelectorContent({
  compact = false,
  backgroundIndexingStatus,
}: {
  compact?: boolean;
  backgroundIndexingStatus: BackgroundIndexingStatusState;
}) {
  const { t } = useTranslation('layout');
  const { activeGame, games = [], isLoading } = useActiveGame();
  const { switchGame } = useGameSwitch();
  const activeActivation = useAppStore((state) =>
    activeGame?.id ? state.gameActivationByGame?.[activeGame.id] : undefined,
  );
  const {
    gamesById: backgroundIndexingByGame,
    isLoaded: backgroundStatusLoaded,
    loadError: backgroundStatusLoadError,
    refresh: refreshBackgroundStatus,
  } = backgroundIndexingStatus;
  const [isSwitching, setIsSwitching] = useState(false);
  const [pendingGame, setPendingGame] = useState<GameConfig | null>(null);
  const indexingDialogRef = useRef<HTMLDialogElement>(null);
  const dialogTitleId = useId();
  const dialogDescriptionId = useId();
  const pendingIndexingStatus = pendingGame
    ? backgroundIndexingByGame.get(pendingGame.id)
    : undefined;
  useDialogSync(indexingDialogRef, pendingGame !== null);

  // Derive display info from active game
  const activeLabel = activeGame?.name ?? t('game_selector.select_game');
  const activeShort =
    GAME_OPTIONS.find((o) => o.value === (activeGame?.game_type as unknown as string))
      ?.label.split(' (')[0]
      .split(' ')
      .map((w: string) => w[0])
      .join('') ?? '-';

  useEffect(() => {
    if (!pendingGame) return;
    const isReadyForSwitch =
      pendingIndexingStatus?.phase === 'Ready' ||
      (backgroundStatusLoaded && !backgroundStatusLoadError && !pendingIndexingStatus);
    if (!isReadyForSwitch) return;

    const gameId = pendingGame.id;
    setPendingGame(null);
    void handleSwitchGame(gameId);
  }, [
    backgroundStatusLoaded,
    backgroundStatusLoadError,
    pendingGame,
    pendingIndexingStatus?.phase,
    handleSwitchGame,
  ]);

  const handleGameSelection = (game: GameConfig) => {
    const backgroundStatus = backgroundIndexingByGame.get(game.id);
    const canSwitch =
      backgroundStatus?.phase === 'Ready' ||
      (backgroundStatusLoaded && !backgroundStatusLoadError && !backgroundStatus);
    if (!canSwitch) {
      setPendingGame(game);
      return;
    }
    void handleSwitchGame(game.id);
  };

  if (isLoading || isSwitching) {
    if (compact) {
      return (
        <button
          type="button"
          className="btn btn-ghost btn-sm btn-square"
          disabled
          aria-label={t('game_selector.loading')}
        >
          <span className="loading loading-spinner loading-xs text-primary" />
        </button>
      );
    }
    return (
      <div className="flex min-w-36 items-center px-2 py-1">
        <div className="flex min-w-0 flex-col">
          <span className="text-sm font-bold leading-none tracking-tight text-base-content">
            {t('app.name')}
          </span>
          <span className="mt-0.5 flex items-center gap-1 text-[10px] text-base-content/55">
            <span className="loading loading-spinner loading-xs text-primary" />
            {t('game_selector.loading')}
          </span>
        </div>
      </div>
    );
  }

  async function handleSwitchGame(gameId: string) {
    const retryingActive =
      gameId === activeGame?.id &&
      (activeActivation?.phase === 'failed' || activeActivation?.phase === 'source_unavailable');
    if (isSwitching || (gameId === activeGame?.id && !retryingActive)) return;

    setIsSwitching(true);
    try {
      await switchGame(gameId);
    } finally {
      setIsSwitching(false);
    }
  }

  if (games.length === 0) {
    if (compact) {
      return (
        <button
          type="button"
          className="btn btn-ghost btn-sm btn-square"
          disabled
          aria-label={t('game_selector.add_game')}
        >
          <Plus size={16} aria-hidden="true" />
        </button>
      );
    }
    return (
      <div className="flex min-w-36 items-center px-2 py-1">
        <div className="flex min-w-0 flex-col">
          <span className="text-sm font-bold leading-none tracking-tight text-base-content">
            {t('app.name')}
          </span>
          <span className="mt-0.5 flex items-center gap-1 text-[10px] text-warning/80">
            <Plus size={12} aria-hidden="true" />
            {t('game_selector.add_game')}
          </span>
        </div>
      </div>
    );
  }

  const needsAttention =
    pendingIndexingStatus?.phase === 'NeedsAttention' || pendingIndexingStatus?.phase === 'Failed';
  const statusUnavailable = !pendingIndexingStatus && backgroundStatusLoadError;
  const dialogTitle = needsAttention
    ? t('game_selector.indexing.attention_title', { game: pendingGame?.name })
    : statusUnavailable
      ? t('game_selector.indexing.status_unavailable_title', { game: pendingGame?.name })
      : t('game_selector.indexing.dialog_title', { game: pendingGame?.name });
  const dialogDescription = needsAttention
    ? t('game_selector.indexing.attention_description')
    : statusUnavailable
      ? t('game_selector.indexing.status_unavailable_description')
      : t('game_selector.indexing.dialog_description');

  const openAndRecheckPendingGame = () => {
    const gameId = pendingGame?.id;
    if (!gameId) return;
    setPendingGame(null);
    void handleSwitchGame(gameId);
  };

  return (
    <>
      <div className="dropdown dropdown-bottom">
      <button
        type="button"
        className={
          compact
            ? 'btn btn-ghost btn-sm btn-square text-base-content/75 hover:text-base-content'
            : 'group flex min-w-36 max-w-52 cursor-pointer items-center gap-2 rounded-md px-2 py-1 text-left text-base-content/75 transition-[background-color,color] duration-150 hover:bg-base-content/5 hover:text-base-content md:px-3'
        }
        aria-label={t('game_selector.select_game')}
      >
        <Gamepad2
          size={15}
          className="shrink-0 text-base-content/45 transition-colors group-hover:text-primary"
        />
        {!compact && (
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="text-sm font-bold leading-none tracking-tight text-base-content">
              {t('app.name')}
            </span>
            <span className="mt-0.5 truncate text-[10px] font-medium text-base-content/55 group-hover:text-base-content/75">
              <span className="hidden sm:inline">{activeLabel}</span>
              <span className="sm:hidden">{activeShort}</span>
            </span>
          </span>
        )}
        {!compact && (
          <span className="text-[10px] opacity-50 transition-opacity group-hover:opacity-100">
            ▼
          </span>
        )}
      </button>
      <LiquidSurface
        liquidRole="overlay"
        className="dropdown-content z-[var(--workspace-layer-popover)] mt-2 w-56 rounded-box shadow-lg"
      >
        <ul tabIndex={0} className="menu w-full p-2">
          {games.map((game: GameConfig) => {
            const isActive = activeGame?.id === game.id;

            return (
              <li key={game.id}>
                <button
                  onClick={() => handleGameSelection(game)}
                  disabled={isSwitching}
                  className={`hover:bg-base-content/10 ${
                    isActive ? 'text-primary font-bold bg-primary/10' : 'text-base-content/70'
                  }`}
                >
                  <span>{game.name}</span>
                  {isActive &&
                    (activeActivation?.phase === 'failed' ||
                      activeActivation?.phase === 'source_unavailable') && (
                      <span className="ml-auto text-xs font-medium text-warning">
                        {t('game_selector.retry')}
                      </span>
                    )}
                </button>
              </li>
            );
          })}
        </ul>
      </LiquidSurface>
      </div>

      <dialog
        ref={indexingDialogRef}
        className="modal"
        aria-labelledby={dialogTitleId}
        aria-describedby={dialogDescriptionId}
        onCancel={(event) => {
          event.preventDefault();
          setPendingGame(null);
        }}
        onClose={() => setPendingGame(null)}
      >
        <div className="modal-box max-w-sm border border-base-content/10 bg-base-100 p-6 shadow-xl">
          <div className="flex items-start gap-3">
            {needsAttention || statusUnavailable ? (
              <AlertCircle
                size={20}
                className="mt-0.5 shrink-0 text-warning"
                aria-hidden="true"
              />
            ) : (
              <Loader2
                size={20}
                className="mt-0.5 shrink-0 animate-spin text-primary motion-reduce:animate-none"
                aria-hidden="true"
              />
            )}
            <div className="min-w-0 space-y-2">
              <h2 id={dialogTitleId} className="text-base font-semibold">
                {dialogTitle}
              </h2>
              <p
                id={dialogDescriptionId}
                className="text-sm leading-6 text-base-content/70"
              >
                {dialogDescription}
              </p>
              <p role="status" aria-live="polite" className="text-xs font-medium text-base-content/60">
                {statusUnavailable
                  ? t('game_selector.indexing.status_unavailable')
                  : t(`game_selector.indexing.phase.${pendingIndexingStatus?.phase ?? 'Queued'}`)}
              </p>
            </div>
          </div>
          <div className="mt-6 flex justify-end">
            {needsAttention && (
              <button type="button" className="btn btn-primary btn-sm" onClick={openAndRecheckPendingGame}>
                {t('game_selector.indexing.open_and_recheck')}
              </button>
            )}
            {statusUnavailable && (
              <button
                type="button"
                className="btn btn-primary btn-sm"
                onClick={() => void refreshBackgroundStatus()}
              >
                {t('game_selector.indexing.retry_status')}
              </button>
            )}
            <form method="dialog">
              <button type="submit" className="btn btn-ghost btn-sm">
                {t('game_selector.indexing.stay')}
              </button>
            </form>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop">
          <button type="submit" aria-label={t('game_selector.indexing.stay')}>
            {t('game_selector.indexing.stay')}
          </button>
        </form>
      </dialog>
    </>
  );
}
