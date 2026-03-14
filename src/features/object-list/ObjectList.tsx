/**
 * Epic 3: ObjectList — main sidebar component for browsing game objects/folders.
 * Thin orchestrator composing: Toolbar, States, Content, Modals, StatusBar.
 * Includes zone-aware DnD with 3 drop targets:
 *   - Top (toolbar area): Auto Organize
 *   - Middle (item rows): Move to specific object
 *   - Bottom (status bar): Append as new object folder
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { FolderPlus, FolderInput, AlertTriangle } from 'lucide-react';
import { useObjectListLogic } from './useObjectListLogic';
import { useFileDrop } from '../../hooks/useFileDrop';
import { useDragAutoScroll } from '../../hooks/useDragAutoScroll';
import ObjectListToolbar from './ObjectListToolbar';
import ObjectListStates from './ObjectListStates';
import ObjectListContent from './ObjectListContent';
import ObjectListModals, { SYNC_CONFIRM_RESET } from './ObjectListModals';
import { useObjectListDropZones } from './useObjectListDropZones';
import DropConfirmModal from './DropConfirmModal';
import ArchiveModal from '../scanner/components/ArchiveModal';
import BulkTagModal from './BulkTagModal';
import { useAppStore } from '../../stores/useAppStore';
import { cn } from '../../lib/utils';

export default function ObjectList() {
  const {
    parentRef,
    isMobile,
    activeGame,
    selectedObjectFolderPath,
    setSelectedObjectFolderPath,
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
    deleteObjectDialog,
    setDeleteObjectDialog,
    confirmDeleteObject,
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
    handleRevealInExplorer,
    handleEnableObject,
    handleDisableObject,
    categoryNames,
    handleDropOnItem,
    handleDropAutoOrganize,
    handleDropOnNewObjectSubmit,
    scanReview,
    handleCommitScan,
    handleCloseScanReview,
    archiveModal, // Restored to original name
    handleArchivesInteractively,
    handleArchiveExtractSubmit,
    handleArchiveExtractSkip,
    handleStopExtraction,
    // Bulk
    bulkSelect,
    bulkTagModal,
    setBulkTagModal,
    handleBulkDelete,
    handleBulkPin,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkAddTags,
    handleBulkRemoveTags,
    handleBulkAutoOrganize,
  } = useObjectListLogic();

  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [autoSetupOpen, setAutoSetupOpen] = useState(false);
  /** Pending paths for the "create new object with pre-selected files" flow */
  const [pendingPaths, setPendingPaths] = useState<string[] | null>(null);
  const [mismatchConfirm, setMismatchConfirm] = useState<string[] | null>(null);

  // --- Refs for zone hit-testing ---
  const toolbarRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  const {
    activeDropZone,
    hoveredItemId,
    tooltipTop,
    dropValidation,
    setDropValidation,
    onDrop,
    handleDragOver,
    handleDragStateChange,
  } = useObjectListDropZones({
    activeGame,
    objects,
    toolbarRef,
    contentRef,
    bottomRef,
    handleDropOnItem,
    handleDropAutoOrganize,
    setPendingPaths,
    setCreateModalOpen,
  });

  const { isDragging, dragPosition } = useFileDrop({
    onDrop,
    onDragOver: handleDragOver,
    onDragStateChange: handleDragStateChange,
    enabled: !!activeGame,
  });

  useDragAutoScroll({
    containerRef: contentRef,
    dragPosition,
    speed: 8,
    threshold: 50,
  });

  /** Modal callbacks */
  const handleConfirmMoveAnyway = useCallback(() => {
    if (!dropValidation) return;
    const { targetId, paths } = dropValidation;
    setDropValidation(null);
    handleDropOnItem(targetId, paths);
  }, [dropValidation, handleDropOnItem, setDropValidation]);

  const handleConfirmMoveToSuggested = useCallback(() => {
    if (!dropValidation?.suggestedId) return;
    const { suggestedId, paths } = dropValidation;
    setDropValidation(null);
    handleDropOnItem(suggestedId, paths);
  }, [dropValidation, handleDropOnItem, setDropValidation]);

  const handleCancelDrop = useCallback(() => {
    setDropValidation(null);
  }, [setDropValidation]);

  const handleSkipValidation = useCallback(() => {
    if (!dropValidation) return;
    const { targetId, paths } = dropValidation;
    setDropValidation(null);
    handleDropOnItem(targetId, paths);
  }, [dropValidation, handleDropOnItem, setDropValidation]);

  const qc = useQueryClient();
  const handleRefresh = useCallback(async () => {
    if (activeGame) {
      try {
        await invoke('repair_orphan_mods', { gameId: activeGame.id });
      } catch (e) {
        console.error('Repair orphan mods failed:', e);
      }
    }
    qc.invalidateQueries({ queryKey: ['objects'] });
    qc.invalidateQueries({ queryKey: ['mod-folders'] });
    qc.invalidateQueries({ queryKey: ['category-counts'] });
  }, [activeGame, qc]);

  // Listen to remote auto-organize requests (e.g., from PreviewPanel empty state)
  useEffect(() => {
    const onAutoOrganizeRequest = () => handleSync();
    window.addEventListener('request-auto-organize', onAutoOrganizeRequest);

    const onAutoOrganizePathsRequest = (e: Event) => {
      const paths = (e as CustomEvent<string[]>).detail;
      if (paths && Array.isArray(paths)) {
        handleDropAutoOrganize(paths);
      }
    };
    window.addEventListener('request-auto-organize-paths', onAutoOrganizePathsRequest);

    // Listen to archive import requests from FolderGrid
    const onArchiveImportRequest = (e: Event) => {
      const { archives, nonArchivePaths, targetDir } = (e as CustomEvent).detail;
      handleArchivesInteractively(archives, {
        type: 'item',
        pathsToIngest: nonArchivePaths || [],
        targetFolder: targetDir,
        targetObjectId: '',
      });
    };
    window.addEventListener('request-archive-import', onArchiveImportRequest);

    return () => {
      window.removeEventListener('request-auto-organize', onAutoOrganizeRequest);
      window.removeEventListener('request-auto-organize-paths', onAutoOrganizePathsRequest);
      window.removeEventListener('request-archive-import', onArchiveImportRequest);
    };
  }, [handleSync, handleDropAutoOrganize, handleArchivesInteractively]);

  const isEmpty = !isLoading && !isError && objects.length === 0;
  const hasNoGame = !activeGame;
  const showContent = !isLoading && !isError && !isEmpty && !hasNoGame;
  const showFilterPanel = !!activeGame;
  const conflictObjects = objects.filter((o) => o.has_naming_conflict);

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
    handleRevealInExplorer,
    handleEnableObject,
    handleDisableObject,
  };

  const activePane = useAppStore((state) => state.activePane);
  const setActivePane = useAppStore((state) => state.setActivePane);

  return (
    <div
      className={cn(
        'flex flex-col h-full bg-base-100/50 relative outline-none transition-shadow duration-200',
        activePane === 'objectList' && 'ring-1 ring-inset ring-primary/20',
      )}
      tabIndex={-1}
      onFocus={(e) => {
        // Prevent setting focus if clicking inside a modal or portal that bubbles up
        if (!e.defaultPrevented) setActivePane('objectList');
      }}
      onKeyDown={(e) => {
        if (activePane !== 'objectList') return;

        if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
          e.preventDefault();
          bulkSelect.selectAll();
          return;
        }

        if (e.key === 'Escape') {
          if (bulkSelect.isAnySelected) {
            e.preventDefault();
            bulkSelect.clearSelection();
            return;
          }
        }

        if (e.key === 'Delete') {
          if (bulkSelect.isAnySelected) {
            e.preventDefault();
            handleBulkDelete(bulkSelect.selectedIds).then(bulkSelect.clearSelection);
            return;
          } else if (selectedObjectFolderPath) {
            e.preventDefault();
            const objToDel = objects.find((o) => o.folder_path === selectedObjectFolderPath);
            if (objToDel) handleDeleteObject(objToDel.id);
            return;
          }
        }
      }}
    >
      {/* Toolbar: search, category selector, sort, actions — also Auto Organize drop zone */}
      <div ref={toolbarRef}>
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
          onRefresh={handleRefresh}
          onCreateNew={() => setCreateModalOpen(true)}
          showFilterPanel={showFilterPanel}
          categoryFilters={categoryFilters}
          activeFilters={activeFilters}
          onFilterChange={handleFilterChange}
          onClearFilters={handleClearFilters}
          statusFilter={statusFilter}
          onStatusFilterChange={setStatusFilter}
          isDragging={isDragging}
          isActiveZone={activeDropZone === 'auto-organize'}
          bulkSelect={{
            isAnySelected: activePane === 'objectList' && bulkSelect.isAnySelected,
            selectionCount: bulkSelect.selectionCount,
            onDelete: () =>
              handleBulkDelete(bulkSelect.selectedIds).then(bulkSelect.clearSelection),
            onPin: (pin) =>
              handleBulkPin(bulkSelect.selectedIds, pin).then(bulkSelect.clearSelection),
            onEnable: () =>
              handleBulkEnable(bulkSelect.selectedIds).then(bulkSelect.clearSelection),
            onDisable: () =>
              handleBulkDisable(bulkSelect.selectedIds).then(bulkSelect.clearSelection),
            onAddTags: () => setBulkTagModal({ open: true, mode: 'add' }),
            onRemoveTags: () => setBulkTagModal({ open: true, mode: 'remove' }),
            onAutoOrganize: () =>
              handleBulkAutoOrganize(bulkSelect.selectedIds).then(bulkSelect.clearSelection),
            onClear: bulkSelect.clearSelection,
          }}
        />
      </div>

      {/* Naming Conflict Warning */}
      {conflictObjects.length > 0 && (
        <div className="mx-2 mt-1 mb-0.5 flex items-center gap-1.5 bg-warning/10 border border-warning/20 rounded-md px-2 py-1">
          <AlertTriangle size={12} className="text-warning shrink-0" />
          <span className="text-[10px] text-warning flex-1 truncate">
            {conflictObjects.length} naming conflict{conflictObjects.length > 1 ? 's' : ''}
          </span>
          <button
            className="text-[10px] text-warning font-semibold hover:underline shrink-0"
            onClick={() => {
              const obj = conflictObjects[0];
              if (activeGame?.mod_path && obj.folder_path) {
                const baseName = obj.name;
                const modPath = activeGame.mod_path.replace(/\\/g, '/');
                const enabledPath = `${modPath}/${obj.folder_path}`;
                const disabledPath = `${modPath}/DISABLED ${baseName}`;
                useAppStore.getState().openConflictDialog({
                  type: 'RenameConflict',
                  attempted_target: enabledPath,
                  existing_path: disabledPath,
                  base_name: baseName,
                });
              }
            }}
          >
            Resolve
          </button>
        </div>
      )}

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
        onClearFilters={handleClearFilters}
        onClearSearch={() => setSidebarSearch('')}
        onCreateNew={() => setCreateModalOpen(true)}
        onAutoSetup={() => setAutoSetupOpen(true)}
      />

      {/* Virtualized list (objects or folders) — item drop zone */}
      <div ref={contentRef} className="flex-1 min-h-0 flex flex-col">
        {showContent && (
          <ObjectListContent
            parentRef={parentRef}
            rowVirtualizer={rowVirtualizer}
            flatObjectItems={flatObjectItems}
            selectedObjectFolderPath={selectedObjectFolderPath}
            setSelectedObjectFolderPath={setSelectedObjectFolderPath}
            selectedObjectType={selectedObjectType}
            setSelectedObjectType={setSelectedObjectType}
            isMobile={isMobile}
            stickyPosition={stickyPosition as 'top' | 'bottom' | null}
            selectedIndex={selectedIndex}
            scrollToSelected={scrollToSelected}
            contextMenuProps={contextMenuProps}
            isDragging={isDragging}
            hoveredItemId={hoveredItemId}
            isAnyBulkSelected={bulkSelect.isAnySelected}
            isBulkSelected={bulkSelect.isSelected}
            onToggleBulkSelect={bulkSelect.toggleSelection}
          />
        )}
      </div>

      {/* Floating tooltip for per-item drop target */}
      {isDragging &&
        activeDropZone === 'item' &&
        hoveredItemId &&
        (() => {
          const obj = objects.find((o) => o.id === hoveredItemId);
          return obj ? (
            <div
              className="absolute right-4 z-40 flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary text-primary-content shadow-xl pointer-events-none"
              style={{ top: tooltipTop }}
            >
              <FolderInput size={14} />
              <span className="text-xs font-semibold whitespace-nowrap">Move to {obj.name}</span>
            </div>
          ) : null;
        })()}

      {/* Bottom zone: status bar or "Append as New Object" drop zone */}
      <div
        ref={bottomRef}
        className={`px-3 border-t transition-all duration-200 relative z-30 ${
          isDragging
            ? activeDropZone === 'new-object'
              ? 'py-5 border-primary bg-primary/15 border-dashed border-t-2'
              : 'py-5 border-base-300/50 bg-base-200/70 border-dashed border-t-2'
            : 'py-1.5 border-base-300/20'
        }`}
        style={isDragging ? { animation: 'slideUp 200ms ease-out' } : undefined}
      >
        {isDragging ? (
          <div
            className={`flex items-center justify-center gap-2 ${
              activeDropZone === 'new-object' ? 'text-primary' : 'text-base-content/50'
            }`}
          >
            <FolderPlus
              size={18}
              className={activeDropZone === 'new-object' ? 'animate-pulse' : ''}
            />
            <span className="text-xs font-medium">Append as New Object Folder</span>
          </div>
        ) : (
          <div className="flex items-center justify-between">
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
        )}
      </div>

      {/* Modals: delete, edit, sync, create */}
      <ObjectListModals
        activeGame={activeGame}
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
        pendingPaths={pendingPaths}
        onImportDropped={(newObjId, newObjName, paths) => {
          handleDropOnNewObjectSubmit(newObjId, newObjName, paths);
          setCreateModalOpen(false);
          setPendingPaths(null);
        }}
        onCloseCreate={() => {
          setCreateModalOpen(false);
          setPendingPaths(null);
        }}
        autoSetupOpen={autoSetupOpen}
        onCloseAutoSetup={() => setAutoSetupOpen(false)}
        deleteObjectDialog={deleteObjectDialog}
        onConfirmDeleteObject={confirmDeleteObject}
        onCancelDeleteObject={() => setDeleteObjectDialog({ open: false, id: '', name: '' })}
        mismatchConfirm={mismatchConfirm}
        onConfirmMismatchHandler={() => {
          if (mismatchConfirm) {
            handleDropAutoOrganize(mismatchConfirm);
          }
          setMismatchConfirm(null);
        }}
        onCancelMismatchHandler={() => setMismatchConfirm(null)}
      />

      {/* Pre-drop validation modal */}
      <DropConfirmModal
        validation={dropValidation}
        onMoveAnyway={handleConfirmMoveAnyway}
        onMoveToSuggested={handleConfirmMoveToSuggested}
        onCancel={handleCancelDrop}
        onSkipValidation={handleSkipValidation}
      />

      {/* Archive extraction modal triggered during DnD */}
      <ArchiveModal
        key={archiveModal.archives.length > 0 ? archiveModal.archives[0].path : 'empty'}
        isOpen={archiveModal.open}
        archives={archiveModal.archives}
        isExtracting={archiveModal.isExtracting}
        error={archiveModal.error}
        passwordError={archiveModal.passwordError}
        extractProgress={archiveModal.extractProgress}
        fileProgress={archiveModal.fileProgress}
        onExtract={handleArchiveExtractSubmit}
        onSkip={handleArchiveExtractSkip}
        onStop={handleStopExtraction}
        targetObjectName={
          archiveModal.pendingDropContext?.targetObjectId
            ? objects.find((o) => o.id === archiveModal.pendingDropContext?.targetObjectId)?.name
            : undefined
        }
      />

      {/* Bulk tag modal */}
      <BulkTagModal
        open={bulkTagModal.open}
        mode={bulkTagModal.mode}
        existingTags={[...bulkSelect.selectedIds]
          .map((id) => objects.find((o) => o.id === id))
          .filter(Boolean)
          .flatMap((obj) => {
            try {
              return JSON.parse(obj!.tags || '[]') as string[];
            } catch {
              return [];
            }
          })}
        onSubmit={(tags) => {
          if (bulkTagModal.mode === 'add') {
            handleBulkAddTags(bulkSelect.selectedIds, tags).then(bulkSelect.clearSelection);
          } else {
            handleBulkRemoveTags(bulkSelect.selectedIds, tags).then(bulkSelect.clearSelection);
          }
        }}
        onClose={() => setBulkTagModal({ open: false, mode: 'add' })}
      />
    </div>
  );
}
