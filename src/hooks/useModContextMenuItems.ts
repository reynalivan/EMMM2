import { useTranslation } from 'react-i18next';
import type { ModFolder } from '../types/mod';
import type { WorkspaceExplorerNode } from '../types/workspace';
import {
  buildModContextMenuItems,
  type ModContextMenuActionHandlers,
  type ModContextMenuItemConfig as ContextMenuItemConfig,
} from '../features/mod-runtime/actions/modContextMenuPolicy';

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
  const handlers: ModContextMenuActionHandlers = {
    openExplorer: onOpenExplorer,
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
