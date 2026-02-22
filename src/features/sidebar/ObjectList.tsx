/**
 * Epic 3: ObjectList — main sidebar component for browsing game objects/folders.
 * Thin orchestrator composing: Toolbar, States, Content, Modals, StatusBar.
 * Includes zone-aware DnD with 3 drop targets:
 *   - Top (toolbar area): Auto Organize
 *   - Middle (item rows): Move to specific object
 *   - Bottom (status bar): Append as new object folder
 */

import { useState, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { FolderPlus, FolderInput } from 'lucide-react';
import { useObjectListLogic } from './useObjectListLogic';
import { useFileDrop, type DragPosition } from '../../hooks/useFileDrop';
import ObjectListToolbar from './ObjectListToolbar';
import ObjectListStates from './ObjectListStates';
import ObjectListContent from './ObjectListContent';
import ObjectListModals, { SYNC_CONFIRM_RESET } from './ObjectListModals';
import { classifyDroppedPaths, validateDropForZone, type DropZone } from './dropUtils';
import DropConfirmModal, { type DropValidation } from './DropConfirmModal';
import { scanService } from '../../services/scanService';
import { toast } from '../../stores/useToastStore';

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
    handleRevealInExplorer,
    handleEnableObject,
    handleDisableObject,
    categoryNames,
    scanReview,
    handleCommitScan,
    handleCloseScanReview,
    handleDropOnItem,
    handleDropAutoOrganize,
  } = useObjectListLogic();

  const [createModalOpen, setCreateModalOpen] = useState(false);
  /** Pending paths for the "create new object with pre-selected files" flow */
  const [pendingPaths, setPendingPaths] = useState<string[] | null>(null);
  // TODO: Wire pendingPaths to CreateObjectModal as a prop for pre-selected files
  void pendingPaths; // referenced in setPendingPaths; will be used when CreateObjectModal is enhanced

  // --- Refs for zone hit-testing ---
  const toolbarRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  /** Resolve which drop zone the cursor is in */
  const resolveDropZone = useCallback((position: DragPosition): DropZone | null => {
    const toolbarEl = toolbarRef.current;
    const bottomEl = bottomRef.current;

    if (toolbarEl) {
      const rect = toolbarEl.getBoundingClientRect();
      if (
        position.x >= rect.left &&
        position.x <= rect.right &&
        position.y >= rect.top &&
        position.y <= rect.bottom
      ) {
        return 'auto-organize';
      }
    }

    if (bottomEl) {
      const rect = bottomEl.getBoundingClientRect();
      if (
        position.x >= rect.left &&
        position.x <= rect.right &&
        position.y >= rect.top &&
        position.y <= rect.bottom
      ) {
        return 'new-object';
      }
    }

    // Default to item zone if within content area
    if (contentRef.current) {
      const rect = contentRef.current.getBoundingClientRect();
      if (
        position.x >= rect.left &&
        position.x <= rect.right &&
        position.y >= rect.top &&
        position.y <= rect.bottom
      ) {
        return 'item';
      }
    }

    return null;
  }, []);

  /** Determine the active drop zone from dragPosition */
  const [activeDropZone, setActiveDropZone] = useState<DropZone | null>(null);
  const [hoveredItemId, setHoveredItemId] = useState<string | null>(null);
  const [tooltipTop, setTooltipTop] = useState<number>(0);
  const [dropValidation, setDropValidation] = useState<DropValidation | null>(null);

  /** Pre-drop validation: score the drop against candidates, show modal if low confidence */
  const handleDropWithValidation = useCallback(
    async (paths: string[], position: DragPosition) => {
      if (!activeGame || !contentRef.current) return;

      // Resolve target object from position
      const element = document.elementFromPoint(position.x, position.y);
      let current: HTMLElement | null = element as HTMLElement;
      while (current && !current.dataset.objectId) {
        current = current.parentElement;
      }
      if (!current?.dataset.objectId) {
        toast.info('Drop on a specific object to move items there.');
        return;
      }

      const targetId = current.dataset.objectId;
      const targetObj = objects.find((o) => o.id === targetId);
      if (!targetObj) {
        toast.error('Target object not found.');
        return;
      }

      // Only validate folders (not loose files)
      const classified = classifyDroppedPaths(paths);
      const foldersToValidate = classified.folders;

      // If no folders, skip validation — just move directly
      if (foldersToValidate.length === 0) {
        handleDropOnItem(targetId, paths);
        return;
      }

      // Show validating modal
      setDropValidation({
        paths,
        targetId,
        targetName: targetObj.name,
        status: 'validating',
      });

      try {
        // Score the first dropped folder against all object names
        const candidateNames = objects.map((o) => o.name);
        const scores = await scanService.scoreCandidatesBatch(
          foldersToValidate[0],
          candidateNames,
          activeGame.game_type,
        );

        // Check if validation was cancelled (skip button)
        // We use a check on the current state
        const targetScore = scores[targetObj.name] ?? 0;

        // Find best match
        let bestName = targetObj.name;
        let bestScore = targetScore;
        for (const [name, score] of Object.entries(scores)) {
          if (score > bestScore) {
            bestName = name;
            bestScore = score;
          }
        }

        const bestObj = objects.find((o) => o.name === bestName);

        // Confidence threshold: 50% or below → show warning
        if (targetScore <= 50) {
          setDropValidation({
            paths,
            targetId,
            targetName: targetObj.name,
            status: 'warning',
            targetScore,
            suggestedId: bestObj?.id,
            suggestedName: bestName,
            suggestedScore: bestScore,
          });
        } else {
          // High confidence — move directly
          setDropValidation(null);
          handleDropOnItem(targetId, paths);
        }
      } catch (e) {
        console.error('Pre-drop validation failed:', e);
        // On validation failure, move directly (fail-open)
        setDropValidation(null);
        handleDropOnItem(targetId, paths);
      }
    },
    [activeGame, objects, handleDropOnItem],
  );

  // US-3.Z: Zone-aware DnD handler
  const onDrop = useCallback(
    (paths: string[], position: DragPosition) => {
      if (!activeGame || paths.length === 0) return;

      const zone = resolveDropZone(position);
      if (!zone) {
        toast.info('Drop inside a zone to import items.');
        return;
      }

      const classified = classifyDroppedPaths(paths);
      const validation = validateDropForZone(zone, classified);

      if (!validation.valid) {
        toast.error(validation.reason ?? 'Invalid drop');
        return;
      }

      switch (zone) {
        case 'auto-organize':
          handleDropAutoOrganize(paths);
          break;
        case 'item':
          handleDropWithValidation(paths, position);
          break;
        case 'new-object':
          setPendingPaths(paths);
          setCreateModalOpen(true);
          break;
      }
    },
    [activeGame, resolveDropZone, handleDropAutoOrganize, handleDropWithValidation],
  );

  // Zone detection via onDragOver callback (React-compliant: setState from event handler)
  const handleDragOver = useCallback(
    (pos: DragPosition) => {
      const zone = resolveDropZone(pos);
      setActiveDropZone(zone);

      // Track which object row the cursor is over (for per-item highlight)
      if (zone === 'item') {
        const el = document.elementFromPoint(pos.x, pos.y);
        let current: HTMLElement | null = el as HTMLElement;
        while (current && !current.dataset.objectId) {
          current = current.parentElement;
        }
        setHoveredItemId(current?.dataset.objectId ?? null);
        // Calculate tooltip Y relative to sidebar root
        const sidebarRect = contentRef.current?.parentElement?.getBoundingClientRect();
        setTooltipTop(sidebarRect ? pos.y - sidebarRect.top - 16 : pos.y);
      } else {
        setHoveredItemId(null);
      }
    },
    [resolveDropZone],
  );

  const handleDragStateChange = useCallback((dragging: boolean) => {
    if (!dragging) {
      setActiveDropZone(null);
      setHoveredItemId(null);
      setTooltipTop(0);
    }
  }, []);

  const { isDragging } = useFileDrop({
    onDrop,
    onDragOver: handleDragOver,
    onDragStateChange: handleDragStateChange,
    enabled: !!activeGame,
  });

  /** Modal callbacks */
  const handleConfirmMoveAnyway = useCallback(() => {
    if (!dropValidation) return;
    const { targetId, paths } = dropValidation;
    setDropValidation(null);
    handleDropOnItem(targetId, paths);
  }, [dropValidation, handleDropOnItem]);

  const handleConfirmMoveToSuggested = useCallback(() => {
    if (!dropValidation?.suggestedId) return;
    const { suggestedId, paths } = dropValidation;
    setDropValidation(null);
    handleDropOnItem(suggestedId, paths);
  }, [dropValidation, handleDropOnItem]);

  const handleCancelDrop = useCallback(() => {
    setDropValidation(null);
  }, []);

  const handleSkipValidation = useCallback(() => {
    if (!dropValidation) return;
    const { targetId, paths } = dropValidation;
    setDropValidation(null);
    handleDropOnItem(targetId, paths);
  }, [dropValidation, handleDropOnItem]);

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
    handleRevealInExplorer,
    handleEnableObject,
    handleDisableObject,
  };

  return (
    <div className={`flex flex-col h-full bg-base-100/50 relative`}>
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
        />
      </div>

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

      {/* Virtualized list (objects or folders) — item drop zone */}
      <div ref={contentRef} className="flex-1 min-h-0 flex flex-col">
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
            isDragging={isDragging}
            hoveredItemId={hoveredItemId}
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
        onCloseCreate={() => {
          setCreateModalOpen(false);
          setPendingPaths(null);
        }}
      />

      {/* Pre-drop validation modal */}
      <DropConfirmModal
        validation={dropValidation}
        onMoveAnyway={handleConfirmMoveAnyway}
        onMoveToSuggested={handleConfirmMoveToSuggested}
        onCancel={handleCancelDrop}
        onSkipValidation={handleSkipValidation}
      />
    </div>
  );
}
