import { useState, useCallback, useRef, useMemo } from 'react';
import { useObjectListLogic } from './useObjectListLogic';
import { useFileDrop } from '../../hooks/useFileDrop';
import { useDragAutoScroll } from '../../hooks/useDragAutoScroll';
import ObjectListToolbar from './ObjectListToolbar';
import ObjectListContent from './ObjectListContent';
import { useObjectListDropZones } from './useObjectListDropZones';
import { useAppStore } from '../../stores/useAppStore';
import { cn } from '../../lib/utils';
import { useObjectListEffects } from './hooks/useObjectListEffects';
import ObjectListConflictBanner from './ObjectListConflictBanner';
import ObjectListDropIndicators from './ObjectListDropIndicators';
import ObjectListAuxiliaryModals from './ObjectListAuxiliaryModals';
import { useObjectListBulkToolbarProps } from './useObjectListBulkToolbarProps';
import { useObjectListKeyboard } from './useObjectListKeyboard';
import ObjectListPrimaryModals from './ObjectListPrimaryModals';
import ObjectListStateHost from './ObjectListStateHost';
import { useObjectListContextMenuProps } from './useObjectListContextMenuProps';

export default function ObjectList() {
  const { state, filters, nav, virtualizer, modals, handlers, bulkSelect } = useObjectListLogic();

  const {
    objects,
    isLoading,
    isError,
    objectsErrorInfo,
    activeGame,
    isMobile,
    isSyncing,
    sourceAvailable,
  } = state;
  const mutationsDisabled = !sourceAvailable;

  const {
    activeFilters,
    categoryFilters,
    schema,
    sortBy,
    setSortBy,
    statusFilter,
    setStatusFilter,
    handleFilterChange,
    handleClearFilters,
  } = filters;

  const {
    selectedObjectFolderPath,
    selectObject,
    selectedObjectType,
    setSelectedObjectType,
    sidebarSearchQuery,
    setSidebarSearch,
  } = nav;
  const {
    parentRef,
    rowVirtualizer,
    flatObjectItems,
    stickyPosition,
    selectedIndex,
    scrollToSelected,
  } = virtualizer;

  const { archiveModal, bulkTagModal, setBulkTagModal } = modals;

  const {
    handleDeleteObject,
    handleEdit,
    handlePin,
    handleMoveCategory,
    handleRevealInExplorer,
    handleEnableObject,
    handleDisableObject,
    categoryNames,
    handleSync,
    handleBackgroundSync,
    handleSyncWithDb,
    handleDropOnItem,
    handleDropAutoOrganize,
    handleArchivesInteractively,
    handleArchiveExtractSubmit,
    handleArchiveExtractSkip,
    handleStopExtraction,
    handleBulkDelete,
    handleBulkPin,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkAddTags,
    handleBulkRemoveTags,
    handleBulkAutoRecognize,
    handleBulkFavorite,
    handleBulkSafe,
  } = handlers;

  const activeGameId = activeGame?.id ?? null;

  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [autoSetupOpen, setAutoSetupOpen] = useState(false);
  const [pendingPaths, setPendingPaths] = useState<string[] | null>(null);

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
    enabled: !!activeGame && sourceAvailable,
  });

  useDragAutoScroll({
    containerRef: contentRef,
    dragPosition,
    speed: 8,
    threshold: 50,
  });

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

  const handleRefresh = useCallback(async () => {
    if (activeGame) {
      await handleBackgroundSync();
    }
  }, [activeGame, handleBackgroundSync]);

  useObjectListEffects({
    activeGameId,
    handleBackgroundSync,
    handleDropAutoOrganize,
    handleArchivesInteractively,
  });

  const isEmpty = !isLoading && !isError && objects.length === 0;
  const hasNoGame = !activeGame;
  const showContent = !isLoading && !isError && !isEmpty && !hasNoGame;
  const showFilterPanel = !!activeGame;
  const conflictObjects = useMemo(() => objects.filter((o) => o.has_naming_conflict), [objects]);

  const contextMenuProps = useObjectListContextMenuProps({
    isSyncing,
    categoryNames,
    handleEdit,
    handleSyncWithDb,
    handleDeleteObject,
    handlePin,
    handleMoveCategory,
    handleRevealInExplorer,
    handleEnableObject,
    handleDisableObject,
  });

  const activePane = useAppStore((state) => state.activePane);
  const setActivePane = useAppStore((state) => state.setActivePane);

  const bulkSelectToolbarProps = useObjectListBulkToolbarProps({
    activePane,
    mutationsDisabled,
    bulkSelect,
    setBulkTagModal,
    handleBulkDelete,
    handleBulkPin,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkAutoRecognize,
    handleBulkFavorite,
    handleBulkSafe,
  });
  const keyboardHandlers = useObjectListKeyboard({
    activePane,
    mutationsDisabled,
    selectedObjectFolderPath,
    objects,
    bulkSelect,
    setActivePane,
    handleBulkDelete,
    handleDeleteObject,
  });

  return (
    <div
      className={cn(
        'flex flex-col h-full bg-base-100/50 relative outline-none transition-shadow duration-200',
        activePane === 'objectList' && 'ring-1 ring-inset ring-primary/20',
      )}
      tabIndex={-1}
      onFocus={keyboardHandlers.onFocus}
      onKeyDown={keyboardHandlers.onKeyDown}
    >
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
          mutationsDisabled={mutationsDisabled}
          bulkSelect={bulkSelectToolbarProps}
        />
      </div>

      <ObjectListConflictBanner conflictObjects={conflictObjects} activeGame={activeGame} />

      <ObjectListStateHost
        isLoading={isLoading}
        isError={isError}
        errorInfo={objectsErrorInfo}
        hasNoGame={hasNoGame}
        isEmpty={isEmpty}
        sidebarSearchQuery={sidebarSearchQuery}
        activeFilters={activeFilters}
        onClearFilters={handleClearFilters}
        onClearSearch={() => setSidebarSearch('')}
        onCreateNew={() => setCreateModalOpen(true)}
        onAutoSetup={() => setAutoSetupOpen(true)}
      />

      <div ref={contentRef} className="flex-1 min-h-0 flex flex-col">
        {showContent && (
          <ObjectListContent
            parentRef={parentRef}
            rowVirtualizer={rowVirtualizer}
            flatObjectItems={flatObjectItems}
            selectedObjectFolderPath={selectedObjectFolderPath}
            onSelectObject={selectObject}
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
            mutationsDisabled={mutationsDisabled}
          />
        )}
      </div>

      <ObjectListDropIndicators
        isDragging={isDragging}
        activeDropZone={activeDropZone}
        hoveredItemId={hoveredItemId}
        tooltipTop={tooltipTop}
        objects={objects}
        selectedObjectType={selectedObjectType}
        objectCount={objects.length}
        onShowAll={() => setSelectedObjectType(null)}
        bottomRef={bottomRef}
      />

      <ObjectListPrimaryModals
        activeGame={activeGame}
        objects={objects}
        modals={modals}
        handlers={handlers}
        createModalOpen={createModalOpen}
        pendingPaths={pendingPaths}
        autoSetupOpen={autoSetupOpen}
        onCloseCreate={() => {
          setCreateModalOpen(false);
          setPendingPaths(null);
        }}
        onCloseAutoSetup={() => setAutoSetupOpen(false)}
      />

      <ObjectListAuxiliaryModals
        dropValidation={dropValidation}
        onMoveAnyway={handleConfirmMoveAnyway}
        onMoveToSuggested={handleConfirmMoveToSuggested}
        onCancelDrop={handleCancelDrop}
        onSkipValidation={handleSkipValidation}
        archiveModal={archiveModal}
        objects={objects}
        onArchiveExtractSubmit={handleArchiveExtractSubmit}
        onArchiveExtractSkip={handleArchiveExtractSkip}
        onStopExtraction={handleStopExtraction}
        bulkTagModal={bulkTagModal}
        selectedIds={bulkSelect.selectedIds}
        onBulkAddTags={handleBulkAddTags}
        onBulkRemoveTags={handleBulkRemoveTags}
        onCloseBulkTagModal={() => setBulkTagModal({ open: false, mode: 'add' })}
        onClearBulkSelection={bulkSelect.clearSelection}
      />
    </div>
  );
}
