import { useMemo } from 'react';
import FolderGridToolbar from './FolderGridToolbar';
import FolderGridBanners from './FolderGridBanners';
import FolderGridEmpty from './FolderGridEmpty';
import FolderGridModals from './FolderGridModals';
import FolderCard from './FolderCard';
import FolderListRow from './FolderListRow';
import DragOverlay from './DragOverlay';
import { Loader2 } from 'lucide-react';
import BulkProgressBar from './BulkProgressBar';
import { useFolderGrid } from './hooks/useFolderGrid';
import { useActiveConflicts } from '../../hooks/useFolders';
import { useAppStore } from '../../stores/useAppStore';
import { cn } from '../../lib/utils';

export default function FolderGrid() {
  const {
    // Data & State
    sortedFolders,
    isLoading,
    isError,
    error,
    isPlaceholderData,
    selfNodeType,
    selfIsMod,
    selfIsEnabled,
    selfReasons,
    conflicts: nameConflicts,
    isGridView,
    isMobile,
    currentPath,
    explorerSearchQuery,
    sortOrder,
    sortLabel,
    viewMode,

    // Virtualization
    parentRef,
    rowVirtualizer,
    columnCount,
    cardWidth,

    // Navigation
    handleNavigate,
    handleBreadcrumbClick,
    handleGoHome,
    setMobilePane,
    setViewMode,
    setExplorerSearch,
    handleSortToggle,
    handleKeyDown,
    focusedId,
    handleRefresh,

    // Selection
    gridSelection,
    toggleGridSelection,
    clearGridSelection,

    // Actions
    handleToggleSelf,
    handleToggleEnabled,
    handleToggleFavorite,
    handleEnableOnlyThis,
    handleMoveToObject,
    moveDialog,
    openMoveDialog,
    closeMoveDialog,
    objects,

    // Duplicate Warning
    duplicateWarning,
    handleDuplicateForceEnable,
    handleDuplicateEnableOnly,
    handleDuplicateCancel,

    // Rename
    renamingId,
    handleRenameRequest,
    handleRenameSubmit,
    handleRenameCancel,

    // Delete
    deleteConfirm,
    setDeleteConfirm,
    handleDeleteRequest,
    handleDeleteConfirm,

    // Epic 5
    isPreviewOpen,
    togglePreview,

    // Bulk
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
    handleBulkMoveToObject,

    pinSafeDialog,
    handleToggleSafeRequest,
    handleToggleSafeSubmit,
    handleToggleSafeCancel,

    activeContextDialog,
    handleActiveContextCancel,
    handleActiveContextSubmit,

    isDragging,
    selectedObject,
    handleImportFiles,
  } = useFolderGrid();

  // Duplicate / Conflict Detection Visual
  const { data: conflicts = [] } = useActiveConflicts();
  const conflictPathSet = useMemo(() => {
    const s = new Set<string>();
    for (const c of conflicts) {
      // Only flag as conflict if 2+ mods share the same hash
      if (c.mod_paths.length > 1) {
        for (const p of c.mod_paths) s.add(p.replace(/\\/g, '/'));
      }
    }
    return s;
  }, [conflicts]);

  // Safe-mode filtering is already applied in useFolderGrid (via filteredFolders)
  const visibleFolders = sortedFolders;

  const isFlatModRoot = selfNodeType === 'FlatModRoot' || selfIsMod;

  const activePane = useAppStore((state) => state.activePane);
  const setActivePane = useAppStore((state) => state.setActivePane);

  return (
    <div
      className={cn(
        'flex flex-col h-full bg-transparent p-4 relative outline-none transition-shadow duration-200',
        activePane === 'folderGrid' && 'ring-1 ring-inset ring-primary/20',
      )}
      onKeyDown={(e) => {
        if (activePane === 'folderGrid') handleKeyDown(e);
      }}
      tabIndex={-1}
      onFocus={(e) => {
        if (!e.defaultPrevented) setActivePane('folderGrid');
      }}
    >
      <FolderGridToolbar
        isMobile={isMobile}
        currentPath={currentPath}
        handleBreadcrumbClick={handleBreadcrumbClick}
        handleGoHome={handleGoHome}
        selectedObject={selectedObject}
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

      <FolderGridBanners
        isLoading={isLoading}
        isError={isError}
        nameConflicts={nameConflicts}
        isFlatModRoot={isFlatModRoot}
        selfIsEnabled={selfIsEnabled}
        selfReasons={selfReasons}
        isMobile={isMobile}
        isPreviewOpen={isPreviewOpen}
        setMobilePane={setMobilePane}
        togglePreview={togglePreview}
        handleToggleSelf={handleToggleSelf}
      />

      {/* Loading */}
      {isLoading && (
        <div className="flex-1 flex items-center justify-center">
          <Loader2 size={24} className="animate-spin text-primary/50" />
        </div>
      )}

      {/* Error */}
      {isError && (
        <div className="flex-1 flex flex-col items-center justify-center gap-2 p-4">
          <p className="text-xs text-error/60">
            {error instanceof Error ? error.message : String(error) || 'Failed to load'}
          </p>
        </div>
      )}

      {/* Empty state (only if NOT flat mod root) */}
      {!isLoading && !isError && visibleFolders.length === 0 && !isFlatModRoot && (
        <FolderGridEmpty
          explorerSearchQuery={explorerSearchQuery}
          currentPath={currentPath}
          setExplorerSearch={setExplorerSearch}
          handleBreadcrumbClick={handleBreadcrumbClick}
          handleImportFiles={handleImportFiles}
        />
      )}

      {/* Empty state for Flat Mod Root (prevents completely blank layout) */}
      {!isLoading && !isError && visibleFolders.length === 0 && isFlatModRoot && (
        <div className="flex-1 flex flex-col items-center justify-center gap-2 p-6 text-base-content/40">
          <p className="text-sm font-medium">This mod contains no subfolder variants.</p>
          <p className="text-xs text-center">
            Use the Preview Panel on the right to edit its metadata or INI files.
          </p>
        </div>
      )}

      {/* Virtualized Grid/List Content */}
      <div
        ref={parentRef}
        className={cn(
          'flex-1 overflow-y-auto scrollbar-thin scrollbar-track-transparent scrollbar-thumb-base-content/20 hover:scrollbar-thumb-base-content/40 transition-opacity duration-150',
          isPlaceholderData ? 'opacity-70 pointer-events-none select-none' : 'opacity-100',
          !isLoading && !isError && visibleFolders.length > 0 ? 'block' : 'hidden',
        )}
      >
        <div className="relative w-full" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
          {rowVirtualizer.getVirtualItems().map((virtualRow) => {
            if (isGridView) {
              // Grid mode: each virtual row = N columns
              const fromIndex = virtualRow.index * columnCount;
              const toIndex = Math.min(fromIndex + columnCount, visibleFolders.length);
              const rowItems = visibleFolders.slice(fromIndex, toIndex);

              return (
                <div
                  key={virtualRow.index}
                  className="absolute top-0 left-0 w-full flex gap-3"
                  style={{
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  {rowItems.map((folder) => (
                    <div key={folder.path} className="flex-none" style={{ width: cardWidth }}>
                      <FolderCard
                        folder={folder}
                        isSelected={gridSelection.has(folder.path)}
                        onNavigate={handleNavigate}
                        toggleSelection={toggleGridSelection}
                        clearSelection={clearGridSelection}
                        onToggleEnabled={handleToggleEnabled}
                        onToggleFavorite={handleToggleFavorite}
                        onEnableOnlyThis={handleEnableOnlyThis}
                        isRenaming={renamingId === folder.path}
                        onRenameSubmit={handleRenameSubmit}
                        onRenameCancel={handleRenameCancel}
                        onRename={() => handleRenameRequest(folder)}
                        onDelete={() => handleDeleteRequest(folder)}
                        isFocused={focusedId === folder.path}
                        selectionSize={gridSelection.size}
                        onBulkToggle={handleBulkToggle}
                        onBulkDelete={handleBulkDeleteRequest}
                        onBulkTag={handleBulkTagRequest}
                        onBulkFavorite={handleBulkFavorite}
                        onBulkSafe={handleBulkSafe}
                        onBulkPin={handleBulkPin}
                        onBulkMoveToObject={handleBulkMoveToObject}
                        onOpenMoveDialog={openMoveDialog}
                        onToggleSafe={() => handleToggleSafeRequest(folder)}
                        hasConflict={conflictPathSet.has(folder.path.replace(/\\/g, '/'))}
                      />
                    </div>
                  ))}
                </div>
              );
            }

            // List mode: each virtual row = 1 item
            const folder = visibleFolders[virtualRow.index];
            if (!folder) return null;

            return (
              <div
                key={folder.path}
                className="absolute top-0 left-0 w-full"
                style={{
                  height: `${virtualRow.size}px`,
                  transform: `translateY(${virtualRow.start}px)`,
                }}
              >
                <FolderListRow
                  item={folder}
                  isSelected={gridSelection.has(folder.path)}
                  toggleSelection={(id: string, multi: boolean) => toggleGridSelection(id, multi)}
                  clearSelection={clearGridSelection}
                  onToggleEnabled={handleToggleEnabled}
                  selectionSize={gridSelection.size}
                  onBulkToggle={handleBulkToggle}
                  onBulkDelete={handleBulkDeleteRequest}
                  onBulkTag={handleBulkTagRequest}
                  onBulkFavorite={handleBulkFavorite}
                  onBulkSafe={handleBulkSafe}
                  onBulkPin={handleBulkPin}
                  onBulkMoveToObject={handleBulkMoveToObject}
                  onRename={() => handleRenameRequest(folder)}
                  onDelete={() => handleDeleteRequest(folder)}
                  onToggleFavorite={handleToggleFavorite}
                  onOpenMoveDialog={openMoveDialog}
                  onToggleSafe={() => handleToggleSafeRequest(folder)}
                  hasConflict={conflictPathSet.has(folder.path.replace(/\\/g, '/'))}
                />
              </div>
            );
          })}
        </div>
      </div>

      <FolderGridModals
        moveDialog={moveDialog}
        closeMoveDialog={closeMoveDialog}
        objects={objects}
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
        duplicateWarning={duplicateWarning}
        handleDuplicateForceEnable={handleDuplicateForceEnable}
        handleDuplicateEnableOnly={handleDuplicateEnableOnly}
        handleDuplicateCancel={handleDuplicateCancel}
        pinSafeDialog={pinSafeDialog}
        handleToggleSafeCancel={handleToggleSafeCancel}
        handleToggleSafeSubmit={handleToggleSafeSubmit}
        activeContextDialog={activeContextDialog}
        handleActiveContextCancel={handleActiveContextCancel}
        handleActiveContextSubmit={handleActiveContextSubmit}
      />

      <BulkProgressBar />

      {/* Drag Overlay */}
      {isDragging && <DragOverlay isDragging={isDragging} />}
    </div>
  );
}
