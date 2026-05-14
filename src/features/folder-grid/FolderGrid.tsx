import FolderGridToolbar from './FolderGridToolbar';
import FolderGridBanners from './FolderGridBanners';
import FolderGridModals from './FolderGridModals';
import DragOverlay from './DragOverlay';
import EnableParentDialog from './EnableParentDialog';
import BulkProgressBar from './BulkProgressBar';
import BulkActionBar from './BulkActionBar';
import { useFolderGrid } from './hooks/useFolderGrid';
import { cn } from '../../lib/utils';
import { useFolderGridViewModel } from './useFolderGridViewModel';
import FolderGridStateViews from './FolderGridStateViews';
import FolderGridContent from './FolderGridContent';

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
    conflicts: nameConflicts,
    sourceUnavailableMessage,
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
    handleRefresh,
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
    bulkDeleteConfirm,
    setBulkDeleteConfirm,
    handleBulkToggle,
    handleBulkTagRequest,
    handleBulkDeleteRequest,
    handleBulkDeleteConfirm,
    handleBulkFavorite,
    handleBulkSafe,
    handleBulkPin,
    pinSafeDialog,
    handleToggleSafeSubmit,
    handleToggleSafeCancel,
    activeContextDialog,
    handleActiveContextCancel,
    handleActiveContextSubmit,
    syncConfirm,
    closeSyncConfirm,
    handleApplySyncMatch,
    isDragging,
    handleImportFiles,
  } = folderGrid;

  const isFlatModRoot = selfDisplayMode === 'flat_mod' || selfIsMod;
  const {
    visibleFolders,
    conflictPathSet,
    activePane,
    setActivePane,
    isIgnoreManagementOpen,
    setIsIgnoreManagementOpen,
    workspaceSourceUnavailableMessage,
    mutationsDisabled,
    handleSelectAll,
  } = useFolderGridViewModel({ sortedFolders, sourceUnavailableMessage });

  return (
    <div
      className={cn(
        'flex flex-col h-full bg-transparent p-4 relative outline-none transition-shadow duration-200',
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
        nameConflicts={nameConflicts}
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
        handleRefresh={handleRefresh}
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
        gridSelection={gridSelection}
        isIgnoreManagementOpen={isIgnoreManagementOpen}
        setIsIgnoreManagementOpen={setIsIgnoreManagementOpen}
        pinSafeDialog={pinSafeDialog}
        handleToggleSafeCancel={handleToggleSafeCancel}
        handleToggleSafeSubmit={handleToggleSafeSubmit}
        activeContextDialog={activeContextDialog}
        handleActiveContextCancel={handleActiveContextCancel}
        handleActiveContextSubmit={handleActiveContextSubmit}
        syncConfirm={syncConfirm}
        handleCloseSyncConfirm={closeSyncConfirm}
        handleApplySyncMatch={handleApplySyncMatch}
        objectId={undefined}
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
        mutationsDisabled={mutationsDisabled}
      />

      {/* Drag Overlay */}
      {isDragging && <DragOverlay isDragging={isDragging} />}
    </div>
  );
}
