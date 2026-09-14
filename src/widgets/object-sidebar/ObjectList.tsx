import { useState, useRef, useMemo } from 'react';
import { useObjectListLogic } from './hooks/useObjectListLogic';
import { useFileDrop } from '../../shared/lib/hooks/useFileDrop';
import { useDragAutoScroll } from '../../shared/lib/hooks/useDragAutoScroll';
import ObjectListToolbar from './components/ObjectListToolbar';
import ObjectListContent, { type ContextMenuHandlerProps } from './components/ObjectListContent';
import { useObjectListDropZones } from './hooks/useObjectListDropZones';
import { useAppStore } from '@/app/store';
import { cn } from '../../shared/lib/utils';
import { useObjectListEffects } from './hooks/useObjectListEffects';
import ObjectListConflictBanner from './components/ObjectListConflictBanner';
import ObjectListDropIndicators from './components/ObjectListDropIndicators';
import ObjectListAuxiliaryModals from './modals/ObjectListAuxiliaryModals';
import { useObjectListBulkToolbarProps } from './hooks/useObjectListBulkToolbarProps';
import { useObjectListKeyboard } from './hooks/useObjectListKeyboard';
import ObjectListPrimaryModals from './modals/ObjectListPrimaryModals';
import ObjectListStates from './components/ObjectListStates';
import { commands } from '@/shared/api/tauri/bindings';
import { toast } from '@/shared/ui/toast';
import { useTranslation } from 'react-i18next';

export default function ObjectList() {
  const { t } = useTranslation('objects');
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

  const { bulkTagModal, setBulkTagModal } = modals;

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
    handleBulkDelete,
    handleBulkPin,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkAddTags,
    handleBulkRemoveTags,
    handleBulkClassifyAndMatch,
    handleBulkFavorite,
    handleBulkSafe,
  } = handlers;

  const activeGameId = activeGame?.id ?? null;

  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [autoSetupOpen, setAutoSetupOpen] = useState(false);
  const [pendingPaths, setPendingPaths] = useState<string[] | null>(null);
  const [isCatalogChecking, setIsCatalogChecking] = useState(false);
  const selectedObject = useMemo(
    () => objects.find((object) => object.folder_path === selectedObjectFolderPath) ?? null,
    [objects, selectedObjectFolderPath],
  );
  const checkSelectedObjectCatalog = async () => {
    if (!activeGameId || !selectedObject) return;
    setIsCatalogChecking(true);
    try {
      await commands.retryObjectIdentitySuggestions(activeGameId, [selectedObject.folder_path]);
      toast.info(t('toolbar.catalog_check_started'));
    } catch {
      toast.error(t('toolbar.catalog_check_failed'));
    } finally {
      setIsCatalogChecking(false);
    }
  };

  const toolbarRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  const {
    activeDropZone,
    hoveredItemId,
    tooltipTop,
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

  useObjectListEffects({
    activeGameId,
    handleBackgroundSync,
    handleDropAutoOrganize,
  });

  const isEmpty = !isLoading && !isError && objects.length === 0;
  const hasNoGame = !activeGame;
  const showContent = !isLoading && !isError && !isEmpty && !hasNoGame;
  const showFilterPanel = !!activeGame;
  const conflictObjects = useMemo(() => objects.filter((o) => o.has_naming_conflict), [objects]);

  // ponytail: plain object, no memo. It is consumed by a component that already
  // re-renders with this one, so a stable identity bought nothing.
  const contextMenuProps: ContextMenuHandlerProps = {
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
  };

  const activePane = useAppStore((state) => state.activePane);
  const setActivePane = useAppStore((state) => state.setActivePane);

  const bulkSelectToolbarProps = useObjectListBulkToolbarProps({
    mutationsDisabled,
    bulkSelect,
    setBulkTagModal,
    handleBulkDelete,
    handleBulkPin,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkClassifyAndMatch,
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
      data-testid="object-list-panel"
      className={cn(
        'object-list-panel flex h-full flex-col bg-base-100/50 pt-[var(--workspace-topbar-height)] relative outline-none transition-shadow duration-200',
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
          onCreateNew={() => setCreateModalOpen(true)}
          onCheckCatalog={selectedObject ? () => void checkSelectedObjectCatalog() : undefined}
          isCatalogChecking={isCatalogChecking}
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

      <ObjectListConflictBanner conflictObjects={conflictObjects} />

      <ObjectListStates
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
            isBulkSelected={bulkSelect.isSelected}
            onToggleBulkSelect={bulkSelect.toggleSelection}
            mutationsDisabled={mutationsDisabled}
            isObjectSwitchPending={handlers.isObjectSwitchPending}
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
        objects={objects}
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
