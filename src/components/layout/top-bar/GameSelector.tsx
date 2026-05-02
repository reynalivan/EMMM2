import { Gamepad2, Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useGameSwitch } from '../../../hooks/useObjectQueries';
import { GAME_OPTIONS, GAME_TYPE_COLORS, type GameConfig } from '../../../types/game';

export default function GameSelector() {
  const { t } = useTranslation('layout');
  const { activeGame, games = [], isLoading } = useActiveGame();
  const { switchGame } = useGameSwitch();

  // Derive display info from active game
  const activeLabel = activeGame?.name ?? t('game_selector.select_game');
  const activeShort =
    GAME_OPTIONS.find((o) => o.value === (activeGame?.game_type as unknown as string))
      ?.label.split(' (')[0]
      .split(' ')
      .map((w: string) => w[0])
      .join('') ?? '—';
  const activeBadge = activeGame?.game_type
    ? (GAME_TYPE_COLORS[activeGame.game_type] ?? 'badge-ghost')
    : 'badge-ghost';

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-base-200/30 border border-base-content/10">
        <div className="loading loading-spinner loading-xs text-primary" />
        <span className="text-sm text-base-content/70">{t('game_selector.loading')}</span>
      </div>
    );
  }

  if (games.length === 0) {
    return (
      <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-base-200/30 border border-warning/30">
        <Plus size={14} className="text-warning" />
        <span className="text-sm text-warning/80">{t('game_selector.add_game')}</span>
      </div>
    );
  }

  return (
    <div className="dropdown dropdown-bottom">
      <div
        tabIndex={0}
        role="button"
        className="flex items-center gap-1.5 md:gap-2 px-2 md:px-3 py-1.5 rounded-full bg-base-200/30 border border-base-content/10 hover:border-primary/50 hover:bg-base-200/50 transition-all cursor-pointer group"
      >
        <div className="p-1 rounded-full bg-base-100/50 text-primary group-hover:text-primary-content transition-colors">
          <Gamepad2 size={12} className="md:w-3.5 md:h-3.5" />
        </div>
        <span className="hidden sm:inline text-sm font-medium text-base-content group-hover:text-primary transition-colors">
          {activeLabel}
        </span>
        <span className="sm:hidden font-bold text-sm text-base-content/90">{activeShort}</span>
        {activeGame && (
          <span className={`badge badge-xs ${activeBadge} opacity-90`}>{activeGame.game_type}</span>
        )}
        <span className="opacity-50 text-[10px] group-hover:opacity-100 transition-opacity">▼</span>
      </div>
      <ul
        tabIndex={0}
        className="dropdown-content z-1 menu p-2 shadow-xl bg-base-100/90 backdrop-blur-xl rounded-box w-56 border border-base-content/10 mt-2"
      >
        {games.map((game: GameConfig) => {
          const isActive = activeGame?.id === game.id;
          const badgeColor = GAME_TYPE_COLORS[game.game_type] ?? 'badge-ghost';

          return (
            <li key={game.id}>
              <button
                onClick={() => switchGame(game.id)}
                className={`hover:bg-base-content/10 flex items-center justify-between ${
                  isActive ? 'text-primary font-bold bg-primary/10' : 'text-base-content/70'
                }`}
              >
                <span>{game.name}</span>
                <span className={`badge badge-xs ${badgeColor}`}>{game.game_type}</span>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
