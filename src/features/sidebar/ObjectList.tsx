/**
 * Epic 3: ObjectList — main sidebar component for browsing game objects/folders.
 * Thin orchestrator composing: Toolbar, States, Content, Modals, StatusBar.
 */

import { useState, useCallback } from 'react';
import { useObjectListLogic } from './useObjectListLogic';
import { useFileDrop } from '../../hooks/useFileDrop';
import ObjectListToolbar from './ObjectListToolbar';
import ObjectListStates from './ObjectListStates';
import ObjectListContent from './ObjectListContent';
import ObjectListModals, { SYNC_CONFIRM_RESET } from './ObjectListModals';

export default function ObjectList() {
  const {
    parentRef,
    isMobile,
    activeGame,
    selectedObject,
    setSelectedObject,
    selectedObjectType,
    setSelectedObjectType,
    sidebarSearchQuery,
    setSidebarSearch,
    deleteDialog,
    setDeleteDialog,
    activeFilters,
    objects,
    schema,
    categoryFilters,
    isLoading,
    isError,
    objectsErrorInfo,
    rowVirtualizer,
    flatObjectItems,
    stickyPosition,
    selectedIndex,
    scrollToSelected,
    handleToggle,
    handleOpen,
    handleDelete,
    confirmDelete,
    handleDeleteObject,
    handleFilterChange,
    handleClearFilters,
    sortBy,
    setSortBy,
    statusFilter,
    setStatusFilter,
    editObject,
    setEditObject,
    handleEdit,
    handleSync,
    isSyncing,
    handleSyncWithDb,
    handleApplySyncMatch,
    syncConfirm,
    setSyncConfirm,
    handlePin,
    handleFavorite,
    handleMoveCategory,
    handleEnableObject,
    handleDisableObject,
    categoryNames,
    scanReview,
    handleCommitScan,
    handleCloseScanReview,
  } = useObjectListLogic();

  const [createModalOpen, setCreateModalOpen] = useState(false);

  // US-3.Z: Sidebar drag & drop visual feedback
  const noopDrop = useCallback((_paths: string[]) => {}, []);
  const { isDragging } = useFileDrop({ onDrop: noopDrop, enabled: !!activeGame });

  const isEmpty = !isLoading && !isError && objects.length === 0;
  const hasNoGame = !activeGame;
  const showContent = !isLoading && !isError && !isEmpty && !hasNoGame;
  const showFilterPanel = !!activeGame;

  /** Props forwarded to ObjectListContent for context-menu rendering */
  const contextMenuProps = {
    isSyncing,
    categoryNames,
    handleEdit,
    handleSyncWithDb,
    handleDelete,
    handleDeleteObject,
    handleToggle,
    handleOpen,
    handlePin,
    handleFavorite,
    handleMoveCategory,
    handleEnableObject,
    handleDisableObject,
  };

  return (
    <div
      className={`flex flex-col h-full bg-base-100/50 transition-all duration-150 ${
        isDragging ? 'ring-2 ring-inset ring-primary/40 bg-primary/5' : ''
      }`}
    >
      {/* Toolbar: search, category selector, sort, actions */}
      <ObjectListToolbar
        sidebarSearchQuery={sidebarSearchQuery}
        onSearchChange={setSidebarSearch}
        schema={schema}
        selectedObjectType={selectedObjectType}
        onSelectObjectType={setSelectedObjectType}
        sortBy={sortBy}
        onSortChange={setSortBy}
        isSyncing={isSyncing}
        onSync={handleSync}
        onCreateNew={() => setCreateModalOpen(true)}
        showFilterPanel={showFilterPanel}
        categoryFilters={categoryFilters}
        activeFilters={activeFilters}
        onFilterChange={handleFilterChange}
        onClearFilters={handleClearFilters}
        statusFilter={statusFilter}
        onStatusFilterChange={setStatusFilter}
      />

      {/* Conditional states: loading / error / no-game / empty */}
      <ObjectListStates
        isLoading={isLoading}
        isError={isError}
        errorMessage={
          objectsErrorInfo
            ? objectsErrorInfo instanceof Error
              ? objectsErrorInfo.message
              : String(objectsErrorInfo)
            : undefined
        }
        hasNoGame={hasNoGame}
        isEmpty={isEmpty}
        sidebarSearchQuery={sidebarSearchQuery}
        activeFilters={activeFilters}
        isSyncing={isSyncing}
        onClearFilters={handleClearFilters}
        onSync={handleSync}
      />

      {/* Virtualized list (objects or folders) */}
      {showContent && (
        <ObjectListContent
          parentRef={parentRef}
          rowVirtualizer={rowVirtualizer}
          flatObjectItems={flatObjectItems}
          selectedObject={selectedObject}
          setSelectedObject={setSelectedObject}
          selectedObjectType={selectedObjectType}
          setSelectedObjectType={setSelectedObjectType}
          isMobile={isMobile}
          stickyPosition={stickyPosition as 'top' | 'bottom' | null}
          selectedIndex={selectedIndex}
          scrollToSelected={scrollToSelected}
          contextMenuProps={contextMenuProps}
        />
      )}

      {/* Status bar */}
      <div className="px-3 py-1.5 border-t border-base-300/20 flex items-center justify-between">
        <span className="text-[10px] text-base-content/30">
          {`${objects.length} object${objects.length !== 1 ? 's' : ''}`}
        </span>
        <div className="flex items-center gap-3">
          {selectedObjectType && (
            <button
              className="text-[10px] text-primary/60 hover:text-primary transition-colors"
              onClick={() => setSelectedObjectType(null)}
            >
              Show All
            </button>
          )}
        </div>
      </div>

      {/* Modals: delete, edit, sync, create */}
      <ObjectListModals
        deleteDialog={deleteDialog}
        onConfirmDelete={confirmDelete}
        onCancelDelete={() => setDeleteDialog({ open: false, path: '', name: '', itemCount: 0 })}
        editObject={editObject}
        onCloseEdit={() => setEditObject(null)}
        syncConfirm={syncConfirm}
        onApplySyncMatch={handleApplySyncMatch}
        onEditManually={() => {
          const obj = objects.find((o) => o.id === syncConfirm.objectId);
          setSyncConfirm(SYNC_CONFIRM_RESET);
          if (obj) setEditObject(obj);
        }}
        onCloseSyncConfirm={() => setSyncConfirm(SYNC_CONFIRM_RESET)}
        scanReview={scanReview}
        onCommitScan={handleCommitScan}
        onCloseScanReview={handleCloseScanReview}
        createModalOpen={createModalOpen}
        onCloseCreate={() => setCreateModalOpen(false)}
      />
    </div>
  );
}
