import { CheckCircle2, Plus, ArrowRight } from 'lucide-react';
import type { GameConfig } from '../../types/game';
import { GAME_TYPE_COLORS } from '../../types/game';

interface Props {
  games: GameConfig[];
  onContinue: () => void;
  onAddMore: () => void;
}

export default function AutoDetectResult({ games, onContinue, onAddMore }: Props) {
  return (
    <div className="min-h-screen bg-base-100 flex items-center justify-center p-6">
      <div className="max-w-lg w-full space-y-8 text-center">
        {/* Success Icon */}
        <div className="space-y-3">
          <CheckCircle2 className="w-16 h-16 text-success mx-auto" />
          <h2 className="text-3xl font-bold">
            {games.length === 1 ? 'Game Found!' : `${games.length} Games Found!`}
          </h2>
          <p className="text-base-content/60">
            The following games have been configured and are ready to manage.
          </p>
        </div>

        {/* Game Cards */}
        <div className="space-y-3">
          {games.map((game) => (
            <div key={game.id} className="card card-border bg-base-200 card-sm">
              <div className="card-body flex-row items-center gap-4">
                <div className="flex-1 text-left">
                  <h3 className="font-semibold text-lg">{game.name}</h3>
                  <p className="text-base-content/50 text-xs truncate max-w-xs">{game.path}</p>
                </div>
                <span className={`badge ${GAME_TYPE_COLORS[game.game_type] || 'badge-neutral'}`}>
                  {game.game_type}
                </span>
              </div>
            </div>
          ))}
        </div>

        {/* Actions */}
        <div className="flex gap-3">
          <button id="btn-add-more" className="btn btn-outline flex-1" onClick={onAddMore}>
            <Plus className="w-4 h-4" />
            Add Another
          </button>
          <button id="btn-continue" className="btn btn-primary flex-1" onClick={onContinue}>
            Continue
            <ArrowRight className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
