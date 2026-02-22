import { CheckCircle2, Plus, ArrowRight, Trash2 } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import type { GameConfig } from '../../types/game';
import { GAME_TYPE_COLORS } from '../../types/game';

interface Props {
  games: GameConfig[];
  onContinue: () => void;
  onAddMore: () => void;
  onRemoveGame: (gameId: string) => void;
}

export default function AutoDetectResult({ games, onContinue, onAddMore, onRemoveGame }: Props) {
  return (
    <div className="min-h-screen bg-base-100 flex items-center justify-center p-6">
      <div className="max-w-lg w-full space-y-8 text-center">
        {/* Success Icon */}
        <div className="space-y-3">
          <motion.div
            initial={{ scale: 0, rotate: -180 }}
            animate={{ scale: 1, rotate: 0 }}
            transition={{
              type: 'spring',
              stiffness: 260,
              damping: 20,
              duration: 0.6,
            }}
            className="flex justify-center"
          >
            <CheckCircle2 className="w-16 h-16 text-success" strokeWidth={2.5} />
          </motion.div>
          <motion.h2
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
            className="text-3xl font-bold"
          >
            {games.length === 1 ? 'Game Found!' : `${games.length} Games Found!`}
          </motion.h2>
          <motion.p
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.3 }}
            className="text-base-content/60"
          >
            The following games have been detected and are ready to manage.
          </motion.p>
        </div>

        {/* Game Cards */}
        <div className="space-y-3 relative">
          <AnimatePresence initial={false}>
            {games.map((game, i) => (
              <motion.div
                key={game.id}
                initial={{ opacity: 0, x: -20, height: 0 }}
                animate={{ opacity: 1, x: 0, height: 'auto' }}
                exit={{ opacity: 0, scale: 0.9, height: 0, marginBottom: 0 }}
                transition={{ duration: 0.2, delay: i * 0.05 }}
                className="card card-border bg-base-200 card-sm overflow-hidden"
              >
                <div className="card-body flex-row items-center gap-4 py-4">
                  <div className="flex-1 text-left">
                    <h3 className="font-semibold text-lg">{game.name}</h3>
                    <p
                      className="text-base-content/50 text-xs truncate max-w-[200px] sm:max-w-xs"
                      title={game.game_exe}
                    >
                      {game.game_exe}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span
                      className={`badge ${GAME_TYPE_COLORS[game.game_type] || 'badge-neutral'}`}
                    >
                      {game.game_type}
                    </span>
                    <button
                      className="btn btn-ghost btn-square btn-sm text-error/70 hover:text-error hover:bg-error/10"
                      onClick={() => onRemoveGame(game.id)}
                      title="Remove from detection"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              </motion.div>
            ))}
          </AnimatePresence>
        </div>

        {/* Actions */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="flex gap-3 pt-2"
        >
          <button id="btn-add-more" className="btn btn-outline flex-1" onClick={onAddMore}>
            <Plus className="w-4 h-4" />
            Add Another
          </button>
          <button id="btn-continue" className="btn btn-primary flex-1" onClick={onContinue}>
            Confirm
            <ArrowRight className="w-4 h-4" />
          </button>
        </motion.div>
      </div>
    </div>
  );
}
