import { formatAppError } from '../../../../shared/lib/appError';
import { useState } from 'react';
import { Plus, Edit2, Trash2, Play, Inbox } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings, GameConfig } from '../../hooks/useSettings';
import GameFormModal from '../../modals/GameFormModal';
import { useAppStore } from '@/app/store';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../../shared/api/tauri/bindings';
import type { GameModsDirectoryInspection } from '../../../../shared/api/tauri/bindings';
import { pathsEqual } from '../../../../shared/lib/pathKey';
import { applyDiskReconcileResult } from '@/features/file-watcher';

interface PendingSourceChange {
  game: GameConfig;
  inspection: GameModsDirectoryInspection;
}

export default function GamesTab() {
  const { t } = useTranslation(['settings', 'common', 'grid']);
  const { settings, saveSettingsAsync } = useSettings();
  const setActiveGameId = useAppStore((state) => state.setActiveGameId);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const queryClient = useQueryClient();
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingGame, setEditingGame] = useState<GameConfig | null>(null);
  const [pendingSourceChange, setPendingSourceChange] = useState<PendingSourceChange | null>(null);
  const [sourceConfirmation, setSourceConfirmation] = useState('');
  const [sourceChangeError, setSourceChangeError] = useState<string | null>(null);
  const [sourceChangePending, setSourceChangePending] = useState(false);

  const handleAdd = () => {
    setEditingGame(null);
    setIsModalOpen(true);
  };

  const handleEdit = (game: GameConfig) => {
    setEditingGame(game);
    setIsModalOpen(true);
  };

  const handleDelete = async (id: string) => {
    if (!settings) return;
    if (window.confirm(t('settings:games.delete_confirm'))) {
      const newGames = settings.games.filter((g) => g.id !== id);
      await saveSettingsAsync({ ...settings, games: newGames });

      // If deleted active game, deselect it
      if (activeGameId === id) {
        setActiveGameId(null);
      }
    }
  };

  const applySourceChange = async (
    pending: PendingSourceChange,
    confirmEmpty: boolean,
    differentGameName: string | null,
  ) => {
    const applied = await commands.applyGameModsDirectory({
      game_id: pending.game.id,
      candidate_path: pending.inspection.candidate_path,
      expected_fingerprint: pending.inspection.fingerprint,
      confirm_empty: confirmEmpty,
      different_confirmation_game_name: differentGameName,
    });
    const refreshed = await commands.getSettings();
    const games = refreshed.games.map((configured) =>
      configured.id === pending.game.id
        ? { ...pending.game, mod_path: applied.game.mod_path }
        : configured,
    );
    await saveSettingsAsync({ ...refreshed, games });
    applyDiskReconcileResult(applied.reconcile, queryClient, applied.game);
  };

  const handleSave = async (game: GameConfig): Promise<boolean> => {
    if (!settings) return false;

    const newGames = [...settings.games];
    const index = newGames.findIndex((g) => g.id === game.id);

    if (index >= 0) {
      if (!pathsEqual(newGames[index].mod_path, game.mod_path)) {
        const inspection = await commands.inspectGameModsDirectory(game.id, game.mod_path);
        const pending = { game, inspection };
        if (
          inspection.summary.classification === 'Matching' ||
          inspection.summary.classification === 'NewLibrary'
        ) {
          await applySourceChange(pending, false, null);
          return true;
        }
        setPendingSourceChange(pending);
        setSourceConfirmation('');
        setSourceChangeError(null);
        return false;
      }
      newGames[index] = game;
    } else {
      newGames.push(game);
    }

    await saveSettingsAsync({ ...settings, games: newGames });
    return true;
  };

  const confirmSourceChange = async () => {
    if (!pendingSourceChange) return;
    setSourceChangePending(true);
    setSourceChangeError(null);
    try {
      const classification = pendingSourceChange.inspection.summary.classification;
      await applySourceChange(
        pendingSourceChange,
        classification === 'Empty',
        classification === 'Different' ? sourceConfirmation : null,
      );
      setPendingSourceChange(null);
      setSourceConfirmation('');
      setIsModalOpen(false);
    } catch (error) {
      setSourceChangeError(formatAppError(error));
    } finally {
      setSourceChangePending(false);
    }
  };

  if (!settings) return <div>{t('common:status.loading')}</div>;

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center bg-base-200/50 p-4 rounded-xl border border-base-300">
        <div>
          <h2 className="text-xl font-bold">{t('settings:games.title')}</h2>
          <p className="mt-1 text-sm opacity-70">{t('settings:games.desc')}</p>
        </div>
        <button className="btn btn-primary gap-2" data-testid="games-add" onClick={handleAdd}>
          <Plus size={18} /> {t('settings:games.add')}
        </button>
      </div>

      <div className="grid grid-cols-1 gap-4">
        {settings.games.length === 0 ? (
          <div className="text-center py-12 opacity-50 border-2 border-dashed border-base-300 rounded-xl">
            <p>{t('settings:games.empty')}</p>
          </div>
        ) : (
          settings.games.map((game) => (
            <div
              key={game.id}
              className={`card bg-base-200 shadow-md border-l-4 transition-all hover:shadow-lg ${activeGameId === game.id ? 'border-primary' : 'border-base-300 opacity-90 hover:opacity-100'}`}
            >
              <div className="card-body p-5 flex flex-row items-center justify-between gap-4">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 bg-base-300 rounded-lg flex items-center justify-center font-bold text-xl text-primary/50">
                    {game.name.charAt(0)}
                  </div>
                  <div>
                    <h3 className="card-title text-base flex items-center gap-2">
                      {game.name}
                      {activeGameId === game.id && (
                        <span className="badge badge-primary badge-xs">
                          {t('settings:games.active')}
                        </span>
                      )}
                    </h3>
                    <div className="text-xs space-y-1 mt-1 opacity-70">
                      <p className="flex items-center gap-1">
                        <span className="font-semibold">
                          {t('settings:games.form.path_label_short')}:
                        </span>{' '}
                        {game.mod_path}
                      </p>
                      <p className="flex items-center gap-1">
                        <span className="font-semibold">
                          {t('settings:games.form.exe_label_short')}:
                        </span>{' '}
                        {game.game_exe}
                      </p>
                    </div>
                  </div>
                </div>

                <div className="join">
                  <button
                    className="btn btn-ghost btn-sm join-item text-accent"
                    onClick={() => {
                      setActiveGameId(game.id);
                      setWorkspaceView('mod-inbox');
                    }}
                    title={t('settings:games.actions.scan_ready_to_move')}
                  >
                    <Inbox size={16} />
                  </button>
                  <button
                    className="btn btn-ghost btn-sm join-item text-primary"
                    onClick={() => setActiveGameId(game.id)}
                    disabled={activeGameId === game.id}
                    title={t('settings:games.actions.set_active')}
                  >
                    <Play size={16} />
                  </button>
                  <button
                    className="btn btn-ghost btn-sm join-item"
                    onClick={() => handleEdit(game)}
                    title={t('settings:games.actions.edit')}
                  >
                    <Edit2 size={16} />
                  </button>
                  <button
                    className="btn btn-ghost btn-sm join-item text-error hover:bg-error/10"
                    onClick={() => void handleDelete(game.id)}
                    title={t('settings:games.actions.remove')}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            </div>
          ))
        )}
      </div>

      <GameFormModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSave={handleSave}
        initialData={editingGame}
        existingModPaths={settings.games
          .filter((game) => game.id !== editingGame?.id)
          .map((game) => game.mod_path)}
      />
      {pendingSourceChange && (
        <dialog open className="modal modal-open" aria-labelledby="settings-source-change-title">
          <div className="modal-box max-w-lg">
            <h3 id="settings-source-change-title" className="text-lg font-bold">
              {t('grid:banners.source_dialog_title')}
            </h3>
            <p className="mt-3 text-sm text-warning" role="alert">
              {t(
                pendingSourceChange.inspection.summary.classification === 'Empty'
                  ? 'grid:banners.source_empty_warning'
                  : 'grid:banners.source_different_warning',
              )}
            </p>
            <code className="mt-3 block break-all rounded bg-base-200 p-2 text-xs">
              {pendingSourceChange.inspection.candidate_path}
            </code>
            {sourceChangeError && (
              <p className="alert alert-error mt-3 text-sm" role="alert">
                {sourceChangeError}
              </p>
            )}
            {pendingSourceChange.inspection.summary.classification === 'Different' && (
              <label className="form-control mt-4">
                <span className="label-text text-xs">
                  {t('grid:banners.source_type_game_name', {
                    name: pendingSourceChange.game.name,
                  })}
                </span>
                <input
                  className="input input-bordered mt-1 w-full"
                  value={sourceConfirmation}
                  onChange={(event) => setSourceConfirmation(event.target.value)}
                  autoComplete="off"
                />
              </label>
            )}
            <div className="modal-action">
              <button
                type="button"
                className="btn btn-ghost"
                disabled={sourceChangePending}
                onClick={() => {
                  setPendingSourceChange(null);
                  setSourceChangeError(null);
                }}
              >
                {t('common:actions.cancel')}
              </button>
              <button
                type="button"
                className="btn btn-warning"
                disabled={
                  sourceChangePending ||
                  (pendingSourceChange.inspection.summary.classification === 'Different' &&
                    sourceConfirmation.trim() !== pendingSourceChange.game.name)
                }
                onClick={() => void confirmSourceChange()}
              >
                {t(
                  pendingSourceChange.inspection.summary.classification === 'Empty'
                    ? 'grid:banners.source_confirm_empty_btn'
                    : 'grid:banners.source_confirm_different_btn',
                )}
              </button>
            </div>
          </div>
        </dialog>
      )}
    </div>
  );
}
