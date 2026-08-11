import { useEffect, useState } from 'react';
import { AlertTriangle, FolderSearch, RotateCw } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';
import { formatAppError } from '../../../lib/appError';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useSettings } from '../../../hooks/useSettings';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import { applyDiskReconcileResult } from '../../file-watcher/hooks';

export default function WorkspaceSourceUnavailableDialog() {
  const { t } = useTranslation(['grid', 'common']);
  const { activeGame } = useActiveGame();
  const { settings, saveSettingsAsync } = useSettings();
  const queryClient = useQueryClient();
  const unavailableMessage = useAppStore((state) =>
    activeGame?.id ? state.diskReconcileByGame[activeGame.id]?.unavailable : null,
  );
  const [pathMissing, setPathMissing] = useState(false);
  const [dismissedKey, setDismissedKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const sourceKey = activeGame ? `${activeGame.id}:${activeGame.mod_path}` : null;

  useEffect(() => {
    let cancelled = false;
    setPathMissing(false);
    setDismissedKey(null);
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
      const games = settings.games.map((game) =>
        game.id === activeGame.id ? { ...game, mod_path: resolved.mods_path } : game,
      );
      await saveSettingsAsync({ ...settings, games });
      await reconcile(resolved.mods_path);
      setPathMissing(false);
      toast.success(t('grid:banners.source_relocated'));
    } catch (error) {
      toast.error(t('grid:banners.source_action_failed', { error: formatAppError(error) }));
    } finally {
      setBusy(false);
    }
  };

  const openDialog = Boolean(
    sourceKey && dismissedKey !== sourceKey && (pathMissing || unavailableMessage),
  );
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
          </div>
        </div>
        <div className="modal-action">
          <button
            type="button"
            className="btn btn-ghost"
            disabled={busy}
            onClick={() => setDismissedKey(sourceKey)}
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
        </div>
      </div>
    </dialog>
  );
}
