import { Gamepad2, Plus } from 'lucide-react';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useGameSwitch } from '../../../hooks/useObjects';
import { GAME_OPTIONS, GAME_TYPE_COLORS } from '../../../types/game';

export default function GameSelector() {
  const { activeGame, games = [], isLoading } = useActiveGame();
  const { switchGame } = useGameSwitch();

  // Derive display info from active game
  const activeLabel = activeGame?.name ?? 'Select Game';
  const activeShort =
    GAME_OPTIONS.find((o) => o.value === activeGame?.game_type)
      ?.label.split(' (')[0]
      .split(' ')
      .map((w) => w[0])
      .join('') ?? '—';
  const activeBadge = activeGame?.game_type
    ? (GAME_TYPE_COLORS[activeGame.game_type] ?? 'badge-ghost')
    : 'badge-ghost';

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-base-200/30 border border-white/5">
        <div className="loading loading-spinner loading-xs text-primary" />
        <span className="text-sm text-white/50">Loading...</span>
      </div>
    );
  }

  if (games.length === 0) {
    return (
      <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-base-200/30 border border-warning/30">
        <Plus size={14} className="text-warning" />
        <span className="text-sm text-warning/80">Add Game</span>
      </div>
    );
  }

  return (
    <div className="dropdown dropdown-bottom">
      <div
        tabIndex={0}
        role="button"
        className="flex items-center gap-1.5 md:gap-2 px-2 md:px-3 py-1.5 rounded-full bg-base-200/30 border border-white/5 hover:border-primary/50 hover:bg-base-200/50 transition-all cursor-pointer group"
      >
        <div className="p-1 rounded-full bg-base-100/50 text-primary group-hover:text-white transition-colors">
          <Gamepad2 size={12} className="md:w-[14px] md:h-[14px]" />
        </div>
        <span className="hidden sm:inline text-sm font-medium text-white/90 group-hover:text-white">
          {activeLabel}
        </span>
        <span className="sm:hidden font-bold text-sm text-white/90">{activeShort}</span>
        {activeGame && (
          <span className={`badge badge-xs ${activeBadge} opacity-60`}>{activeGame.game_type}</span>
        )}
        <span className="opacity-30 text-[10px] group-hover:opacity-100 transition-opacity">▼</span>
      </div>
      <ul
        tabIndex={0}
        className="dropdown-content z-1 menu p-2 shadow-xl bg-base-100/80 backdrop-blur-xl rounded-box w-56 border border-white/10 mt-2"
      >
        {games.map((game) => {
          const isActive = activeGame?.id === game.id;
          const badgeColor = GAME_TYPE_COLORS[game.game_type] ?? 'badge-ghost';

          return (
            <li key={game.id}>
              <button
                onClick={() => switchGame(game.id)}
                className={`hover:bg-white/5 flex items-center justify-between ${
                  isActive ? 'text-primary font-bold bg-primary/10' : 'text-white/70'
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
