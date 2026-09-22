import { useMemo } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useAppStore } from '@/app/store';
import { useActiveGame } from '@/entities/game';
import { useFolderGridNav } from './useFolderGridNav';
import { useFolderGridBulk } from './useFolderGridBulk';
import { useFolderGridImport } from './useFolderGridImport';
import { useWorkspaceRuntime } from '@/features/workspace-runtime';
import { useFolderGridRuntime } from './useFolderGridRuntime';
import { useFolderGridActions } from './useFolderGridActions';
import { useFolderGridSelection } from './useFolderGridSelection';
import { DEFAULT_SOURCE_UNAVAILABLE_MESSAGE } from '@/features/workspace-runtime';
import { useWorkspaceExplorerSelection } from './useWorkspaceExplorerSelection';

export function useFolderGrid() {
  // Selector-scoped: a bare useAppStore() here re-runs the whole grid
  // computation on any write to any slice.
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
  } = useAppStore(
    useShallow((state) => ({
      currentPath: state.currentPath,
      gridSelection: state.gridSelection,
      clearGridSelection: state.clearGridSelection,
      setMobilePane: state.setMobilePane,
      sortField: state.sortField,
      sortOrder: state.sortOrder,
      setSortField: state.setSortField,
      setSortOrder: state.setSortOrder,
      viewMode: state.viewMode,
      setViewMode: state.setViewMode,
      explorerSearchQuery: state.explorerSearchQuery,
      setExplorerSearch: state.setExplorerSearch,
      explorerSubPath: state.explorerSubPath,
      explorerScrollOffset: state.explorerScrollOffset,
      setExplorerScrollOffset: state.setExplorerScrollOffset,
      isPreviewOpen: state.isPreviewOpen,
      togglePreview: state.togglePreview,
      setGridSelection: state.setGridSelection,
      selectedObjectFolderPath: state.selectedObjectFolderPath,
    })),
  );
  const runtime = useWorkspaceRuntime();
  const { activeGame } = useActiveGame();

  const {
    workspace,
    rawResponse,
    rawFolders,
    previousFolders,
    hasMorePreviousFolders,
    isLoadingMorePreviousFolders,
    loadMorePreviousFolders,
    sortedFolders,
    isLoading,
    isRefreshing,
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
    explorerQuery,
    isExplorerSearchPending,
    totalMatching,
    listingRevision,
  } = useFolderGridRuntime({
    activeGameId: activeGame?.id,
    viewMode,
    currentPath,
    explorerSubPath,
    explorerScrollOffset,
    setExplorerScrollOffset,
    sortField,
    sortOrder,
    explorerSearchQuery,
  });
  const loadedExplorerPaths = useMemo(
    () => sortedFolders.map((folder) => folder.path),
    [sortedFolders],
  );

  const explorerSelection = useWorkspaceExplorerSelection({
    query: explorerQuery,
    selectionScopeKey: explorerSearchQuery,
    listingRevision,
    totalMatching,
    loadedPaths: loadedExplorerPaths,
    explicitPaths: gridSelection,
    setExplicitPaths: setGridSelection,
    clearExplicitPaths: clearGridSelection,
  });

  const selfNodeType = rawResponse?.self_node_type || null;
  const selfDisplayMode = rawResponse?.self_display_mode ?? 'unknown';
  const selfIsMod = rawResponse?.self_is_mod ?? false;
  const selfIsEnabled = rawResponse?.self_is_enabled ?? false;
  const selfIsEffectivelyActive = rawResponse?.self_is_effectively_active ?? false;
  const selfReasons = rawResponse?.self_classification_reasons || [];
  const ancestorDisabledBy = rawResponse?.ancestor_disabled_by ?? null;
  const ancestorDisabledPath = rawResponse?.ancestor_disabled_path ?? null;
  const objects = useMemo(() => workspace?.objects ?? [], [workspace?.objects]);
  const sourceAvailable = workspace?.runtime?.source_state?.status !== 'unavailable';
  const sourceUnavailableMessage = sourceAvailable
    ? null
    : (workspace?.runtime?.source_state?.message ?? DEFAULT_SOURCE_UNAVAILABLE_MESSAGE);
  const recoveryStatus = workspace?.runtime?.recovery_status ?? 'ready';

  const nav = useFolderGridNav({
    currentPath,
    explorerSubPath,
    selectedObjectFolderPath,
  });

  const {
    actions,
    handleRevealInExplorer,
    currentFolderPath,
    handleOpenCurrentFolderInExplorer,
    isCreateFolderOpen,
    isCreatingFolder,
    openCreateFolderDialog,
    closeCreateFolderDialog,
    handleCreateFolder,
    handleToggleSelf,
    openEnableParentDialog,
    handleToggleEnabledGuarded,
  } = useFolderGridActions({
    activeGame,
    explorerSubPath,
    ancestorDisabledPath,
    objects,
    clearGridSelection: explorerSelection.clearSelection,
    sourceAvailable,
  });

  const bulk = useFolderGridBulk({
    selection: explorerSelection.selection,
    explorerQuery,
    listingRevision,
    selectionStable: !isExplorerSearchPending,
    sortedFolders,
    clearGridSelection: explorerSelection.clearSelection,
    removeGridSelectionPaths: explorerSelection.removePaths,
    openMoveDialog: actions.openMoveDialog,
  });

  const { focusedId, handleKeyDown, handleToggleSelection, handleActivateItem } =
    useFolderGridSelection({
      sortedFolders,
      selectedModPath: runtime.state.selectedModPath,
      selection: explorerSelection.selection,
      addSelectionPaths: explorerSelection.addPaths,
      toggleSelectionPath: explorerSelection.togglePath,
      clearSelection: explorerSelection.clearSelection,
      selectAllMatching: explorerSelection.selectAllMatching,
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
    activeGameId: activeGame?.id,
    activeModPath: activeGame?.mod_path,
    selectedObjectFolderPath,
    explorerSubPath,
  });

  return {
    rawFolders,
    previousFolders,
    hasMorePreviousFolders,
    isLoadingMorePreviousFolders,
    loadMorePreviousFolders,
    sortedFolders,
    isLoading,
    isRefreshing,
    isError,
    error,
    isPlaceholderData,
    selfNodeType,
    selfDisplayMode,
    selfIsMod,
    selfIsEnabled,
    selfIsEffectivelyActive,
    selfReasons,
    ancestorDisabledBy,
    sourceUnavailableMessage,
    recoveryStatus,
    openEnableParentDialog,
    handleToggleEnabledGuarded,
    isGridView,
    isMobile,
    selectedObjectFolderPath,
    currentPath,
    explorerSearchQuery,
    explorerQuery,
    isExplorerSearchPending,
    totalMatching,
    sortField,
    sortOrder,
    setSortField,
    setSortOrder,
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
    handleKeyDown,
    focusedId,
    selectedModPath: runtime.state.selectedModPath,
    gridSelection,
    explorerSelection: explorerSelection.selection,
    selectedCount: explorerSelection.selectedCount,
    isPathSelected: explorerSelection.isPathSelected,
    selectAllMatching: explorerSelection.selectAllMatching,
    toggleGridSelection: handleToggleSelection,
    activateGridItem: handleActivateItem,
    clearGridSelection: explorerSelection.clearSelection,
    ...actions,
    renamingId: actions.renameDialog.folder?.path ?? null,
    handleRevealInExplorer,
    currentFolderPath,
    handleOpenCurrentFolderInExplorer,
    isCreateFolderOpen,
    isCreatingFolder,
    openCreateFolderDialog,
    closeCreateFolderDialog,
    handleCreateFolder,
    handleToggleSelf,
    ...bulk,
    objects,
    isDragging,
    handleImportFiles,
    isPreviewOpen,
    togglePreview,
    isSwitchPending: actions.isSwitchPending,
    isFolderSwitchPending: actions.isFolderSwitchPending,
    getFolderPendingDesiredEnabled: actions.getPendingDesiredEnabled,
  };
}
