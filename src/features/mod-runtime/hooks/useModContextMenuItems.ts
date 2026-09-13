import { useTranslation } from 'react-i18next';
import type { ModFolder } from '@/entities/game-object';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import {
  buildModContextMenuItems,
  type ModContextMenuActionHandlers,
  type ModContextMenuItemConfig as ContextMenuItemConfig,
} from '../actions/modContextMenuPolicy';
import { useModViewerLaunch } from '../actions/useModViewerLaunch';

export interface UseModContextMenuItemsProps {
  folder: WorkspaceExplorerNode;
  onRename: () => void;
  onDelete: () => void;
  onToggleEnabled: () => void;
  onToggleFavorite: () => void;
  onEnableOnlyThis?: () => void;
  onToggleSafe?: () => void;
  onOpenMoveDialog?: (folder: ModFolder) => void;
  onNavigateModPack?: (folderName: string) => void;
  onSyncWithDb?: () => void;
  onOpenExplorer?: () => void;
  onPasteThumbnail?: () => void;
  onImportThumbnail?: () => void;
}

export function useModContextMenuItems({
  folder,
  onRename,
  onDelete,
  onToggleEnabled,
  onToggleFavorite,
  onEnableOnlyThis,
  onToggleSafe,
  onOpenMoveDialog,
  onNavigateModPack,
  onSyncWithDb,
  onOpenExplorer,
  onPasteThumbnail,
  onImportThumbnail,
}: UseModContextMenuItemsProps): ContextMenuItemConfig[] {
  const { t } = useTranslation(['grid']);
  const modViewer = useModViewerLaunch(folder);
  const handlers: ModContextMenuActionHandlers = {
    openExplorer: onOpenExplorer,
    openModViewer: modViewer.visible ? () => void modViewer.launch() : undefined,
    modViewerExperimental: modViewer.experimental,
    rename: onRename,
    toggleEnabled: onToggleEnabled,
    enableOnlyThis: onEnableOnlyThis,
    toggleFavorite: onToggleFavorite,
    pasteThumbnail: onPasteThumbnail,
    importThumbnail: onImportThumbnail,
    toggleSafe: onToggleSafe,
    moveToObject: onOpenMoveDialog ? () => onOpenMoveDialog(folder) : undefined,
    navigateModPack: onNavigateModPack ? () => onNavigateModPack(folder.folder_name) : undefined,
    syncWithDb: onSyncWithDb,
    delete: onDelete,
  };

  return buildModContextMenuItems(t, folder, handlers);
}
