import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import FolderGridToolbar from './FolderGridToolbar';
import FolderGridBanners from './FolderGridBanners';
import FolderGridEmpty from './FolderGridEmpty';
import FolderGridModals from './FolderGridModals';
import FolderCard from './FolderCard';
import FolderListRow from './FolderListRow';
import DragOverlay from './DragOverlay';
import EnableParentDialog from './EnableParentDialog';
import { Loader2, RefreshCw, CheckSquare, FolderOpen } from 'lucide-react';
import BulkProgressBar from './BulkProgressBar';
import BulkActionBar from './BulkActionBar';
import { useFolderGrid } from './hooks/useFolderGrid';
import { useActiveConflicts } from '../../hooks/useFolders';
import { useAppStore } from '../../stores/useAppStore';
import { cn } from '../../lib/utils';
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from '../../components/ui/ContextMenu';
import { commands } from '../../lib/bindings';
import { useActiveGame } from '../../hooks/useActiveGame';

export default function FolderGrid() {
  const { t } = useTranslation(['grid']);
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
    virtualItems,
    totalSize,
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

    // Parent-disabled lock state
    ancestorDisabledBy,
    enableParentDialogOpen,
    setEnableParentDialogOpen,
    handleEnableParent,
    handleToggleEnabledGuarded,

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

    // Sync with DB
    syncConfirm,
    handleSyncWithDb,
    handleCloseSyncConfirm,
    handleApplySyncMatch,

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
  const isIgnoreManagementOpen = useAppStore((state) => state.isIgnoreManagementOpen);
  const setIsIgnoreManagementOpen = useAppStore((state) => state.setIgnoreManagementOpen);
  const { activeGame } = useActiveGame();

  // Current absolute path for "Open Folder in Explorer" on background right-click
  const currentAbsPath = useMemo(() => {
    if (!activeGame?.mod_path) return null;
    const parts = [activeGame.mod_path, ...currentPath.filter(Boolean)];
    return parts.join('\\');
  }, [activeGame, currentPath]);

  const handleOpenFolderInExplorer = async () => {
    if (!currentAbsPath || !activeGame?.id) return;
    try {
      await commands.openInExplorer({ gameId: activeGame.id, path: currentAbsPath });
    } catch (err) {
      console.error('Failed to open folder:', err);
    }
  };

  const handleSelectAll = () => {
    useAppStore.getState().setGridSelection(new Set(visibleFolders.map((f) => f.path)));
  };

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

        if (e.key === 'Delete' && gridSelection.size > 0) {
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
        selfIsEnabled={selfIsEnabled}
        selfReasons={selfReasons}
        isMobile={isMobile}
        isPreviewOpen={isPreviewOpen}
        setMobilePane={setMobilePane}
        togglePreview={togglePreview}
        handleToggleSelf={handleToggleSelf}
        ancestorDisabledBy={ancestorDisabledBy}
        currentPath={currentPath}
        onOpenEnableParentDialog={() => setEnableParentDialogOpen(true)}
      />

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
            {error instanceof Error ? error.message : String(error) || t('status.load_error')}
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
          <p className="text-sm font-medium">{t('status.no_subfolders')}</p>
          <p className="text-xs text-center">{t('status.preview_hint')}</p>
        </div>
      )}

      {/* Virtualized Grid/List Content — wrapped in a background context menu */}
      <ContextMenu
        content={
          <>
            <ContextMenuItem icon={RefreshCw} onClick={handleRefresh}>
              {t('context.refresh')}
            </ContextMenuItem>
            <ContextMenuItem
              icon={CheckSquare}
              onClick={handleSelectAll}
              disabled={visibleFolders.length === 0}
            >
              {t('context.select_all')}
            </ContextMenuItem>
            <ContextMenuSeparator />
            <ContextMenuItem
              icon={FolderOpen}
              onClick={handleOpenFolderInExplorer}
              disabled={!currentAbsPath}
            >
              {t('context.open_explorer')}
            </ContextMenuItem>
          </>
        }
      >
        <div
          ref={parentRef}
          className={cn(
            'flex-1 overflow-y-auto scrollbar-thin scrollbar-track-transparent scrollbar-thumb-base-content/20 hover:scrollbar-thumb-base-content/40 transition-opacity duration-150',
            isPlaceholderData ? 'opacity-70 pointer-events-none select-none' : 'opacity-100',
            !isLoading && !isError && visibleFolders.length > 0 ? 'block' : 'hidden',
          )}
        >
          <div className="relative w-full" style={{ height: `${totalSize}px` }}>
            {virtualItems.map((virtualRow) => {
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
                          onToggleEnabled={handleToggleEnabledGuarded}
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
                          onSyncWithDb={handleSyncWithDb}
                          hasConflict={conflictPathSet.has(folder.path.replace(/\\/g, '/'))}
                          isLockedByParent={!!ancestorDisabledBy}
                          onRequestEnableParent={() => setEnableParentDialogOpen(true)}
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
                    toggleSelection={(id: string, multi: boolean, isShift?: boolean) =>
                      toggleGridSelection(id, multi, isShift)
                    }
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
      </ContextMenu>

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
        handleCloseSyncConfirm={handleCloseSyncConfirm}
        handleApplySyncMatch={handleApplySyncMatch}
        objectId={undefined}
        currentPath={typeof currentPath === 'string' ? currentPath : undefined}
        objects={objects}
      />

      {/* Enable Parent Dialog */}
      {ancestorDisabledBy && (
        <EnableParentDialog
          open={enableParentDialogOpen}
          onClose={() => setEnableParentDialogOpen(false)}
          ancestorName={ancestorDisabledBy}
          willActivate={sortedFolders.filter((f) => f.is_enabled)}
          stayDisabled={sortedFolders.filter((f) => !f.is_enabled)}
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
      />

      {/* Drag Overlay */}
      {isDragging && <DragOverlay isDragging={isDragging} />}
    </div>
  );
}
