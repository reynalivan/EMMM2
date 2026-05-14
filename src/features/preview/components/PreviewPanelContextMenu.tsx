import React from 'react';
import { MoreVertical } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ModFolder } from '../../../types/mod';
import type { WorkspaceExplorerNode } from '../../../types/workspace';
import { useModContextMenuItems } from '../../../hooks/useModContextMenuItems';
import { useModContextMenuActions } from '../../mod-runtime/actions/useModContextMenuActions';

interface PreviewPanelContextMenuProps {
  folder: WorkspaceExplorerNode;
  onRename: () => void;
  onDelete: () => void;
  onToggle: (folder: ModFolder) => void;
  onToggleFavorite: (folder: ModFolder) => void;
  onEnableOnlyThis: (folder: ModFolder) => void;
  onOpenMoveDialog?: (folder: ModFolder) => void;
  onToggleSafe: (folder: ModFolder) => void;
}

export default function PreviewPanelContextMenu({
  folder,
  onRename,
  onDelete,
  onToggle,
  onToggleFavorite,
  onEnableOnlyThis,
  onOpenMoveDialog,
  onToggleSafe,
}: PreviewPanelContextMenuProps) {
  const { t } = useTranslation('preview');
  const contextActions = useModContextMenuActions(folder);
  const items = useModContextMenuItems({
    folder,
    onRename,
    onDelete,
    onToggleEnabled: () => onToggle(folder),
    onToggleFavorite: () => onToggleFavorite(folder),
    onEnableOnlyThis: () => onEnableOnlyThis(folder),
    onToggleSafe: () => onToggleSafe(folder),
    onOpenMoveDialog,
    onOpenExplorer: contextActions.openExplorer,
    onPasteThumbnail: contextActions.pasteThumbnailFromClipboard,
    onImportThumbnail: contextActions.importThumbnail,
  });

  return (
    <div className="dropdown dropdown-end">
      <div
        tabIndex={0}
        role="button"
        className="btn btn-ghost btn-sm btn-square text-base-content/70 hover:text-base-content hover:bg-base-content/10"
        title={t('actions.more_actions')}
      >
        <MoreVertical size={16} />
      </div>
      <ul
        tabIndex={0}
        className="dropdown-content z-100 menu p-2 shadow-xl bg-base-100/95 backdrop-blur-xl rounded-box w-56 border border-base-content/10 mt-2"
      >
        {items.map((item) => {
          if (item.hidden) return null;

          return (
            <React.Fragment key={item.id}>
              {item.separatorBefore && <div className="divider my-0"></div>}
              <li>
                <button
                  onClick={() => {
                    item.onClick();
                    // Optional: Blur the active element to close the daisyUI dropdown
                    if (document.activeElement instanceof HTMLElement) {
                      document.activeElement.blur();
                    }
                  }}
                  className={item.danger ? 'text-error hover:text-error hover:bg-error/10' : ''}
                >
                  <item.icon size={14} className="opacity-70" /> {item.label}
                </button>
              </li>
            </React.Fragment>
          );
        })}
      </ul>
    </div>
  );
}
