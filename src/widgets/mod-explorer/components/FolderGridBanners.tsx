import { FolderOpen, AlertTriangle, LoaderCircle, Lock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  openFolderConflictManagerDialog,
  openRenameConfirmationDialog,
} from '@/features/workspace-runtime/state/workspaceDialogs';
import WorkspaceSourceUnavailableBanner from './WorkspaceSourceUnavailableBanner';
import { useAppStore } from '../../../app/store/useAppStore';

const EMPTY_DISK_CONFLICTS: never[] = [];

export interface FolderGridBannersProps {
  isLoading: boolean;
  isError: boolean;
  isFlatModRoot: boolean;
  selfIsEnabled: boolean;
  selfReasons: string[];
  isMobile: boolean;
  isPreviewOpen: boolean;
  currentPath: string[];
  setMobilePane: (pane: 'sidebar' | 'grid' | 'details') => void;
  togglePreview: () => void;
  handleToggleSelf: (enabled: boolean) => void;
  /** Display name of nearest disabled ancestor — null when not locked */
  ancestorDisabledBy: string | null;
  /** Open the EnableParent confirmation dialog with impact preview */
  onOpenEnableParentDialog: () => void;
  diskSourceUnavailableMessage: string | null;
  recoveryStatus: 'ready' | 'syncing' | 'failed';
  mutationsDisabled: boolean;
}

export default function FolderGridBanners({
  isLoading,
  isError,
  isFlatModRoot,
  selfIsEnabled,
  selfReasons,
  isMobile,
  isPreviewOpen,
  currentPath,
  setMobilePane,
  togglePreview,
  handleToggleSelf,
  ancestorDisabledBy,
  onOpenEnableParentDialog,
  diskSourceUnavailableMessage,
  recoveryStatus,
  mutationsDisabled,
}: FolderGridBannersProps) {
  const { t } = useTranslation(['grid', 'folder_grid']);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const conflictsByGame = useAppStore((state) => state.folderConflictsByGame);
  const renameConfirmationsByGame = useAppStore((state) => state.renameConfirmationsByGame);
  const reconcileProgress = useAppStore((state) =>
    activeGameId ? (state.diskReconcileByGame[activeGameId]?.progress ?? null) : null,
  );
  const diskConflicts = activeGameId
    ? (conflictsByGame[activeGameId] ?? EMPTY_DISK_CONFLICTS)
    : EMPTY_DISK_CONFLICTS;
  const renameConfirmations = activeGameId
    ? (renameConfirmationsByGame[activeGameId] ?? EMPTY_DISK_CONFLICTS)
    : EMPTY_DISK_CONFLICTS;

  if (isLoading || isError) {
    return null;
  }

  const isObjectLevel = currentPath.length === 1;

  return (
    <>
      {diskSourceUnavailableMessage && (
        <WorkspaceSourceUnavailableBanner message={diskSourceUnavailableMessage} />
      )}

      {(recoveryStatus === 'syncing' || reconcileProgress) && (
        <div
          className="mb-3 flex items-center gap-3 rounded-lg border border-info/30 bg-info/10 px-3 py-2"
          role="status"
          data-testid="workspace-reconcile-sync-banner"
        >
          <LoaderCircle
            size={16}
            className="shrink-0 animate-spin motion-reduce:animate-none text-info"
          />
          <div className="min-w-0 flex-1">
            <div className="text-xs text-info">{t('banners.disk_syncing')}</div>
            {reconcileProgress && reconcileProgress.total_units !== null && (
              <div className="mt-1 flex items-center gap-2">
                <progress
                  className="progress progress-info h-1.5 flex-1"
                  value={reconcileProgress.completed_units}
                  max={reconcileProgress.total_units}
                  aria-label={t('banners.disk_syncing')}
                />
                <span className="shrink-0 text-[10px] tabular-nums text-info/80">
                  {reconcileProgress.completed_units}/{reconcileProgress.total_units}
                  {reconcileProgress.eta_ms !== null
                    ? ` · ~${Math.ceil(reconcileProgress.eta_ms / 1000)}s`
                    : ''}
                </span>
              </div>
            )}
            {reconcileProgress && reconcileProgress.current_root && (
              <div className="mt-1 truncate text-[10px] text-info/75">
                {reconcileProgress.current_root}
              </div>
            )}
          </div>
        </div>
      )}

      {diskConflicts.length > 0 && (
        <div
          className="mb-3 flex items-center gap-2 rounded-lg border border-warning/30 bg-warning/10 px-3 py-2"
          role="status"
          data-testid="folder-conflict-banner"
        >
          <AlertTriangle size={16} className="shrink-0 text-warning" />
          <span className="flex-1 text-xs text-warning">
            {t('folder_grid:conflict_manager.banner', { count: diskConflicts.length })}
          </span>
          <button className="btn btn-xs btn-warning" onClick={openFolderConflictManagerDialog}>
            {t('folder_grid:conflict_manager.resolve')}
          </button>
        </div>
      )}

      {renameConfirmations.length > 0 && (
        <div
          className="mb-3 flex items-center gap-2 rounded-lg border border-info/30 bg-info/10 px-3 py-2"
          role="status"
          data-testid="rename-confirmation-banner"
        >
          <AlertTriangle size={16} className="shrink-0 text-info" />
          <span className="flex-1 text-xs text-info">
            {t('folder_grid:rename_confirmation.banner', {
              count: renameConfirmations.length,
            })}
          </span>
          <button className="btn btn-xs btn-info" onClick={openRenameConfirmationDialog}>
            {t('folder_grid:rename_confirmation.review')}
          </button>
        </div>
      )}

      {/* ── Parent-Disabled Notice (compact, topmost) ─────────────────────── */}
      {ancestorDisabledBy && (
        <div className="sticky top-0 z-20 mb-3 flex items-center gap-2 bg-warning/10 border-b border-warning/20 px-3 py-1.5 -mx-4 -mt-4 shadow-sm backdrop-blur-md">
          <Lock size={12} className="text-warning shrink-0" />
          <div className="flex-1 min-w-0">
            <p className="text-[10px] font-bold text-warning/90 leading-none truncate uppercase tracking-wider">
              {isObjectLevel
                ? t('banners.parent_disabled_object_title')
                : t('banners.parent_disabled_title', { name: ancestorDisabledBy })}
            </p>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            <button
              className="btn btn-sm btn-warning text-[10px] px-4 font-bold shadow-sm"
              onClick={onOpenEnableParentDialog}
            >
              {t('banners.enable_parent_btn')}
            </button>
          </div>
        </div>
      )}

      {/* ── Flat Mod Root Banner (ONLY when disabled) ─────────────────────── */}
      {isFlatModRoot && !selfIsEnabled && (
        <div className="mb-4 bg-base-200 border border-base-content/10 rounded-xl p-6 flex flex-col md:flex-row items-start md:items-center gap-4 mx-4 shadow-sm relative overflow-hidden">
          {/* Decorative left accent */}
          <div className="absolute left-0 top-0 bottom-0 w-1 bg-base-content/20" />

          <div className="flex-1 pl-2">
            <h3 className="text-lg font-bold flex items-center gap-2">
              <FolderOpen className="text-base-content/40" size={20} />
              {t('banners.flat_mod_title')}
            </h3>
            <p className="text-sm text-base-content/60 mt-1 max-w-2xl">
              {t('banners.flat_mod_desc')}
            </p>
            {selfReasons.length > 0 && (
              <div className="mt-3 flex flex-wrap gap-1.5">
                {selfReasons.map((reason, i) => (
                  <span
                    key={i}
                    className="text-[10px] font-mono bg-base-300 px-2 py-0.5 rounded text-base-content/50 border border-base-content/5"
                  >
                    {reason}
                  </span>
                ))}
              </div>
            )}
          </div>

          <div className="flex items-center gap-2 mt-4 md:mt-0 whitespace-nowrap">
            <button
              className="btn btn-sm btn-success"
              disabled={mutationsDisabled}
              onClick={() => handleToggleSelf(true)}
            >
              {t('banners.enable_mod')}
            </button>
            <button
              className="btn btn-sm btn-outline btn-ghost"
              onClick={() => {
                if (isMobile) {
                  setMobilePane('details');
                } else if (!isPreviewOpen) {
                  togglePreview();
                }
              }}
            >
              {t('banners.view_details')}
            </button>
          </div>
        </div>
      )}
    </>
  );
}
