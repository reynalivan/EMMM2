import {
  Search,
  ChevronLeft,
  ArrowUpDown,
  LayoutGrid,
  List,
  Loader2,
  FolderOpen,
  RefreshCw,
} from 'lucide-react';
import FolderCard from './FolderCard';
import FolderListRow from './FolderListRow';
import MoveToObjectDialog from './MoveToObjectDialog';
import ExplorerBreadcrumbs from './Breadcrumbs';
import ConfirmDialog from '../../components/ui/ConfirmDialog';
import DuplicateWarningModal from './DuplicateWarningModal';
import { BulkTagModal } from './BulkTagModal';
import DragOverlay from './DragOverlay';
import BulkProgressBar from './BulkProgressBar';
import { useFolderGrid } from './hooks/useFolderGrid';
import { useAppStore } from '../../stores/useAppStore';

export default function FolderGrid() {
  const safeMode = useAppStore((state) => state.safeMode);
  const {
    // Data & State
    sortedFolders,
    isLoading,
    isError,
    error,
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

    // Bulk
    bulkTagOpen,
    setBulkTagOpen,
    bulkDeleteConfirm,
    setBulkDeleteConfirm,
    handleBulkToggle,
    handleBulkTagRequest,
    handleBulkDeleteRequest,
    handleBulkDeleteConfirm,

    // DnD
    isDragging,
    selectedObject,
  } = useFolderGrid();

  const visibleFolders = safeMode
    ? sortedFolders.filter((folder) => folder.is_safe)
    : sortedFolders;

  return (
    <div
      className="flex flex-col h-full bg-transparent p-4 relative outline-none"
      onKeyDown={handleKeyDown}
      tabIndex={0}
    >
      {/* Top Bar: Breadcrumbs & View Controls */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <button
            onClick={() => setMobilePane('sidebar')}
            className="btn btn-ghost btn-sm btn-square md:hidden text-base-content/50 hover:text-base-content"
          >
            <ChevronLeft size={20} />
          </button>

          <ExplorerBreadcrumbs
            path={currentPath}
            onNavigate={handleBreadcrumbClick}
            onGoHome={handleGoHome}
            isRootHidden={!!selectedObject}
          />
        </div>

        {/* View/Sort toggle buttons */}
        <div className="flex items-center gap-1">
          <button
            onClick={handleSortToggle}
            className="btn btn-ghost btn-xs gap-1 text-base-content/50 hover:text-base-content"
            title={`Sort: ${sortLabel} ${sortOrder === 'asc' ? '↑' : '↓'}`}
          >
            <ArrowUpDown size={14} />
            <span className="text-[10px] font-semibold hidden sm:inline">
              {sortLabel} {sortOrder === 'asc' ? '↑' : '↓'}
            </span>
          </button>

          {!isMobile && (
            <>
              <button
                onClick={() => setViewMode('grid')}
                className={`btn btn-ghost btn-xs btn-square ${viewMode === 'grid' ? 'text-primary' : 'text-base-content/40'}`}
              >
                <LayoutGrid size={14} />
              </button>
              <button
                onClick={() => setViewMode('list')}
                className={`btn btn-ghost btn-xs btn-square ${viewMode === 'list' ? 'text-primary' : 'text-base-content/40'}`}
              >
                <List size={14} />
              </button>
            </>
          )}
        </div>
      </div>

      {/* Search toolbar */}
      <div className="flex items-center gap-3 mb-3 bg-base-300/50 p-2 rounded-lg border border-base-content/5">
        <div className="relative flex-1 group">
          <Search
            className="absolute left-3 top-1/2 -translate-y-1/2 text-base-content/30 group-focus-within:text-primary transition-colors"
            size={16}
          />
          <input
            type="text"
            placeholder="Search mods..."
            className="input input-sm w-full pl-10 bg-transparent border-transparent focus:border-transparent text-base-content placeholder:text-base-content/20 transition-all focus:bg-base-content/5 rounded-md"
            value={explorerSearchQuery}
            onChange={(e) => setExplorerSearch(e.target.value)}
          />
        </div>
        <span className="text-[10px] text-base-content/30 font-medium tabular-nums shrink-0">
          {visibleFolders.length} item{visibleFolders.length !== 1 ? 's' : ''}
        </span>
        <button
          onClick={handleRefresh}
          className="btn btn-ghost btn-xs btn-square text-base-content/30 hover:text-primary transition-colors"
          title="Refresh folder list"
        >
          <RefreshCw size={14} />
        </button>
      </div>

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

      {/* Empty state */}
      {!isLoading && !isError && visibleFolders.length === 0 && (
        <div className="flex-1 flex flex-col items-center justify-center gap-3 p-6">
          <FolderOpen size={40} className="text-base-content/15" />
          <p className="text-sm text-base-content/40 text-center">
            {explorerSearchQuery
              ? 'No mods match your search'
              : 'No mod folders found. Add mods to your game directory to get started.'}
          </p>
        </div>
      )}

      {/* Virtualized Grid/List Content */}
      {!isLoading && !isError && visibleFolders.length > 0 && (
        <div
          ref={parentRef}
          className="flex-1 overflow-y-auto scrollbar-thin scrollbar-track-transparent scrollbar-thumb-base-content/20 hover:scrollbar-thumb-base-content/40"
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
                          onOpenMoveDialog={openMoveDialog}
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
                    onRename={() => handleRenameRequest(folder)}
                    onDelete={() => handleDeleteRequest(folder)}
                    onToggleFavorite={handleToggleFavorite}
                    onOpenMoveDialog={openMoveDialog}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Move To Object Dialog */}
      {moveDialog.open && moveDialog.folder && (
        <MoveToObjectDialog
          open={moveDialog.open}
          onClose={closeMoveDialog}
          objects={objects}
          currentObjectId={moveDialog.folder.object_id ?? undefined}
          currentStatus={moveDialog.folder.is_enabled}
          onSubmit={(targetId, status) => {
            if (!moveDialog.folder) return;
            handleMoveToObject(moveDialog.folder, targetId, status);
            closeMoveDialog();
          }}
        />
      )}

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        open={deleteConfirm.open}
        title="Delete to Trash?"
        message={`Are you sure you want to move "${deleteConfirm.folder?.name}" to trash? You can undo this later.`}
        confirmLabel="Move to Trash"
        danger
        onConfirm={handleDeleteConfirm}
        onCancel={() => setDeleteConfirm({ open: false, folder: null })}
      />

      <ConfirmDialog
        open={bulkDeleteConfirm}
        title={`Delete ${gridSelection.size} Mods?`}
        message={`Are you sure you want to move ${gridSelection.size} mods to trash?`}
        confirmLabel="Move All to Trash"
        danger
        onConfirm={handleBulkDeleteConfirm}
        onCancel={() => setBulkDeleteConfirm(false)}
      />

      {bulkTagOpen && (
        <BulkTagModal
          isOpen={true}
          onClose={() => setBulkTagOpen(false)}
          selectedPaths={Array.from(gridSelection)}
        />
      )}

      {/* Duplicate Character Warning */}
      <DuplicateWarningModal
        open={duplicateWarning.open}
        targetName={duplicateWarning.folder?.name ?? ''}
        duplicates={duplicateWarning.duplicates}
        onForceEnable={handleDuplicateForceEnable}
        onEnableOnlyThis={handleDuplicateEnableOnly}
        onCancel={handleDuplicateCancel}
      />

      <BulkProgressBar />

      {/* Drag Overlay */}
      {isDragging && <DragOverlay isDragging={isDragging} />}
    </div>
  );
}
