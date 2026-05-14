import { useMemo } from 'react';
import { useAppStore } from '../../../stores/useAppStore';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useFolderGridNav } from './useFolderGridNav';
import { useFolderGridBulk } from './useFolderGridBulk';
import { useFolderGridImport } from './useFolderGridImport';
import { useWorkspaceRuntime } from '../../workspace-runtime/state/workspaceStoreBridge';
import { useFolderGridRuntime } from './useFolderGridRuntime';
import { useFolderGridActions } from './useFolderGridActions';
import { useFolderGridSelection } from './useFolderGridSelection';
import { DEFAULT_SOURCE_UNAVAILABLE_MESSAGE } from '../../workspace-runtime/actions/workspaceActionAvailability';

export function useFolderGrid() {
  const {
    currentPath,
    gridSelection,
    clearGridSelection,
    setMobilePane,
    sortField,
    sortOrder,
    setSortField,
    setSortOrder,
    viewMode,
    setViewMode,
    explorerSearchQuery,
    setExplorerSearch,
    explorerSubPath,
    explorerScrollOffset,
    setExplorerScrollOffset,
    isPreviewOpen,
    togglePreview,
    setGridSelection,
    selectedObjectFolderPath,
  } = useAppStore();
  const runtime = useWorkspaceRuntime();
  const { activeGame } = useActiveGame();

  const {
    workspace,
    rawResponse,
    rawFolders,
    sortedFolders,
    isLoading,
    isError,
    error,
    isPlaceholderData,
    isMobile,
    isGridView,
    parentRef,
    virtualItems,
    totalSize,
    scrollToIndex,
    columnCount,
    cardWidth,
  } = useFolderGridRuntime({
    viewMode,
    explorerSubPath,
    explorerScrollOffset,
    setExplorerScrollOffset,
    sortField,
    sortOrder,
    explorerSearchQuery,
  });

  const selfNodeType = rawResponse?.self_node_type || null;
  const selfDisplayMode = rawResponse?.self_display_mode ?? 'unknown';
  const selfIsMod = rawResponse?.self_is_mod ?? false;
  const selfIsEnabled = rawResponse?.self_is_enabled ?? false;
  const selfIsEffectivelyActive = rawResponse?.self_is_effectively_active ?? false;
  const selfReasons = rawResponse?.self_classification_reasons || [];
  const conflicts = rawResponse?.conflicts || [];
  const ancestorDisabledBy = rawResponse?.ancestor_disabled_by ?? null;
  const ancestorDisabledPath = rawResponse?.ancestor_disabled_path ?? null;
  const objects = useMemo(() => workspace?.objects ?? [], [workspace?.objects]);
  const sourceAvailable = workspace?.runtime?.source_state?.status !== 'unavailable';
  const sourceUnavailableMessage = sourceAvailable
    ? null
    : (workspace?.runtime?.source_state?.message ?? DEFAULT_SOURCE_UNAVAILABLE_MESSAGE);

  const nav = useFolderGridNav({
    currentPath,
    explorerSubPath,
    selectedObjectFolderPath,
    sortField,
    sortOrder,
    setSortField,
    setSortOrder,
  });

  const {
    actions,
    switchActions,
    enableParentDialog,
    handleRefresh,
    handleRevealInExplorer,
    currentAbsPath,
    handleOpenCurrentFolderInExplorer,
    handleToggleSelf,
    openEnableParentDialog,
    closeEnableParentDialog,
    handleEnableParent,
    handleToggleEnabledGuarded,
  } = useFolderGridActions({
    activeGame,
    currentPath,
    explorerSubPath,
    ancestorDisabledBy,
    ancestorDisabledPath,
    rawFolders,
    objects,
    clearGridSelection,
    sourceAvailable,
  });

  const bulk = useFolderGridBulk({
    gridSelection,
    sortedFolders,
    clearGridSelection,
    openMoveDialog: actions.openMoveDialog,
  });

  const { focusedId, handleKeyDown, handleToggleSelection, handleActivateItem } =
    useFolderGridSelection({
      sortedFolders,
      gridSelection,
      setGridSelection,
      currentPath,
      isGridView,
      columnCount,
      isMobile,
      scrollToIndex,
      selectMod: runtime.selectMod,
      handleNavigate: nav.handleNavigate,
      handleBreadcrumbClick: nav.handleBreadcrumbClick,
      handleDeleteRequest: actions.handleDeleteRequest,
      handleRenameRequest: actions.handleRenameRequest,
    });

  const { isDragging, handleImportFiles } = useFolderGridImport({
    parentRef,
    activeModPath: activeGame?.mod_path,
    explorerSubPath,
  });

  return {
    rawFolders,
    sortedFolders,
    isLoading,
    isError,
    error,
    isPlaceholderData,
    selfNodeType,
    selfDisplayMode,
    selfIsMod,
    selfIsEnabled,
    selfIsEffectivelyActive,
    selfReasons,
    conflicts,
    ancestorDisabledBy,
    sourceUnavailableMessage,
    enableParentDialogOpen: enableParentDialog.open,
    enableParentDialogAncestorName: enableParentDialog.ancestorName,
    enableParentDialogWillActivate: enableParentDialog.willActivate,
    enableParentDialogStayDisabled: enableParentDialog.stayDisabled,
    openEnableParentDialog,
    closeEnableParentDialog,
    handleEnableParent,
    handleToggleEnabledGuarded,
    isGridView,
    isMobile,
    selectedObjectFolderPath,
    currentPath,
    explorerSearchQuery,
    sortField,
    sortOrder,
    sortLabel: nav.sortLabel,
    viewMode,
    parentRef,
    virtualItems,
    totalSize,
    scrollToIndex,
    columnCount,
    cardWidth,
    handleNavigate: nav.handleNavigate,
    handleBreadcrumbClick: nav.handleBreadcrumbClick,
    handleGoHome: nav.handleGoHome,
    setMobilePane,
    setViewMode,
    setExplorerSearch,
    handleSortToggle: nav.handleSortToggle,
    handleKeyDown,
    focusedId,
    selectedModPath: runtime.state.selectedModPath,
    handleRefresh,
    gridSelection,
    toggleGridSelection: handleToggleSelection,
    activateGridItem: handleActivateItem,
    clearGridSelection,
    ...actions,
    renamingId: actions.renameDialog.folder?.path ?? null,
    handleRevealInExplorer,
    currentAbsPath,
    handleOpenCurrentFolderInExplorer,
    handleToggleSelf,
    duplicateWarning: actions.duplicateWarning,
    handleDuplicateForceEnable: actions.handleDuplicateForceEnable,
    handleDuplicateEnableOnly: actions.handleDuplicateEnableOnly,
    handleDuplicateCancel: actions.handleDuplicateCancel,
    ...bulk,
    objects,
    isDragging,
    handleImportFiles,
    isPreviewOpen,
    togglePreview,
    isSwitchPending: switchActions.isPending,
    isFolderSwitchPending: switchActions.isNodePending,
  };
}
