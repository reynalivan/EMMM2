import { useLayoutEffect, useRef } from 'react';
import FolderGridToolbar from './components/FolderGridToolbar';
import FolderGridBanners from './components/FolderGridBanners';
import FolderGridModals from './modals/FolderGridModals';
import DragOverlay from './components/DragOverlay';
import EnableParentDialog from './modals/EnableParentDialog';
import BulkProgressBar from './components/BulkProgressBar';
import BulkActionBar from './components/BulkActionBar';
import { useFolderGrid } from './hooks/useFolderGrid';
import { cn } from '../../shared/lib/utils';
import { useFolderGridViewModel } from './hooks/useFolderGridViewModel';
import FolderGridStateViews from './components/FolderGridStateViews';
import FolderGridContent from './components/FolderGridContent';
import FolderGridFooter from './components/FolderGridFooter';

export default function FolderGrid() {
  const rootRef = useRef<HTMLDivElement>(null);
  const chromeRef = useRef<HTMLDivElement>(null);
  const folderGrid = useFolderGrid();
  const {
    previousFolders,
    sortedFolders,
    isLoading,
    isError,
    error,
    selfDisplayMode,
    selfIsMod,
    selfIsEnabled,
    selfIsEffectivelyActive,
    selfReasons,
    sourceUnavailableMessage,
    recoveryStatus,
    isMobile,
    currentPath,
    explorerSearchQuery,
    sortField,
    sortOrder,
    setSortField,
    setSortOrder,
    viewMode,
    handleNavigate,
    handleBreadcrumbClick,
    handleGoHome,
    setMobilePane,
    setViewMode,
    setExplorerSearch,
    handleKeyDown,
    gridSelection,
    clearGridSelection,
    handleToggleSelf,
    isCreateFolderOpen,
    isCreatingFolder,
    openCreateFolderDialog,
    closeCreateFolderDialog,
    handleCreateFolder,
    handleMoveToObject,
    moveDialog,
    closeMoveDialog,
    objects,
    ancestorDisabledBy,
    enableParentDialogOpen,
    enableParentDialogAncestorName,
    enableParentDialogWillActivate,
    enableParentDialogStayDisabled,
    openEnableParentDialog,
    closeEnableParentDialog,
    handleEnableParent,
    deleteConfirm,
    setDeleteConfirm,
    handleDeleteConfirm,
    isPreviewOpen,
    togglePreview,
    bulkTagOpen,
    setBulkTagOpen,
    handleBulkTagSubmit,
    bulkDeleteConfirm,
    setBulkDeleteConfirm,
    handleBulkToggle,
    handleBulkTagRequest,
    handleBulkDeleteRequest,
    handleBulkDeleteConfirm,
    handleBulkFavorite,
    handleBulkSafe,
    handleBulkPin,
    handleBulkMoveToObject,
    activeContextDialog,
    handleActiveContextCancel,
    handleActiveContextSubmit,
    isDragging,
    handleImportFiles,
  } = folderGrid;

  const isFlatModRoot = selfDisplayMode === 'flat_mod' || selfIsMod;
  const {
    visibleFolders,
    conflictPathSet,
    folderConflictScopes,
    activePane,
    setActivePane,
    isIgnoreManagementOpen,
    setIsIgnoreManagementOpen,
    workspaceSourceUnavailableMessage,
    mutationsDisabled,
    handleSelectAll,
  } = useFolderGridViewModel({ sortedFolders, sourceUnavailableMessage, recoveryStatus });

  useLayoutEffect(() => {
    const root = rootRef.current;
    const chrome = chromeRef.current;
    if (!root || !chrome) return;

    const syncChromeInset = () => {
      root.style.setProperty('--folder-grid-chrome-height', `${chrome.offsetHeight}px`);
    };

    syncChromeInset();
    const observer = new ResizeObserver(syncChromeInset);
    observer.observe(chrome);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      ref={rootRef}
      data-testid="folder-grid"
      className={cn(
        'folder-grid-container relative flex h-full min-h-0 min-w-0 flex-col bg-transparent outline-none transition-shadow duration-200',
        activePane === 'folderGrid' && 'ring-1 ring-inset ring-primary/20',
      )}
      onKeyDown={(e) => {
        if (activePane !== 'folderGrid') return;

        if (e.key === 'Escape' && gridSelection.size > 0) {
          e.preventDefault();
          clearGridSelection();
          return;
        }

        if (e.key === 'Delete' && gridSelection.size > 0 && !mutationsDisabled) {
          e.preventDefault();
          handleBulkDeleteRequest();
          return;
        }

        handleKeyDown(e);
      }}
      tabIndex={-1}
      onFocus={(e) => {
        if (!e.defaultPrevented) setActivePane('folderGrid');
      }}
    >
      <div className="shrink-0 px-4">
        <FolderGridBanners
          isLoading={isLoading}
          isError={isError}
          isFlatModRoot={isFlatModRoot}
          selfIsEnabled={selfIsEffectivelyActive || selfIsEnabled}
          selfReasons={selfReasons}
          isMobile={isMobile}
          isPreviewOpen={isPreviewOpen}
          setMobilePane={setMobilePane}
          togglePreview={togglePreview}
          handleToggleSelf={handleToggleSelf}
          ancestorDisabledBy={ancestorDisabledBy}
          currentPath={currentPath}
          onOpenEnableParentDialog={openEnableParentDialog}
          diskSourceUnavailableMessage={workspaceSourceUnavailableMessage}
          recoveryStatus={recoveryStatus}
          mutationsDisabled={mutationsDisabled}
        />
      </div>

      <div ref={chromeRef} className="shrink-0">
        <FolderGridToolbar
          isMobile={isMobile}
          currentPath={currentPath}
          handleBreadcrumbClick={handleBreadcrumbClick}
          previousFolderItems={previousFolders}
          handleNavigate={handleNavigate}
          handleGoHome={handleGoHome}
          setMobilePane={setMobilePane}
          sortField={sortField}
          sortOrder={sortOrder}
          setSortField={setSortField}
          setSortOrder={setSortOrder}
          viewMode={viewMode}
          setViewMode={setViewMode}
          explorerSearchQuery={explorerSearchQuery}
          setExplorerSearch={setExplorerSearch}
          canCreateFolder={!mutationsDisabled && folderGrid.currentFolderPath !== null}
          onCreateFolder={openCreateFolderDialog}
        />
      </div>

      <FolderGridStateViews
        isLoading={isLoading}
        isError={isError}
        error={error}
        visibleCount={visibleFolders.length}
        isFlatModRoot={isFlatModRoot}
        explorerSearchQuery={explorerSearchQuery}
        currentPath={currentPath}
        setExplorerSearch={setExplorerSearch}
        handleBreadcrumbClick={handleBreadcrumbClick}
        handleImportFiles={handleImportFiles}
      />

      <FolderGridContent
        model={folderGrid}
        visibleFolders={visibleFolders}
        conflictPathSet={conflictPathSet}
        folderConflictScopes={folderConflictScopes}
        mutationsDisabled={mutationsDisabled}
        onSelectAll={handleSelectAll}
      />

      <div className="pointer-events-none absolute inset-x-0 bottom-0 z-20 px-4 pb-2">
        <FolderGridFooter visibleCount={visibleFolders.length} />
      </div>

      <FolderGridModals
        moveDialog={moveDialog}
        closeMoveDialog={closeMoveDialog}
        handleMoveToObject={handleMoveToObject}
        deleteConfirm={deleteConfirm}
        setDeleteConfirm={setDeleteConfirm}
        handleDeleteConfirm={handleDeleteConfirm}
        bulkDeleteConfirm={bulkDeleteConfirm}
        setBulkDeleteConfirm={setBulkDeleteConfirm}
        handleBulkDeleteConfirm={handleBulkDeleteConfirm}
        bulkTagOpen={bulkTagOpen}
        setBulkTagOpen={setBulkTagOpen}
        handleBulkTagSubmit={handleBulkTagSubmit}
        gridSelection={gridSelection}
        isIgnoreManagementOpen={isIgnoreManagementOpen}
        setIsIgnoreManagementOpen={setIsIgnoreManagementOpen}
        activeContextDialog={activeContextDialog}
        handleActiveContextCancel={handleActiveContextCancel}
        handleActiveContextSubmit={handleActiveContextSubmit}
        currentPath={typeof currentPath === 'string' ? currentPath : undefined}
        objects={objects}
        createFolderOpen={isCreateFolderOpen}
        existingFolderNames={folderGrid.rawFolders?.map((folder) => folder.name) ?? []}
        isCreatingFolder={isCreatingFolder}
        closeCreateFolderDialog={closeCreateFolderDialog}
        handleCreateFolder={handleCreateFolder}
      />

      {/* Enable Parent Dialog */}
      {ancestorDisabledBy && (
        <EnableParentDialog
          open={enableParentDialogOpen}
          onClose={closeEnableParentDialog}
          ancestorName={enableParentDialogAncestorName}
          willActivate={enableParentDialogWillActivate}
          stayDisabled={enableParentDialogStayDisabled}
          onConfirm={handleEnableParent}
        />
      )}

      <BulkProgressBar />

      <BulkActionBar
        count={gridSelection.size}
        onClear={clearGridSelection}
        onToggle={handleBulkToggle}
        onDelete={handleBulkDeleteRequest}
        onPin={handleBulkPin}
        onFavorite={handleBulkFavorite}
        onMarkSafe={handleBulkSafe}
        onUpdateInfo={handleBulkTagRequest}
        onMoveToObject={handleBulkMoveToObject}
        mutationsDisabled={mutationsDisabled}
      />

      {/* Drag Overlay */}
      {isDragging && <DragOverlay isDragging={isDragging} />}
    </div>
  );
}
