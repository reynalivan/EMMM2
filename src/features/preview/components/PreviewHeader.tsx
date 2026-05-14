import { useMemo } from 'react';
import { ChevronRight, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { WorkspaceExplorerNode, WorkspaceNode } from '../../../types/workspace';
import type { useSharedModActions } from '../../mod-runtime/actions/useSharedModActions';
import { buildWorkspaceSwitchPolicy } from '../../workspace-runtime/actions/workspaceSwitchPolicy';
import { maskWorkspaceNodeCapabilities } from '../../workspace-runtime/actions/workspaceActionAvailability';
import { WorkspaceSwitchControl } from '../../workspace-runtime/components/WorkspaceSwitchControl';
import { WorkspaceSwitchLabel } from '../../workspace-runtime/components/WorkspaceSwitchLabel';
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
  const actionFolder = useMemo(
    () => (selectedFolder ? maskWorkspaceNodeCapabilities(selectedFolder, !canEdit) : null),
    [canEdit, selectedFolder],
  );
  const switchPolicy = buildWorkspaceSwitchPolicy(t, actionFolder);

  return (
    <div
      className={`sticky top-0 z-20 -mx-6 mb-6 px-6 transition-all duration-200 flex flex-col justify-center border-b ${
        isScrolled
          ? 'pt-4 pb-2 bg-base-100/95 backdrop-blur-md border-base-content/10 shadow-sm'
          : 'pt-6 pb-2 bg-transparent border-transparent'
      }`}
    >
      <div className="flex items-center justify-between transition-all duration-200">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 transition-all duration-200 mb-0">
            <button
              onClick={onBackToGrid}
              aria-label={t('preview:actions.back_to_grid')}
              className={`btn btn-circle btn-ghost text-base-content/50 hover:text-base-content md:hidden transition-all duration-200 ${isScrolled ? 'btn-xs' : 'btn-sm'}`}
            >
              <ChevronRight className="rotate-180" size={isScrolled ? 14 : 16} />
            </button>
            <input
              type="text"
              className={`bg-transparent p-0 m-0 border-none outline-none focus:ring-1 focus:ring-primary focus:bg-base-200/50 rounded px-1 -ml-1 truncate tracking-tight text-base-content transition-all duration-200 origin-left hover:bg-base-content/5 ${
                isScrolled ? 'text-sm font-semibold' : 'text-xl font-bold'
              }`}
              value={titleDraft || ''}
              placeholder={resolvedTitle || t('preview:empty.no_mod_selected')}
              onChange={(event) => onTitleChange(event.target.value)}
              disabled={!canEdit}
            />
          </div>
          {resolvedSubtitle && (
            <p
              className={`truncate text-base-content/50 transition-all duration-200 ${
                isScrolled ? 'mt-0.5 text-[10px]' : 'mt-1 text-xs'
              }`}
              title={resolvedSubtitle}
            >
              {resolvedSubtitle}
            </p>
          )}
          <label
            className={`label cursor-pointer justify-start gap-2 p-0 opacity-80 hover:opacity-100 transition-all duration-200 ${isScrolled ? '-mt-0.5' : 'mt-1'}`}
          >
            <WorkspaceSwitchControl
              node={actionFolder}
              policy={switchPolicy}
              isPending={
                !canEdit || actions.isSwitchPending || actions.isFolderSwitchPending(actionFolder)
              }
              size={isScrolled ? 'xs' : 'sm'}
              ariaLabel={t('preview:actions.toggle_enabled')}
              onToggle={(node: WorkspaceNode) => {
                if (node.node_kind !== 'object') {
                  void actions.handleToggleEnabled(node);
                }
              }}
            />
            <WorkspaceSwitchLabel
              node={actionFolder}
              policy={switchPolicy}
              className={`font-medium text-base-content/60 transition-all duration-200 ${isScrolled ? 'text-[10px]' : 'text-sm'}`}
            />
          </label>
          {sourceUnavailableMessage && (
            <p className="mt-1 truncate text-warning/80 text-xs" title={sourceUnavailableMessage}>
              {sourceUnavailableMessage}
            </p>
          )}
          {warningText && (
            <p
              className={`mt-1 truncate text-warning/80 transition-all duration-200 ${
                isScrolled ? 'text-[10px]' : 'text-xs'
              }`}
              title={warningTooltip ?? warningText}
            >
              {warningText}
            </p>
          )}
        </div>

        <div className="ml-2 flex items-center gap-1">
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
            className={`btn btn-circle btn-ghost hidden text-base-content/30 hover:bg-base-content/5 hover:text-base-content md:inline-flex transition-all duration-200 ${isScrolled ? 'btn-xs' : 'btn-sm'}`}
            title={t('preview:actions.close')}
          >
            <X size={isScrolled ? 16 : 18} />
          </button>
          <button
            onClick={onBackToGrid}
            aria-label={t('preview:actions.close')}
            className={`btn btn-circle btn-ghost text-base-content/30 hover:text-base-content md:hidden transition-all duration-200 ${isScrolled ? 'btn-xs' : 'btn-sm'}`}
          >
            <X size={isScrolled ? 16 : 18} />
          </button>
        </div>
      </div>
    </div>
  );
}
