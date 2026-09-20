import { useState } from 'react';
import { Gamepad2, Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { GAME_OPTIONS, useActiveGame, type GameConfig } from '@/entities/game';
import { useGameSwitch } from '@/features/workspace-runtime';
import { useAppStore } from '@/app/store';
import { LiquidSurface } from '@/shared/ui/liquid';

interface GameSelectorProps {
  compact?: boolean;
}

export default function GameSelector({ compact = false }: GameSelectorProps) {
  const { t } = useTranslation('layout');
  const { activeGame, games = [], isLoading } = useActiveGame();
  const { switchGame } = useGameSwitch();
  const activeActivation = useAppStore((state) =>
    activeGame?.id ? state.gameActivationByGame?.[activeGame.id] : undefined,
  );
  const [isSwitching, setIsSwitching] = useState(false);

  // Derive display info from active game
  const activeLabel = activeGame?.name ?? t('game_selector.select_game');
  const activeShort =
    GAME_OPTIONS.find((o) => o.value === (activeGame?.game_type as unknown as string))
      ?.label.split(' (')[0]
      .split(' ')
      .map((w: string) => w[0])
      .join('') ?? '-';
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

  const handleSwitchGame = async (gameId: string) => {
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
  };

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

  return (
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
                  onClick={() => void handleSwitchGame(game.id)}
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
  );
}
