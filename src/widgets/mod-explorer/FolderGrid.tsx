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

export default function FolderGrid() {
  const folderGrid = useFolderGrid();
  const {
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
    sortOrder,
    sortLabel,
    viewMode,
    handleBreadcrumbClick,
    handleGoHome,
    setMobilePane,
    setViewMode,
    setExplorerSearch,
    handleSortToggle,
    handleKeyDown,
    gridSelection,
    clearGridSelection,
    handleToggleSelf,
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

  return (
    <div
      data-testid="folder-grid"
      className={cn(
        'flex min-h-0 min-w-0 flex-col h-full bg-transparent p-4 relative outline-none transition-shadow duration-200',
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

      <FolderGridToolbar
        isMobile={isMobile}
        currentPath={currentPath}
        handleBreadcrumbClick={handleBreadcrumbClick}
        handleGoHome={handleGoHome}
        setMobilePane={setMobilePane}
        handleSortToggle={handleSortToggle}
        sortLabel={sortLabel}
        sortOrder={sortOrder}
        viewMode={viewMode}
        setViewMode={setViewMode}
        explorerSearchQuery={explorerSearchQuery}
        setExplorerSearch={setExplorerSearch}
        visibleCount={visibleFolders.length}
      />

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
