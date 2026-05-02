import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'motion/react';
import { ArrowLeft, Trash2, AlertTriangle, Plus, Check } from 'lucide-react';
import { getGameTypeKey as getGameKey, type GameConfig } from '../../types/game';

interface AutoDetectResultProps {
  games: GameConfig[];
  onBack: () => void;
  onConfirm: (games: GameConfig[]) => void;
  onRemoveGame: (gameId: string) => void;
}

export function AutoDetectResult({
  games,
  onBack,
  onConfirm,
  onRemoveGame,
}: AutoDetectResultProps) {
  const { t } = useTranslation('onboarding');
  const [confirmedGames, setConfirmedGames] = useState<GameConfig[]>(games);

  const handleRemove = (gameId: string) => {
    setConfirmedGames((prev) => prev.filter((g) => g.id !== gameId));
    onRemoveGame(gameId);
  };

  const getGameTypeKey = (type?: string | number) => {
    if (type === undefined || type === null) return '';
    const typeStr = String(type);
    const translationKey = `manual_setup.game_types.${typeStr.toLowerCase()}`;
    const translatedValue = t(translationKey);
    return translatedValue === translationKey ? typeStr : translatedValue;
  };

  return (
    <div className="min-h-screen bg-base-100 flex flex-col p-6 items-center justify-center relative">
      <div className="flex items-center gap-4 mb-4 w-full max-w-2xl">
        <button
          onClick={onBack}
          className="btn btn-ghost btn-sm gap-2"
          aria-label={t('result.back')}
        >
          <ArrowLeft className="w-4 h-4" />
          {t('result.back_to_welcome')}
        </button>
      </div>

      <motion.div
        initial={{ scale: 0, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ type: 'spring', damping: 12, stiffness: 200, delay: 0.1 }}
        className="mb-6 relative"
      >
        <div className="absolute inset-0 bg-success/20 blur-2xl rounded-full scale-150 animate-pulse" />
        <div className="w-20 h-20 rounded-full bg-linear-to-br from-success/20 to-success/40 border-2 border-success/30 flex items-center justify-center relative z-10 shadow-lg shadow-success/10">
          <Check size={40} className="text-success drop-shadow-[0_0_8px_rgba(34,197,94,0.5)]" />
        </div>
      </motion.div>

      <div className="text-center mb-10">
        <h2 className="text-3xl font-bold bg-github-gradient bg-clip-text text-transparent mb-2">
          {t('result.title', { count: confirmedGames.length })}
        </h2>
        <p className="text-base-content/60">{t('result.subtitle')}</p>
      </div>

      <div className="w-full max-w-2xl bg-base-200/50 rounded-2xl border border-base-content/5 p-6 backdrop-blur-sm overflow-y-auto max-h-[50vh]">
        <div className="space-y-4">
          {confirmedGames.map((game) => (
            <motion.div
              layout
              key={game.id}
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 10 }}
              className="bg-base-100 rounded-xl p-4 border border-base-content/10 flex flex-col gap-3 group"
            >
              <div className="flex items-center justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <h3 className="font-bold text-lg truncate group-hover:text-primary transition-colors">
                    {game.name}
                  </h3>
                  <code className="text-[10px] text-base-content/40 truncate block">
                    {game.game_exe}
                  </code>
                </div>
                <div className="flex items-center gap-2">
                  <span className="badge badge-outline badge-sm px-3 py-2 font-semibold">
                    {getGameTypeKey(getGameKey(game.game_type))}
                  </span>
                  <button
                    className="btn btn-ghost btn-xs text-error hover:bg-error/10"
                    onClick={() => handleRemove(game.id)}
                    title={t('result.remove_tip')}
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>

              {game.warnings && game.warnings.length > 0 && (
                <div className="mt-2 pt-2 border-t border-base-content/5">
                  <h3 className="text-xs font-semibold text-warning flex items-center gap-2 mb-2">
                    <AlertTriangle className="w-3 h-3" />
                    {t('result.warnings_header')}
                  </h3>
                  <ul className="space-y-1">
                    {game.warnings.map((w: string, i: number) => (
                      <li
                        key={i}
                        className="text-[10px] text-base-content/60 flex items-start gap-2"
                      >
                        <span className="mt-1 w-1 h-1 rounded-full bg-base-content/30 shrink-0" />
                        {w}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </motion.div>
          ))}
        </div>
      </div>

      <div className="flex justify-center gap-4 mt-10 w-full max-w-2xl">
        <button className="btn btn-outline gap-2 flex-1" onClick={onBack}>
          <Plus className="w-4 h-4" />
          {t('result.add_another')}
        </button>
        <button
          className="btn btn-primary px-8 gap-2 shadow-lg shadow-primary/20 flex-1 hover:brightness-110 active:scale-[0.98] transition-all"
          onClick={() => onConfirm(confirmedGames)}
        >
          {t('result.confirm')}
          <Check className="w-4 h-4 text-primary-content" />
        </button>
      </div>
    </div>
  );
}
