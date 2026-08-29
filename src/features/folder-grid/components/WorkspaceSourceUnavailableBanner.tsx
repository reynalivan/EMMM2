import { formatAppError } from '../../../lib/appError';
import { useState } from 'react';
import { AlertTriangle, FolderSearch, RotateCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { applyDiskReconcileResult } from '../../file-watcher/hooks';
import { toast } from '../../../stores/useToastStore';
import { openWorkspaceSourceRecoveryDialog } from '../../workspace-runtime/state/workspaceDialogs';

interface WorkspaceSourceUnavailableBannerProps {
  message: string;
}

/**
 * Actionable banner shown when the active game's mods folder is missing on disk
 * (moved, renamed, or drive disconnected). Offers an in-place recovery:
 * - Retry: re-run reconcile — repairs the common case where the folder returned.
 * - Locate folder…: reopen the recovery dialog, which inspects and classifies
 *   the candidate before the backend atomically applies it.
 */
export default function WorkspaceSourceUnavailableBanner({
  message,
}: WorkspaceSourceUnavailableBannerProps) {
  const { t } = useTranslation(['grid']);
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);

  const reconcile = async () => {
    if (!activeGame?.id) {
      return;
    }
    const result = await commands.reconcileDiskStateCmd(activeGame.id, 'ManualRepair', null, true);
    applyDiskReconcileResult(result, queryClient, activeGame);
  };

  const handleRetry = async () => {
    if (busy || !activeGame?.id) {
      return;
    }
    setBusy(true);
    try {
      await reconcile();
    } catch (error) {
      toast.error(t('grid:banners.source_action_failed', { error: formatAppError(error) }));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="mb-3 flex items-center gap-3 bg-error/10 border border-error/20 rounded-lg px-3 py-2">
      <AlertTriangle size={16} className="text-error shrink-0" />
      <div className="flex-1 min-w-0">
        <p className="text-xs text-error font-medium truncate">{message}</p>
        <p className="text-[11px] text-error/70">{t('grid:banners.source_hint')}</p>
      </div>
      <div className="flex items-center gap-1.5 shrink-0">
        <button
          className="btn btn-xs btn-ghost btn-error gap-1"
          onClick={handleRetry}
          disabled={busy || !activeGame?.id}
        >
          <RotateCw size={12} />
          {t('grid:banners.source_retry_btn')}
        </button>
        <button
          className="btn btn-xs btn-error gap-1"
          onClick={openWorkspaceSourceRecoveryDialog}
          disabled={busy || !activeGame?.id}
        >
          <FolderSearch size={12} />
          {t('grid:banners.source_locate_btn')}
        </button>
      </div>
    </div>
  );
}
