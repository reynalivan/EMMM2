import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../../stores/useAppStore';
import { useThumbnail } from '../../../hooks/useThumbnail';
import { formatWorkspaceWarning } from '../../workspace-runtime/workspaceSemantics';
import { buildWorkspaceSwitchPolicy } from '../../workspace-runtime/actions/workspaceSwitchPolicy';
import { maskWorkspaceNodeCapabilities } from '../../workspace-runtime/actions/workspaceActionAvailability';
import { getFolderTypeChip } from '../utils/FolderTypeChip';
import type { WorkspaceExplorerNode } from '../../../types/workspace';

interface FolderNodeBulkHandlers {
  onBulkToggle?: (enable: boolean) => void;
  onBulkDelete?: () => void;
  onBulkTag?: () => void;
  onBulkFavorite?: (favorite: boolean) => void;
  onBulkSafe?: (safe: boolean) => void;
  onBulkPin?: (pin: boolean) => void;
  onBulkMoveToObject?: () => void;
}

interface UseFolderNodeViewOptions {
  node: WorkspaceExplorerNode;
  variant: 'card' | 'row';
  isSelected: boolean;
  selectionSize: number;
  mutationsDisabled: boolean;
  toggleSelection: (id: string, multi: boolean, isShift?: boolean) => void;
  onActivate?: (path: string) => void;
  bulk: FolderNodeBulkHandlers;
}

/**
 * useFolderNodeView — shared setup for FolderCard and FolderListRow: type chip,
 * capability masking, switch policy, lazy thumbnail with error/loaded reset,
 * bulk context-menu props, and the click/selection handler. The components keep
 * only their JSX differences.
 */
export function useFolderNodeView({
  node,
  variant,
  isSelected,
  selectionSize,
  mutationsDisabled,
  toggleSelection,
  onActivate,
  bulk,
}: UseFolderNodeViewOptions) {
  const { t } = useTranslation(['grid', 'common']);
  const typeChip = getFolderTypeChip(node.type_chip, t, variant);
  const actionNode = useMemo(
    () => maskWorkspaceNodeCapabilities(node, mutationsDisabled),
    [node, mutationsDisabled],
  );
  const primaryWarningText = formatWorkspaceWarning(t, node.primary_warning);
  const switchPolicy = useMemo(() => buildWorkspaceSwitchPolicy(t, actionNode), [actionNode, t]);

  // Lazy thumbnail: resolved per-node via separate backend command
  const activeGameId = useAppStore((state) => state.activeGameId);
  const { data: thumbnailPath, isLoading: thumbLoading } = useThumbnail(
    activeGameId || '',
    node.path,
  );
  const [imgError, setImgError] = useState(false);
  const [imgLoaded, setImgLoaded] = useState(false);

  // Reset image state when the thumbnail path changes (e.g. after lazy resolve or
  // update). Adjusted during render rather than in an effect so the new path never
  // commits a frame with the previous image's error/loaded flags.
  const [prevThumbnailPath, setPrevThumbnailPath] = useState(thumbnailPath);
  if (thumbnailPath !== prevThumbnailPath) {
    setPrevThumbnailPath(thumbnailPath);
    setImgError(false);
    setImgLoaded(false);
  }

  const thumbnailSrc = thumbnailPath && !imgError ? thumbnailPath : null;
  const isBulkSelection =
    isSelected && selectionSize > 1 && useAppStore.getState().activePane === 'folderGrid';
  const bulkMenuProps = {
    count: selectionSize,
    onToggle: mutationsDisabled ? undefined : bulk.onBulkToggle,
    onDelete: mutationsDisabled ? undefined : bulk.onBulkDelete,
    onTag: mutationsDisabled ? undefined : bulk.onBulkTag,
    onFavorite: mutationsDisabled ? undefined : bulk.onBulkFavorite,
    onSafe: mutationsDisabled ? undefined : bulk.onBulkSafe,
    onPin: mutationsDisabled ? undefined : bulk.onBulkPin,
    onMoveToObject: mutationsDisabled ? undefined : bulk.onBulkMoveToObject,
  };

  const handleClick = (e: React.MouseEvent) => {
    if (node.display_mode === 'internal_assets') {
      return;
    }

    if (e.ctrlKey || e.shiftKey) {
      toggleSelection(node.path, true, e.shiftKey);
    } else {
      onActivate?.(node.path);
    }
  };

  return {
    t,
    typeChip,
    actionNode,
    primaryWarningText,
    switchPolicy,
    thumbnailSrc,
    thumbLoading,
    imgError,
    setImgError,
    imgLoaded,
    setImgLoaded,
    isBulkSelection,
    bulkMenuProps,
    handleClick,
  };
}
