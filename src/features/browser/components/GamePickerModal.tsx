import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import type { GameConfig } from '../../../types/game';

interface Props {
  downloadIds: string[];
  open: boolean;
  onClose: () => void;
  /** Called when user confirms a game and the import bulk-queue is kicked off. */
  onConfirm: (gameId: string) => void;
}

export function GamePickerModal({ downloadIds, open, onClose, onConfirm }: Props) {
  const { t } = useTranslation(['browser']);
  const [selectedGameId, setSelectedGameId] = useState<string>('');

  const gamesQuery = useQuery<GameConfig[]>({
    queryKey: ['games'],
    queryFn: () => commands.getGames(),
    enabled: open,
  });

  const games = gamesQuery.data ?? [];

  const handleConfirm = () => {
    if (!selectedGameId) return;
    onConfirm(selectedGameId);
  };

  if (!open) return null;

  return (
    <dialog
      id="game-picker-modal"
      className="modal modal-open"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="modal-box max-w-sm">
        <h3 className="font-bold text-lg mb-1">{t('picker.title')}</h3>
        <p className="text-sm text-base-content/60 mb-4">
          {t('picker.description')}{' '}
          <span className="badge badge-neutral badge-sm">{downloadIds.length}</span>{' '}
          {t('picker.files', { count: downloadIds.length })}.
        </p>

        {gamesQuery.isLoading && (
          <div className="flex justify-center py-6">
            <span className="loading loading-spinner loading-md" />
          </div>
        )}

        {!gamesQuery.isLoading && games.length === 0 && (
          <div className="alert alert-warning">
            <span>{t('picker.no_games')}</span>
          </div>
        )}

        {games.length > 0 && (
          <div className="flex flex-col gap-2 mb-4">
            {games.map((g) => (
              <label
                key={g.id}
                className={`
                  flex items-center gap-3 p-3 rounded-lg border cursor-pointer
                  transition-colors
                  ${
                    selectedGameId === g.id
                      ? 'border-primary bg-primary/10'
                      : 'border-base-300 hover:border-base-content/30'
                  }
                `}
              >
                <input
                  type="radio"
                  name="game-picker-radio"
                  className="radio radio-primary radio-sm"
                  value={g.id}
                  checked={selectedGameId === g.id}
                  onChange={() => setSelectedGameId(g.id)}
                />
                <div>
                  <p className="text-sm font-medium">{g.name}</p>
                  <p className="text-xs text-base-content/50 truncate max-w-55">{g.mod_path}</p>
                </div>
              </label>
            ))}
          </div>
        )}

        <div className="modal-action mt-2">
          <button className="btn btn-ghost btn-sm" onClick={onClose}>
            {t('picker.cancel')}
          </button>
          <button
            id="game-picker-confirm-btn"
            className="btn btn-primary btn-sm"
            disabled={!selectedGameId}
            onClick={handleConfirm}
          >
            {t('picker.confirm')}
          </button>
        </div>
      </div>
    </dialog>
  );
}
