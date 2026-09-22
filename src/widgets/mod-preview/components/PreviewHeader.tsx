import { useMemo, useRef } from 'react';
import { Box, ChevronRight, Pencil, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { WorkspaceExplorerNode, WorkspaceNode } from '@/entities/workspace';
import { isWorkspaceExplorerNode } from '@/entities/workspace';
import type { useSharedModActions } from '@/features/mod-runtime';
import { buildWorkspaceSwitchPolicy } from '@/features/workspace-runtime';
import { maskWorkspaceNodeCapabilities } from '@/features/workspace-runtime';
import { WorkspaceSwitchControl } from '@/features/workspace-runtime';
import { WorkspaceSwitchLabel } from '@/features/workspace-runtime';
import { useModViewerLaunch } from '@/features/mod-runtime';
import { LiquidSurface } from '@/shared/ui/liquid';
import PreviewPanelContextMenu from './PreviewPanelContextMenu';

type PreviewActions = ReturnType<typeof useSharedModActions>;

interface PreviewHeaderProps {
  selectedFolder: WorkspaceExplorerNode | null | undefined;
  resolvedTitle: string | null;
  resolvedSubtitle: string | null;
  titleDraft: string;
  warningText: string | null;
  warningTooltip: string | null;
  sourceUnavailableMessage: string | null;
  isScrolled: boolean;
  canEdit: boolean;
  actions: PreviewActions;
  onTitleChange: (value: string) => void;
  onBackToGrid: () => void;
  onClearSelection: () => void;
}

export default function PreviewHeader({
  selectedFolder,
  resolvedTitle,
  resolvedSubtitle,
  titleDraft,
  warningText,
  warningTooltip,
  sourceUnavailableMessage,
  isScrolled,
  canEdit,
  actions,
  onTitleChange,
  onBackToGrid,
  onClearSelection,
}: PreviewHeaderProps) {
  const { t } = useTranslation(['preview', 'common']);
  const titleInputRef = useRef<HTMLInputElement>(null);
  const actionFolder = useMemo(
    () => (selectedFolder ? maskWorkspaceNodeCapabilities(selectedFolder, !canEdit) : null),
    [canEdit, selectedFolder],
  );
  const switchPolicy = buildWorkspaceSwitchPolicy(t, actionFolder);
  const pendingDesiredEnabled = actions.getPendingDesiredEnabled(actionFolder);
  const displayedSwitchPolicy =
    pendingDesiredEnabled === undefined
      ? switchPolicy
      : {
          ...switchPolicy,
          checked: pendingDesiredEnabled,
          label: switchPolicy.blocked
            ? switchPolicy.label
            : t(pendingDesiredEnabled ? 'common:status.enabled' : 'common:status.disabled'),
        };
  const modViewer = useModViewerLaunch(actionFolder);

  return (
    <LiquidSurface
      liquidRole="nav"
      className={`sticky top-[var(--workspace-topbar-height)] z-20 -mx-6 mb-5 block transition-[padding,border-color] duration-200 ${
        isScrolled ? 'py-3' : 'pb-2 pt-5'
      }`}
      contentClassName="h-auto px-6"
    >
      <div className="flex items-center justify-between">
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-2">
            <button
              onClick={onBackToGrid}
              aria-label={t('preview:actions.back_to_grid')}
              className={`btn btn-circle btn-ghost text-base-content/50 hover:text-base-content md:hidden ${isScrolled ? 'btn-xs' : 'btn-sm'}`}
            >
              <ChevronRight className="rotate-180" size={isScrolled ? 14 : 16} />
            </button>
            <input
              ref={titleInputRef}
              type="text"
              aria-label={t('preview:actions.rename_mod')}
              title={canEdit ? t('preview:actions.rename_mod') : undefined}
              className={`min-w-0 flex-1 rounded bg-transparent p-0 px-1 -ml-1 tracking-tight text-base-content outline-none transition-[background-color,font-size] duration-150 hover:bg-base-content/5 focus:bg-base-200/50 focus:ring-1 focus:ring-primary ${
                isScrolled ? 'text-sm font-semibold' : 'text-lg font-semibold'
              }`}
              value={titleDraft || ''}
              placeholder={resolvedTitle || t('preview:empty.no_mod_selected')}
              onChange={(event) => onTitleChange(event.target.value)}
              disabled={!canEdit}
            />
            {canEdit && (
              <button
                type="button"
                className="btn btn-circle btn-ghost btn-xs shrink-0 text-base-content/45 hover:text-base-content"
                aria-label={t('preview:actions.rename_mod')}
                title={t('preview:actions.rename_mod')}
                onClick={() => titleInputRef.current?.focus()}
              >
                <Pencil size={13} />
              </button>
            )}
          </div>
        </div>

        <div className="ml-2 flex items-center gap-1">
          {modViewer.visible && (
            <button
              type="button"
              className={`btn btn-circle btn-ghost text-base-content/70 hover:text-base-content ${
                isScrolled ? 'btn-xs' : 'btn-sm'
              }`}
              aria-label={modViewer.actionLabel}
              title={modViewer.tooltip}
              onClick={() => void modViewer.launch()}
            >
              <Box size={isScrolled ? 14 : 16} />
            </button>
          )}
          {actionFolder && (
            <PreviewPanelContextMenu
              folder={actionFolder}
              onRename={() => canEdit && actions.handleRenameRequest(actionFolder)}
              onDelete={() => canEdit && actions.handleDeleteRequest(actionFolder)}
              onToggle={(folder) => canEdit && actions.handleToggleEnabled(folder)}
              onToggleFavorite={(folder) => canEdit && actions.handleToggleFavorite(folder)}
              onEnableOnlyThis={(folder) => canEdit && actions.handleEnableOnlyThis(folder)}
              onOpenMoveDialog={canEdit ? actions.openMoveDialog : undefined}
              onToggleSafe={(folder) => canEdit && actions.handleToggleSafeRequest(folder)}
            />
          )}
          <button
            onClick={onClearSelection}
            aria-label={t('preview:actions.unselect_mod')}
            className={`btn btn-circle btn-ghost hidden text-base-content/30 hover:bg-base-content/5 hover:text-base-content md:inline-flex ${isScrolled ? 'btn-xs' : 'btn-sm'}`}
            title={t('preview:actions.close')}
          >
            <X size={isScrolled ? 16 : 18} />
          </button>
          <button
            onClick={onBackToGrid}
            aria-label={t('preview:actions.close')}
            className={`btn btn-circle btn-ghost text-base-content/30 hover:text-base-content md:hidden ${isScrolled ? 'btn-xs' : 'btn-sm'}`}
          >
            <X size={isScrolled ? 16 : 18} />
          </button>
        </div>
      </div>
      <div
        className={`mt-1 flex min-w-0 items-center gap-2 ${isScrolled ? 'text-[10px]' : 'text-xs'}`}
      >
        <label className="flex shrink-0 cursor-pointer items-center gap-1.5 text-base-content/65 hover:text-base-content">
          <WorkspaceSwitchControl
            node={actionFolder}
            policy={displayedSwitchPolicy}
            isPending={!canEdit || actions.isSwitchPending}
            isBusy={
              isWorkspaceExplorerNode(actionFolder) && actions.isFolderSwitchPending(actionFolder)
            }
            size="xs"
            ariaLabel={t('preview:actions.toggle_enabled')}
            onToggle={(node: WorkspaceNode) => {
              if (isWorkspaceExplorerNode(node)) {
                void actions.handleToggleEnabled(node);
              }
            }}
          />
          <WorkspaceSwitchLabel
            node={actionFolder}
            policy={displayedSwitchPolicy}
            className="font-medium"
          />
        </label>
        {resolvedSubtitle && (
          <span className="min-w-0 flex-1 truncate text-base-content/55" title={resolvedSubtitle}>
            {resolvedSubtitle}
          </span>
        )}
        {(sourceUnavailableMessage || warningText) && (
          <span
            className="max-w-1/2 shrink truncate text-warning/85"
            title={sourceUnavailableMessage ?? warningTooltip ?? warningText ?? undefined}
          >
            {sourceUnavailableMessage ?? warningText}
          </span>
        )}
      </div>
    </LiquidSurface>
  );
}
