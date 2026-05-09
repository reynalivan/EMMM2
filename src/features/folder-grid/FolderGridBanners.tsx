import { FolderOpen, AlertTriangle, Lock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ConflictGroup } from '../../types/mod';
import { openWorkspaceConflictDialog } from '../workspace-runtime/state/workspaceDialogs';

export interface FolderGridBannersProps {
  isLoading: boolean;
  isError: boolean;
  nameConflicts: ConflictGroup[];
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
}

export default function FolderGridBanners({
  isLoading,
  isError,
  nameConflicts,
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
}: FolderGridBannersProps) {
  const { t } = useTranslation(['grid']);

  if (isLoading || isError) {
    return null;
  }

  const isObjectLevel = currentPath.length === 1;

  return (
    <>
      {diskSourceUnavailableMessage && (
        <div className="mb-3 flex items-center gap-2 bg-error/10 border border-error/20 rounded-lg px-3 py-2">
          <AlertTriangle size={16} className="text-error shrink-0" />
          <span className="text-xs text-error flex-1">{diskSourceUnavailableMessage}</span>
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

      {/* ── Naming Conflict Banner ─────────────────────────────────────────── */}
      {nameConflicts.length > 0 && (
        <div className="mb-3 flex items-center gap-2 bg-warning/10 border border-warning/20 rounded-lg px-3 py-2">
          <AlertTriangle size={16} className="text-warning shrink-0" />
          <span className="text-xs text-warning flex-1">
            {t('banners.conflict_detected', { count: nameConflicts.length })}
          </span>
          <button
            className="btn btn-xs btn-warning btn-outline"
            onClick={() => {
              const c = nameConflicts[0];
              if (c.members.length >= 2) {
                const enabled = c.members.find((m) => m.is_enabled);
                const disabled = c.members.find((m) => !m.is_enabled);
                if (enabled && disabled) {
                  openWorkspaceConflictDialog({
                    type: 'RenameConflict',
                    attempted_target: enabled.path,
                    existing_path: disabled.path,
                    base_name: c.base_name,
                  });
                }
              }
            }}
          >
            {t('banners.resolve_btn')}
          </button>
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
            <button className="btn btn-sm btn-success" onClick={() => handleToggleSelf(true)}>
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
