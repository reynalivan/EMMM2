import { formatAppError } from '../../../../shared/lib/appError';
import { useState } from 'react';
import { Plus, Edit2, Trash2, Play, Inbox, LoaderCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '@/entities/settings';
import type { GameConfig } from '@/entities/game';
import GameFormModal from '../../modals/GameFormModal';
import { useAppStore } from '@/app/store';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../../shared/api/tauri/bindings';
import type { GameModsDirectoryInspection } from '../../../../shared/api/tauri/bindings';
import { pathsEqual } from '../../../../shared/lib/pathKey';
import { applyDiskReconcileResult } from '@/features/file-watcher';
import ConfirmDialog from '@/shared/ui/components/ui/ConfirmDialog';
import { SettingsSection } from '../SettingsLayout';

interface PendingSourceChange {
  game: GameConfig;
  inspection: GameModsDirectoryInspection;
}

interface SourceChangeSummary {
  objectsAdded: number;
  objectsRemoved: number;
  objectsMoved: number;
  modsAdded: number;
  modsRemoved: number;
  modsMoved: number;
  collectionsAffected: number;
  emmmDataMoved: boolean;
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
  const [sourceChangeWarning, setSourceChangeWarning] = useState<string | null>(null);
  const [sourceChangeSummary, setSourceChangeSummary] = useState<SourceChangeSummary | null>(null);
  const [sourceChangePending, setSourceChangePending] = useState(false);
  const [pendingDeleteGameId, setPendingDeleteGameId] = useState<string | null>(null);

  const handleAdd = () => {
    setEditingGame(null);
    setIsModalOpen(true);
  };

  const handleEdit = (game: GameConfig) => {
    setEditingGame(game);
    setIsModalOpen(true);
  };

  const handleDelete = (id: string) => {
    setPendingDeleteGameId(id);
  };

  const confirmDelete = async () => {
    if (!settings) return;
    if (!pendingDeleteGameId) return;
    const newGames = settings.games.filter((game) => game.id !== pendingDeleteGameId);
    await saveSettingsAsync({ ...settings, games: newGames });

    if (activeGameId === pendingDeleteGameId) {
      setActiveGameId(null);
    }
    setPendingDeleteGameId(null);
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
    const { object_changes, mod_changes } = applied.reconcile.change_summary;
    setSourceChangeSummary({
      objectsAdded: object_changes.added,
      objectsRemoved: object_changes.removed,
      objectsMoved: object_changes.renamed,
      modsAdded: mod_changes.added,
      modsRemoved: mod_changes.removed,
      modsMoved: mod_changes.renamed,
      collectionsAffected: applied.reconcile.collection_reference_impact.affected_collection_count,
      emmmDataMoved: applied.emmm_data_moved,
    });
    setSourceChangeWarning(applied.watcher_warning);
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
          setSourceChangePending(true);
          setSourceChangeWarning(null);
          setSourceChangeSummary(null);
          try {
            await applySourceChange(pending, false, null);
          } finally {
            setSourceChangePending(false);
          }
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
    setSourceChangeWarning(null);
    setSourceChangeSummary(null);
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
    <div>
      <SettingsSection
        id="games-settings-heading"
        title={t('settings:games.title')}
        description={t('settings:games.desc')}
        action={
          <button
            className="btn btn-primary btn-sm gap-2 whitespace-nowrap"
            data-testid="games-add"
            onClick={handleAdd}
            disabled={sourceChangePending}
          >
            <Plus size={18} /> {t('settings:games.add')}
          </button>
        }
      >
        <div className="divide-y divide-base-300">
          {settings.games.length === 0 ? (
            <div className="border border-dashed border-base-300 py-10 text-center text-sm text-base-content/60">
              <p>{t('settings:games.empty')}</p>
            </div>
          ) : (
            settings.games.map((game) => (
              <div
                key={game.id}
                className={`py-3 transition-colors duration-150 ${
                  activeGameId === game.id ? 'bg-primary/5' : 'hover:bg-base-content/3'
                }`}
              >
                <div className="flex flex-col items-start justify-between gap-3 px-2 sm:flex-row sm:items-center">
                  <div className="flex items-center gap-4">
                    <div className="flex h-9 w-9 items-center justify-center rounded-md bg-base-300/70 text-sm font-semibold text-base-content/60">
                      {game.name.charAt(0)}
                    </div>
                    <div>
                      <h3 className="flex items-center gap-2 text-sm font-semibold">
                        {game.name}
                        {activeGameId === game.id && (
                          <span className="badge badge-primary badge-xs">
                            {t('settings:games.active')}
                          </span>
                        )}
                      </h3>
                      <div className="mt-1 space-y-0.5 text-xs text-base-content/60">
                        <p className="flex items-center gap-1">
                          <span className="font-semibold">
                            {t('settings:games.form.path_label_short')}:
                          </span>{' '}
                          {game.mod_path}
                        </p>
                        <p className="flex items-center gap-1">
                          <span className="font-semibold">
                            {game.launch_mode === 'xxmi_managed'
                              ? t('settings:games.form.xxmi_label_short')
                              : t('settings:games.form.exe_label_short')}
                            :
                          </span>{' '}
                          {game.launch_mode === 'xxmi_managed'
                            ? game.xxmi_launcher_exe
                            : game.game_exe}
                        </p>
                      </div>
                    </div>
                  </div>

                  <div className="join">
                    <button
                      className="btn btn-ghost btn-sm join-item"
                      onClick={() => {
                        setActiveGameId(game.id);
                        setWorkspaceView('mod-inbox');
                      }}
                      disabled={sourceChangePending}
                      title={t('settings:games.actions.scan_ready_to_move')}
                    >
                      <Inbox size={16} />
                    </button>
                    <button
                      className="btn btn-ghost btn-sm join-item text-primary"
                      onClick={() => setActiveGameId(game.id)}
                      disabled={sourceChangePending || activeGameId === game.id}
                      title={t('settings:games.actions.set_active')}
                    >
                      <Play size={16} />
                    </button>
                    <button
                      className="btn btn-ghost btn-sm join-item"
                      onClick={() => handleEdit(game)}
                      disabled={sourceChangePending}
                      title={t('settings:games.actions.edit')}
                    >
                      <Edit2 size={16} />
                    </button>
                    <button
                      className="btn btn-ghost btn-sm join-item text-error hover:bg-error/10"
                      onClick={() => handleDelete(game.id)}
                      disabled={sourceChangePending}
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
      </SettingsSection>
      {sourceChangeWarning && (
        <div className="alert alert-warning mt-4 text-sm" role="alert">
          {sourceChangeWarning}
        </div>
      )}
      {sourceChangeSummary && (
        <div className="alert alert-success mt-4 text-sm" role="status">
          <span>
            {t('settings:games.source_change_summary', { ...sourceChangeSummary })}{' '}
            {t(
              sourceChangeSummary.emmmDataMoved
                ? 'settings:games.source_change_artifacts_moved'
                : 'settings:games.source_change_artifacts_unchanged',
            )}
          </span>
        </div>
      )}

      <GameFormModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSave={handleSave}
        initialData={editingGame}
        existingModPaths={settings.games
          .filter((game) => game.id !== editingGame?.id)
          .map((game) => game.mod_path)}
        isSourceMigrationPending={sourceChangePending}
      />
      <ConfirmDialog
        open={pendingDeleteGameId !== null}
        title={t('settings:games.actions.remove')}
        message={t('settings:games.delete_confirm')}
        danger
        onCancel={() => setPendingDeleteGameId(null)}
        onConfirm={() => void confirmDelete()}
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
            {sourceChangePending && (
              <p className="alert alert-info mt-3 text-sm" role="status">
                <LoaderCircle className="size-4 animate-spin" aria-hidden="true" />
                {t('settings:games.source_change_progress')}
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
