import { useEffect, useState } from 'react';
import { AlertTriangle, FolderSearch, RotateCw } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';
import type { AppSettings, GameModsDirectoryInspection } from '../../../lib/bindings';
import { formatAppError } from '../../../lib/appError';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useSettings } from '../../../hooks/useSettings';
import { settingsKeys } from '../../../hooks/settingsQuery';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import { applyDiskReconcileResult } from '../../file-watcher/hooks';
import { closeWorkspaceDialog } from '../../workspace-runtime/state/workspaceDialogs';
import { useWorkspaceRuntimeSelector } from '../../workspace-runtime/state/workspaceStoreBridge';

export default function WorkspaceSourceUnavailableDialog() {
  const { t } = useTranslation(['grid', 'common']);
  const { activeGame } = useActiveGame();
  const { settings } = useSettings();
  const queryClient = useQueryClient();
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const unavailableMessage = useAppStore((state) =>
    activeGame?.id ? state.diskReconcileByGame[activeGame.id]?.unavailable : null,
  );
  const [pathMissing, setPathMissing] = useState(false);
  const [dismissedKey, setDismissedKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [inspection, setInspection] = useState<GameModsDirectoryInspection | null>(null);
  const [differentConfirmation, setDifferentConfirmation] = useState('');
  const sourceKey = activeGame ? `${activeGame.id}:${activeGame.mod_path}` : null;

  useEffect(() => {
    let cancelled = false;
    setPathMissing(false);
    setDismissedKey(null);
    setInspection(null);
    setDifferentConfirmation('');
    if (!activeGame?.mod_path) {
      return;
    }

    commands
      .checkPathExistsCmd(activeGame.mod_path)
      .then((exists) => {
        if (!cancelled) setPathMissing(!exists);
      })
      .catch(() => {
        if (!cancelled) setPathMissing(true);
      });

    return () => {
      cancelled = true;
    };
  }, [activeGame?.id, activeGame?.mod_path]);

  const reconcile = async (nextModPath?: string) => {
    if (!activeGame?.id) return;
    const result = await commands.reconcileDiskStateCmd(activeGame.id, 'ManualRepair', null, true);
    applyDiskReconcileResult(
      result,
      queryClient,
      nextModPath ? { ...activeGame, mod_path: nextModPath } : activeGame,
    );
  };

  const handleRetry = async () => {
    if (busy || !activeGame?.mod_path) return;
    setBusy(true);
    try {
      const exists = await commands.checkPathExistsCmd(activeGame.mod_path);
      setPathMissing(!exists);
      if (exists) await reconcile();
    } catch (error) {
      toast.error(t('grid:banners.source_action_failed', { error: formatAppError(error) }));
    } finally {
      setBusy(false);
    }
  };

  const handleLocate = async () => {
    if (busy || !activeGame?.id || !settings) return;
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('grid:banners.source_locate_title'),
    });
    if (!selected || typeof selected !== 'string') return;

    setBusy(true);
    try {
      const resolved = await commands.resolveGameFolder(selected);
      const nextInspection = await commands.inspectGameModsDirectory(
        activeGame.id,
        resolved.mods_path,
      );
      setInspection(nextInspection);
      setDifferentConfirmation('');
      if (
        nextInspection.summary.classification === 'Matching' ||
        nextInspection.summary.classification === 'NewLibrary'
      ) {
        await applyCandidate(nextInspection, false, null);
      }
    } catch (error) {
      toast.error(t('grid:banners.source_action_failed', { error: formatAppError(error) }));
    } finally {
      setBusy(false);
    }
  };

  const applyCandidate = async (
    candidate: GameModsDirectoryInspection,
    confirmEmpty: boolean,
    confirmationGameName: string | null,
  ) => {
    if (!activeGame) return;
    const result = await commands.applyGameModsDirectory({
      game_id: activeGame.id,
      candidate_path: candidate.candidate_path,
      expected_fingerprint: candidate.fingerprint,
      confirm_empty: confirmEmpty,
      different_confirmation_game_name: confirmationGameName,
    });
    const refreshedSettings = await commands.getSettings();
    queryClient.setQueryData<AppSettings>(settingsKeys.all, refreshedSettings);
    applyDiskReconcileResult(result.reconcile, queryClient, result.game);
    if (dialogState.kind === 'sourceRecovery') closeWorkspaceDialog('sourceRecovery');
    setInspection(null);
    setPathMissing(false);
    toast.success(t('grid:banners.source_relocated'));
  };

  const handleConfirmedApply = async () => {
    if (!inspection || busy) return;
    setBusy(true);
    try {
      await applyCandidate(
        inspection,
        inspection.summary.classification === 'Empty',
        inspection.summary.classification === 'Different' ? differentConfirmation : null,
      );
    } catch (error) {
      toast.error(t('grid:banners.source_action_failed', { error: formatAppError(error) }));
    } finally {
      setBusy(false);
    }
  };

  const openDialog =
    dialogState.kind === 'sourceRecovery' ||
    Boolean(sourceKey && dismissedKey !== sourceKey && (pathMissing || unavailableMessage));
  if (!openDialog || !activeGame) return null;

  return (
    <dialog open className="modal modal-open" aria-labelledby="missing-mods-path-title">
      <div className="modal-box max-w-lg border border-error/30">
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-1 shrink-0 text-error" size={24} />
          <div className="min-w-0">
            <h2 id="missing-mods-path-title" className="text-lg font-bold">
              {t('grid:banners.source_dialog_title')}
            </h2>
            <p className="mt-1 text-sm text-base-content/70">
              {unavailableMessage || t('grid:banners.source_dialog_message')}
            </p>
            <code className="mt-3 block break-all rounded bg-base-200 p-2 text-xs">
              {activeGame.mod_path}
            </code>
            {inspection && (
              <div className="mt-4 rounded-lg border border-base-content/15 bg-base-200/60 p-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-semibold">
                    {t(`grid:banners.source_classification_${inspection.summary.classification}`)}
                  </span>
                  <span className="badge badge-sm badge-outline">
                    {t('grid:banners.source_candidate_counts', {
                      objects: inspection.summary.candidate_object_count,
                      mods: inspection.summary.candidate_mod_count,
                    })}
                  </span>
                </div>
                <code className="mt-2 block break-all text-xs text-base-content/70">
                  {inspection.candidate_path}
                </code>
                {inspection.summary.classification === 'Empty' && (
                  <p className="mt-3 text-sm text-warning" role="alert">
                    {t('grid:banners.source_empty_warning')}
                  </p>
                )}
                {inspection.summary.classification === 'Different' && (
                  <div className="mt-3">
                    <p className="text-sm text-error" role="alert">
                      {t('grid:banners.source_different_warning')}
                    </p>
                    <label className="form-control mt-3">
                      <span className="label-text text-xs">
                        {t('grid:banners.source_type_game_name', { name: activeGame.name })}
                      </span>
                      <input
                        className="input input-bordered input-sm mt-1 w-full focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2"
                        value={differentConfirmation}
                        onChange={(event) => setDifferentConfirmation(event.target.value)}
                        autoComplete="off"
                      />
                    </label>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
        <div className="modal-action">
          <button
            type="button"
            className="btn btn-ghost"
            disabled={busy}
            onClick={() => {
              setDismissedKey(sourceKey);
              if (dialogState.kind === 'sourceRecovery') closeWorkspaceDialog('sourceRecovery');
            }}
          >
            {t('grid:banners.source_later_btn')}
          </button>
          <button
            type="button"
            className="btn btn-ghost gap-2"
            disabled={busy}
            onClick={handleRetry}
          >
            <RotateCw size={15} />
            {t('grid:banners.source_retry_btn')}
          </button>
          <button
            type="button"
            className="btn btn-error gap-2"
            disabled={busy}
            onClick={handleLocate}
          >
            <FolderSearch size={15} />
            {t('grid:banners.source_locate_btn')}
          </button>
          {inspection?.summary.classification === 'Empty' && (
            <button
              type="button"
              className="btn btn-warning"
              disabled={busy}
              onClick={handleConfirmedApply}
            >
              {t('grid:banners.source_confirm_empty_btn')}
            </button>
          )}
          {inspection?.summary.classification === 'Different' && (
            <button
              type="button"
              className="btn btn-error"
              disabled={busy || differentConfirmation.trim() !== activeGame.name}
              onClick={handleConfirmedApply}
            >
              {t('grid:banners.source_confirm_different_btn')}
            </button>
          )}
        </div>
      </div>
    </dialog>
  );
}
