import React from 'react';
import { ContextMenuItem, ContextMenuSeparator } from '../../components/ui/ContextMenu';
import type { ModFolder } from '../../types/mod';
import type { WorkspaceExplorerNode } from '../../types/workspace';
import { useModContextMenuItems } from '../../hooks/useModContextMenuItems';
import { useModContextMenuActions } from '../mod-runtime/actions/useModContextMenuActions';

interface FolderCardContextMenuProps {
  folder: WorkspaceExplorerNode;
  onRename: () => void;
  onDelete: () => void;
  onToggle: () => void;
  onToggleFavorite: () => void;
  onEnableOnlyThis?: () => void;
  onOpenMoveDialog?: (folder: ModFolder) => void;
  onNavigate?: (folderName: string) => void;
  onToggleSafe?: () => void;
  onSyncWithDb?: () => void;
}

export default function FolderCardContextMenu({
  folder,
  onRename,
  onDelete,
  onToggle,
  onToggleFavorite,
  onEnableOnlyThis,
  onOpenMoveDialog,
  onNavigate,
  onToggleSafe,
  onSyncWithDb,
}: FolderCardContextMenuProps) {
  const contextActions = useModContextMenuActions(folder);
  const items = useModContextMenuItems({
    folder,
    onRename,
    onDelete,
    onToggleEnabled: onToggle,
    onToggleFavorite,
    onEnableOnlyThis,
    onToggleSafe,
    onOpenMoveDialog,
    onNavigateModPack: onNavigate,
    onSyncWithDb,
    onOpenExplorer: contextActions.openExplorer,
    onPasteThumbnail: contextActions.pasteThumbnailFromClipboard,
    onImportThumbnail: contextActions.importThumbnail,
  });

  return (
    <>
      {items.map((item) => {
        if (item.hidden) return null;

        return (
          <React.Fragment key={item.id}>
            {item.separatorBefore && <ContextMenuSeparator />}
            <ContextMenuItem icon={item.icon} danger={item.danger} onClick={item.onClick}>
              {item.label}
            </ContextMenuItem>
          </React.Fragment>
        );
      })}
    </>
  );
}
