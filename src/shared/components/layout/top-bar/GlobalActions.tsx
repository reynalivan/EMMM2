import {
  RefreshCw,
  Settings,
  MoreVertical,
  PanelRightClose,
  PanelRightOpen,
  Trash2,
} from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../../../stores/useAppStore';
import LaunchBar from '../../../../features/launch-bar/LaunchBar';
import { commands } from '../../../../core/tauri/bindings';
import { formatAppError } from '../../../../core/lib/appError';
import { toast, useToastStore } from '../../../../stores/useToastStore';
import { useActiveGame } from '../../../../features/dashboard/hooks/useActiveGame';
import { applyDiskReconcileResult } from '../../../../features/file-watcher/hooks/useFileWatcher';

export default function GlobalActions() {
  const { t } = useTranslation('layout');
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const workspaceView = useAppStore((state) => state.workspaceView);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const isPreviewOpen = useAppStore((state) => state.isPreviewOpen);
  const togglePreview = useAppStore((state) => state.togglePreview);
  const reconcileInProgress = useAppStore((state) =>
    activeGame ? Boolean(state.diskReconcileByGame[activeGame.id]?.progress) : false,
  );
  const [isRefreshing, setIsRefreshing] = useState(false);
  const refreshInFlight = useRef(false);

  const openRecycleBin = () => {
    void commands.openRecycleBin().catch((error: unknown) => toast.error(formatAppError(error)));
  };

  const runFullReconcile = async () => {
    if (!activeGame || refreshInFlight.current) {
      return;
    }

    refreshInFlight.current = true;
    setIsRefreshing(true);
    const toastStore = useToastStore.getState();
    const pendingToastId = toastStore.addToast(
      'info',
      t('actions.refresh_started', { name: activeGame.name }),
      0,
    );

    try {
      const result = await commands.reconcileDiskStateCmd(
        activeGame.id,
        'ManualRepair',
        null,
        true,
      );
      applyDiskReconcileResult(result, queryClient, activeGame);
      toastStore.removeToast(pendingToastId);

      switch (result.status) {
        case 'Applied':
          toastStore.addToast(
            'success',
            t(
              result.change_summary.has_user_visible_changes
                ? 'actions.refresh_complete'
                : 'actions.refresh_already_synced',
            ),
          );
          break;
        case 'AppliedWithFolderConflicts':
          toastStore.addToast(
            'warning',
            t('actions.refresh_conflicts', { count: result.folder_conflicts.length }),
          );
          break;
        case 'NeedsRenameConfirmation':
          toastStore.addToast(
            'warning',
            t('actions.refresh_rename_confirmation', {
              count: result.rename_confirmations.length,
            }),
          );
          break;
        case 'SourceUnavailable':
          toastStore.addToast(
            'error',
            t('actions.refresh_source_unavailable', {
              error: result.error_message ?? activeGame.mod_path,
            }),
          );
          break;
      }
    } catch (error) {
      toastStore.removeToast(pendingToastId);
      toastStore.addToast('error', t('actions.refresh_failed', { error: formatAppError(error) }));
    } finally {
      refreshInFlight.current = false;
      setIsRefreshing(false);
    }
  };

  const refreshDisabled = !activeGame || isRefreshing || reconcileInProgress;

  return (
    <div className="flex items-center gap-2 md:gap-3">
      {/* Desktop Tools */}
      <div className="hidden md:flex items-center gap-1">
        <button
          className="btn btn-ghost btn-sm btn-square text-base-content/50 hover:text-warning hover:bg-base-content/10"
          title={t('actions.trash')}
          onClick={openRecycleBin}
        >
          <Trash2 size={18} />
        </button>
        <button
          type="button"
          className="btn btn-ghost btn-sm btn-square text-base-content/50 hover:text-primary hover:bg-base-content/10 disabled:text-base-content/20"
          title={t('actions.refresh')}
          aria-label={t('actions.refresh')}
          disabled={refreshDisabled}
          onClick={() => void runFullReconcile()}
        >
          <RefreshCw
            size={18}
            className={isRefreshing ? 'animate-spin motion-reduce:animate-none' : undefined}
          />
        </button>
        <button
          className="btn btn-ghost btn-sm btn-square text-base-content/50 hover:text-primary hover:bg-base-content/10"
          title={t('actions.settings')}
          onClick={() => setWorkspaceView('settings')}
        >
          <Settings size={18} />
        </button>

        {/* Launch Bar (Epic 10) */}
        <LaunchBar />
      </div>

      {/* Mobile Menu Dropdown */}
      <div className="dropdown dropdown-end md:hidden">
        <div
          tabIndex={0}
          role="button"
          className="btn btn-sm btn-ghost btn-square text-base-content/70"
        >
          <MoreVertical size={18} />
        </div>
        <ul
          tabIndex={0}
          className="dropdown-content z-100 menu p-2 shadow-2xl bg-base-100/95 backdrop-blur-xl rounded-box w-48 mt-2 border border-base-content/10"
        >
          <li>
            <a
              className="gap-2 hover:bg-base-content/10"
              onClick={() => setWorkspaceView('settings')}
            >
              <Settings size={16} /> {t('actions.settings')}
            </a>
          </li>
          <li>
            <a className="gap-2 hover:bg-base-content/10" onClick={openRecycleBin}>
              <Trash2 size={16} /> {t('actions.trash')}
            </a>
          </li>
          <li>
            <button
              type="button"
              className="gap-2 hover:bg-base-content/10"
              disabled={refreshDisabled}
              onClick={() => void runFullReconcile()}
            >
              <RefreshCw
                size={16}
                className={isRefreshing ? 'animate-spin motion-reduce:animate-none' : undefined}
              />{' '}
              {t('actions.refresh')}
            </button>
          </li>
        </ul>
      </div>

      <div className="w-px h-6 bg-base-content/10 mx-1 hidden md:block" />

      {/* Desktop Toggle Preview */}
      {workspaceView === 'mods' && (
        <button
          onClick={togglePreview}
          className={`btn btn-sm btn-square hidden md:flex ml-1 transition-all duration-300 ${isPreviewOpen ? 'btn-ghost text-primary bg-primary/10' : 'btn-ghost text-base-content/30 hover:text-primary'}`}
          title={isPreviewOpen ? t('actions.hide_preview') : t('actions.show_preview')}
        >
          {isPreviewOpen ? <PanelRightClose size={18} /> : <PanelRightOpen size={18} />}
        </button>
      )}
    </div>
  );
}
