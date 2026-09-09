import { CheckSquare, FolderOpen } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import FolderCard from './FolderCard';
import FolderListRow from './FolderListRow';
import { cn } from '../../../shared/lib/utils';
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from '../../../shared/ui/components/ui/ContextMenu';
import { normalizeWorkspacePath } from '@/features/workspace-runtime';
import { isFolderConflictProtected } from '../hooks/folderConflictScope';
import type { useFolderGrid } from '../hooks/useFolderGrid';

type FolderGridModel = ReturnType<typeof useFolderGrid>;

interface FolderGridContentProps {
  model: FolderGridModel;
  visibleFolders: FolderGridModel['sortedFolders'];
  conflictPathSet: Set<string>;
  folderConflictScopes: string[];
  mutationsDisabled: boolean;
  onSelectAll: () => void;
}

export default function FolderGridContent({
  model,
  visibleFolders,
  conflictPathSet,
  folderConflictScopes,
  mutationsDisabled,
  onSelectAll,
}: FolderGridContentProps) {
  const { t } = useTranslation(['grid']);
  const {
    parentRef,
    virtualItems,
    totalSize,
    columnCount,
    cardWidth,
    isGridView,
    isPlaceholderData,
    isLoading,
    isError,
    gridSelection,
    selectedModPath,
    handleNavigate,
    activateGridItem,
    toggleGridSelection,
    handleToggleEnabledGuarded,
    handleToggleFavorite,
    handleEnableOnlyThis,
    renamingId,
    handleRenameSubmit,
    handleRenameCancel,
    handleRenameRequest,
    handleDeleteRequest,
    focusedId,
    handleBulkToggle,
    handleBulkDeleteRequest,
    handleBulkTagRequest,
    handleBulkFavorite,
    handleBulkSafe,
    handleBulkPin,
    handleBulkMoveToObject,
    openMoveDialog,
    handleToggleSafeRequest,
    handleSyncWithDb,
    ancestorDisabledBy,
    openEnableParentDialog,
    isSwitchPending,
    isFolderSwitchPending,
    currentAbsPath,
    handleOpenCurrentFolderInExplorer,
  } = model;

  return (
    <ContextMenu
      content={
        <>
          <ContextMenuItem
            icon={CheckSquare}
            onClick={onSelectAll}
            disabled={visibleFolders.length === 0}
          >
            {t('context.select_all')}
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem
            icon={FolderOpen}
            onClick={() => {
              void handleOpenCurrentFolderInExplorer();
            }}
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
          'min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto scrollbar-thin scrollbar-track-transparent scrollbar-thumb-base-content/20 hover:scrollbar-thumb-base-content/40 transition-opacity duration-150',
          isPlaceholderData ? 'opacity-70 pointer-events-none select-none' : 'opacity-100',
          !isLoading && !isError && visibleFolders.length > 0 ? 'block' : 'hidden',
        )}
      >
        <div className="relative w-full" style={{ height: `${totalSize}px` }}>
          {virtualItems.map((virtualRow) => {
            if (isGridView) {
              const fromIndex = virtualRow.index * columnCount;
              const toIndex = Math.min(fromIndex + columnCount, visibleFolders.length);
              const rowItems = visibleFolders.slice(fromIndex, toIndex);

              return (
                <div
                  key={virtualRow.index}
                  className="absolute top-0 left-0 w-full grid gap-3 justify-center"
                  style={{
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                    gridTemplateColumns: `repeat(${columnCount}, ${cardWidth}px)`,
                  }}
                >
                  {rowItems.map((folder) => {
                    const protectedByConflict = isFolderConflictProtected(
                      folder.path,
                      folderConflictScopes,
                    );
                    const folderMutationsDisabled = mutationsDisabled || protectedByConflict;
                    return (
                      <div key={folder.path} className="min-w-0">
                        <FolderCard
                          folder={folder}
                          isSelected={gridSelection.has(folder.path)}
                          isActive={selectedModPath === folder.path}
                          onNavigate={handleNavigate}
                          onActivate={activateGridItem}
                          toggleSelection={toggleGridSelection}
                          onToggleEnabled={handleToggleEnabledGuarded}
                          onToggleFavorite={handleToggleFavorite}
                          onEnableOnlyThis={handleEnableOnlyThis}
                          isRenaming={renamingId === folder.path}
                          onRenameSubmit={handleRenameSubmit}
                          onRenameCancel={handleRenameCancel}
                          onRename={handleRenameRequest}
                          onDelete={handleDeleteRequest}
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
                          onToggleSafe={handleToggleSafeRequest}
                          onSyncWithDb={handleSyncWithDb}
                          hasConflict={conflictPathSet.has(normalizeWorkspacePath(folder.path))}
                          hasFolderNameConflict={protectedByConflict}
                          isLockedByParent={!!ancestorDisabledBy}
                          onRequestEnableParent={openEnableParentDialog}
                          isSwitchPending={
                            folderMutationsDisabled ||
                            isSwitchPending ||
                            isFolderSwitchPending(folder)
                          }
                          isSwitchBusy={isFolderSwitchPending(folder)}
                          mutationsDisabled={folderMutationsDisabled}
                        />
                      </div>
                    );
                  })}
                </div>
              );
            }

            const folder = visibleFolders[virtualRow.index];
            if (!folder) {
              return null;
            }

            const protectedByConflict = isFolderConflictProtected(
              folder.path,
              folderConflictScopes,
            );
            const folderMutationsDisabled = mutationsDisabled || protectedByConflict;
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
                  isActive={selectedModPath === folder.path}
                  onActivate={activateGridItem}
                  toggleSelection={toggleGridSelection}
                  onToggleEnabled={handleToggleEnabledGuarded}
                  selectionSize={gridSelection.size}
                  onBulkToggle={handleBulkToggle}
                  onBulkDelete={handleBulkDeleteRequest}
                  onBulkTag={handleBulkTagRequest}
                  onBulkFavorite={handleBulkFavorite}
                  onBulkSafe={handleBulkSafe}
                  onBulkPin={handleBulkPin}
                  onBulkMoveToObject={handleBulkMoveToObject}
                  onRename={handleRenameRequest}
                  onDelete={handleDeleteRequest}
                  onToggleFavorite={handleToggleFavorite}
                  onEnableOnlyThis={handleEnableOnlyThis}
                  onOpenMoveDialog={openMoveDialog}
                  onToggleSafe={handleToggleSafeRequest}
                  onSyncWithDb={handleSyncWithDb}
                  hasConflict={conflictPathSet.has(normalizeWorkspacePath(folder.path))}
                  hasFolderNameConflict={protectedByConflict}
                  isSwitchPending={
                    folderMutationsDisabled || isSwitchPending || isFolderSwitchPending(folder)
                  }
                  isSwitchBusy={isFolderSwitchPending(folder)}
                  mutationsDisabled={folderMutationsDisabled}
                />
              </div>
            );
          })}
        </div>
      </div>
    </ContextMenu>
  );
}
