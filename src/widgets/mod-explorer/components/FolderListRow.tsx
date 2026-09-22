import { memo } from 'react';
import { Folder, File, Copy, AlertTriangle, Star } from 'lucide-react';
import type { ModFolder } from '@/entities/game-object';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import BulkContextMenu from './BulkContextMenu';
import { useModContextMenuActions, useModContextMenuItems } from '@/features/mod-runtime';

import { ContextMenu } from '../../../shared/ui/components/ui/ContextMenu';
import { formatWorkspaceReason } from '@/features/workspace-runtime';
import { WorkspaceSwitchControl } from '@/features/workspace-runtime';
import { WorkspaceSwitchLabel } from '@/features/workspace-runtime';
import { useFolderNodeView } from '../hooks/useFolderNodeView';

interface FolderListRowProps {
  item: WorkspaceExplorerNode;
  isSelected: boolean;
  isActive?: boolean;
  toggleSelection: (id: string, multi: boolean, isShift?: boolean) => void;
  onActivate?: (path: string) => void;
  onToggleEnabled?: (folder: WorkspaceExplorerNode) => void;
  onToggleFavorite?: (folder: ModFolder) => void;
  selectionSize?: number;
  onBulkToggle?: (enable: boolean) => void;
  onBulkDelete?: () => void;
  onBulkTag?: () => void;
  onBulkFavorite?: (favorite: boolean) => void;
  onBulkSafe?: (safe: boolean) => void;
  onBulkPin?: (pin: boolean) => void;
  onBulkMoveToObject?: () => void;
  onRename?: (folder: ModFolder) => void;
  onDelete?: (folder: ModFolder) => void;
  onOpenMoveDialog?: (folder: ModFolder) => void;
  onToggleSafe?: (folder: ModFolder) => void;
  onEnableOnlyThis?: (folder: ModFolder) => void;
  onSyncWithDb?: (folder: ModFolder) => void;
  hasConflict?: boolean;
  hasFolderNameConflict?: boolean;
  isSwitchPending?: boolean;
  /** The immediate desired state while a switch transaction is in flight. */
  pendingDesiredEnabled?: boolean;
  /** A switch for this folder is in flight; used for accessible busy state only. */
  isSwitchBusy?: boolean;
  mutationsDisabled?: boolean;
}

function FolderListRowInner({
  item,
  isSelected,
  isActive,
  toggleSelection,
  onActivate,
  onToggleEnabled,
  onToggleFavorite,
  selectionSize = 0,
  onBulkToggle,
  onBulkDelete,
  onBulkTag,
  onBulkFavorite,
  onBulkSafe,
  onBulkPin,
  onBulkMoveToObject,
  onRename,
  onDelete,
  onOpenMoveDialog,
  onToggleSafe,
  onEnableOnlyThis,
  onSyncWithDb,
  hasConflict = false,
  hasFolderNameConflict = false,
  isSwitchPending = false,
  pendingDesiredEnabled,
  isSwitchBusy = false,
  mutationsDisabled = false,
}: FolderListRowProps) {
  const {
    t,
    typeChip,
    actionNode: actionItem,
    primaryWarningText,
    switchPolicy,
    thumbnailSrc,
    thumbLoading,
    setImgError,
    isBulkSelection,
    bulkMenuProps,
    handleClick,
  } = useFolderNodeView({
    node: item,
    variant: 'row',
    isSelected,
    selectionSize,
    mutationsDisabled,
    toggleSelection,
    onActivate,
    bulk: {
      onBulkToggle,
      onBulkDelete,
      onBulkTag,
      onBulkFavorite,
      onBulkSafe,
      onBulkPin,
      onBulkMoveToObject,
    },
  });
  const displayedSwitchPolicy =
    pendingDesiredEnabled === undefined
      ? switchPolicy
      : {
          ...switchPolicy,
          checked: pendingDesiredEnabled,
          label: switchPolicy.blocked
            ? switchPolicy.label
            : t(pendingDesiredEnabled ? 'common:status.enabled' : 'common:status.disabled'),
        };
  const inactiveReasonText = formatWorkspaceReason(t, item.inactive_reason);
  const contextActions = useModContextMenuActions(item);
  const contextItems = useModContextMenuItems({
    folder: actionItem,
    onRename: () => !mutationsDisabled && onRename?.(item),
    onDelete: () => !mutationsDisabled && onDelete?.(item),
    onToggleEnabled: () => !mutationsDisabled && onToggleEnabled?.(item),
    onToggleFavorite: () => !mutationsDisabled && onToggleFavorite?.(item),
    onEnableOnlyThis:
      onEnableOnlyThis && !mutationsDisabled ? () => onEnableOnlyThis(item) : undefined,
    onOpenMoveDialog: mutationsDisabled ? undefined : onOpenMoveDialog,
    onToggleSafe: mutationsDisabled ? undefined : () => onToggleSafe?.(item),
    onSyncWithDb: onSyncWithDb && !mutationsDisabled ? () => onSyncWithDb(item) : undefined,
    onOpenExplorer: hasFolderNameConflict ? undefined : contextActions.openExplorer,
    onPasteThumbnail: mutationsDisabled ? undefined : contextActions.pasteThumbnailFromClipboard,
    onImportThumbnail: mutationsDisabled ? undefined : contextActions.importThumbnail,
  });

  const handleContextClick = (_: React.MouseEvent) => {
    if (item.display_mode === 'internal_assets') {
      return;
    }

    // If we right click and it's NOT selected, select it (and clear others if no modifier)
    // Actually ContextMenu trigger handles visibility, but we want to ensure selection logic is visually consistent.
    // If user right clicks an unselected item, standard OS behavior is to select it.
    if (!isSelected) {
      toggleSelection(item.path, false);
    }
  };

  return (
    <ContextMenu
      content={
        isBulkSelection ? (
          <BulkContextMenu {...bulkMenuProps} />
        ) : (
          <>
            {contextItems.map((contextItem) => {
              if (contextItem.hidden) {
                return null;
              }

              return (
                <div key={contextItem.id}>
                  {contextItem.separatorBefore ? <div className="divider my-0" /> : null}
                  <button
                    type="button"
                    className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm ${
                      contextItem.danger ? 'text-error hover:bg-error/10' : 'hover:bg-base-200'
                    }`}
                    onClick={contextItem.onClick}
                  >
                    <contextItem.icon size={14} className="opacity-70" />
                    {contextItem.label}
                  </button>
                </div>
              );
            })}
          </>
        )
      }
    >
      <div
        id={`grid-item-${item.path}`}
        onClick={handleClick}
        onContextMenu={handleContextClick}
        className={`
        group flex h-full items-center gap-3 rounded-md border p-2 cursor-pointer transition-[background-color,border-color] duration-150
        ${item.display_mode === 'internal_assets' ? 'opacity-50' : ''}
        ${!item.is_effectively_active ? 'opacity-[0.65] grayscale-[0.8]' : ''}
        ${
          isActive
            ? 'border-primary/60 bg-primary/10'
            : isSelected
              ? 'border-primary/50 bg-primary/10'
              : 'border-base-content/5 bg-base-200 hover:bg-base-300'
        }
      `}
      >
        <div className="group relative w-10 h-10 shrink-0 bg-base-300 rounded-md overflow-hidden flex items-center justify-center select-none border border-base-content/5">
          {thumbLoading ? (
            <div className="w-full h-full skeleton bg-base-300" />
          ) : thumbnailSrc ? (
            <img
              src={thumbnailSrc}
              alt=""
              decoding="async"
              className="w-full h-full object-cover opacity-90"
              draggable={false}
              onError={() => setImgError(true)}
            />
          ) : item.is_directory ? (
            <Folder size={18} className="text-base-content/20" />
          ) : (
            <File size={18} className="text-base-content/20" />
          )}

          {/* Bulk selection stays centered over the thumbnail in the compact row layout. */}
          <div
            className={`pointer-events-none absolute inset-0 z-20 flex items-center justify-center transition-opacity duration-150
              ${isSelected ? 'opacity-100' : 'opacity-100 sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100'}`}
          >
            <input
              type="checkbox"
              className="pointer-events-auto checkbox checkbox-primary checkbox-xs border shadow-sm bg-base-100"
              checked={isSelected}
              onChange={(e) => {
                e.stopPropagation();
                const isShift =
                  e.nativeEvent instanceof MouseEvent && (e.nativeEvent as MouseEvent).shiftKey;
                toggleSelection(item.path, true, isShift);
              }}
              onClick={(e) => e.stopPropagation()}
            />
          </div>
        </div>

        <div className="min-w-0 flex-1 flex items-center gap-3">
          <div
            className={`text-sm font-medium truncate leading-tight flex-1
            ${isActive || isSelected ? 'text-primary' : 'text-base-content/80'}
            ${!displayedSwitchPolicy.checked ? 'line-through text-base-content/50' : ''}
            ${!item.is_effectively_active && displayedSwitchPolicy.checked ? 'text-base-content/55' : ''}`}
          >
            {item.display_name}
          </div>

          {/* Node type badge */}
          {typeChip && (
            <div
              className={`flex items-center gap-0.5 rounded-md px-1.5 py-0.5 shrink-0 ${typeChip.className}`}
            >
              <typeChip.icon size={10} />
              <span className="text-[9px] font-bold">{typeChip.label}</span>
            </div>
          )}

          {(hasFolderNameConflict || hasConflict) && (
            <div className="flex shrink-0 items-center rounded-md border border-base-content/10 bg-base-100 p-0.5">
              {hasFolderNameConflict && (
                <div
                  role="img"
                  aria-label={t('card.name_conflict')}
                  className="flex h-5 w-5 items-center justify-center rounded-sm text-warning"
                  title={t('card.name_conflict_title')}
                >
                  <AlertTriangle size={10} />
                </div>
              )}
              {hasFolderNameConflict && hasConflict && (
                <span className="h-3 w-px bg-base-content/15" />
              )}
              {hasConflict && (
                <div
                  role="img"
                  aria-label={t('card.shared_hash')}
                  className="flex h-5 w-5 items-center justify-center rounded-sm text-info"
                  title={t('card.hash_conflict_title')}
                >
                  <Copy size={10} />
                </div>
              )}
            </div>
          )}

          {/* Corrupt badge */}
          {!hasConflict && !hasFolderNameConflict && item.warnings.length > 0 && (
            <div
              className="flex items-center gap-0.5 px-1.5 py-0.5 bg-error/20 text-error rounded-md shrink-0"
              title={primaryWarningText || item.warnings.join('\n') || t('card.corrupt_ini_title')}
            >
              <AlertTriangle size={10} />
              <span className="text-[9px] font-bold">{t('badges.corrupt')}</span>
            </div>
          )}

          <div className="flex items-center gap-2 shrink-0">
            <button
              onClick={(e) => {
                e.stopPropagation();
                if (mutationsDisabled) {
                  return;
                }
                onToggleFavorite?.(item);
              }}
              className={`rounded-full p-1 transition-[color,opacity] duration-150
                 ${
                   item.is_favorite
                     ? 'text-warning opacity-100'
                     : 'text-base-content/45 opacity-100 sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100 hover:text-warning'
                 }
               `}
              title={t(item.is_favorite ? 'card.unfavorite' : 'card.favorite')}
              disabled={mutationsDisabled}
            >
              <Star
                size={16}
                className={`drop-shadow-sm ${item.is_favorite ? 'fill-current' : ''}`}
              />
            </button>
            <label
              className="flex items-center gap-1.5 cursor-pointer"
              onClick={(e) => e.stopPropagation()}
            >
              <WorkspaceSwitchControl
                node={actionItem}
                policy={displayedSwitchPolicy}
                isPending={isSwitchPending || mutationsDisabled}
                isBusy={isSwitchBusy}
                size="xs"
                ariaLabel={t('common:actions.toggle')}
                onToggle={() => {
                  if (!mutationsDisabled) {
                    onToggleEnabled?.(item);
                  }
                }}
              />
              <WorkspaceSwitchLabel
                node={actionItem}
                policy={displayedSwitchPolicy}
                className="text-[10px] font-semibold text-base-content/40 hidden sm:inline"
              />
              {inactiveReasonText && !displayedSwitchPolicy.checked && (
                <span className="hidden text-[10px] text-warning/70 sm:inline">
                  {inactiveReasonText}
                </span>
              )}
            </label>
          </div>
        </div>
      </div>
    </ContextMenu>
  );
}

/** Memoized FolderListRow — prevents re-renders when virtualizer recalculates but props are unchanged */
const FolderListRow = memo(FolderListRowInner);
export default FolderListRow;
