/**
 * FolderListRow — List view row for a mod folder.
 * Used by FolderGrid in list mode.
 */

import { memo, useState, useEffect, useMemo } from 'react';
import {
  Folder,
  File,
  Copy,
  Package,
  Layers,
  AlertTriangle,
  Star,
  type LucideIcon,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ModFolder } from '../../types/mod';
import type { WorkspaceExplorerNode, WorkspaceTypeChip } from '../../types/workspace';
import BulkContextMenu from './BulkContextMenu';
import { useAppStore } from '../../stores/useAppStore';
import { useModContextMenuItems } from '../../hooks/useModContextMenuItems';
import { useModContextMenuActions } from '../mod-runtime/actions/useModContextMenuActions';

import {
  ContextMenu,
} from '../../components/ui/ContextMenu';
import { useThumbnail } from '../../hooks/useThumbnail';
import {
  formatWorkspaceReason,
  formatWorkspaceWarning,
} from '../workspace-runtime/workspaceSemantics';
import { buildWorkspaceSwitchPolicy } from '../workspace-runtime/actions/workspaceSwitchPolicy';
import { WorkspaceSwitchControl } from '../workspace-runtime/components/WorkspaceSwitchControl';
import { WorkspaceSwitchLabel } from '../workspace-runtime/components/WorkspaceSwitchLabel';

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
  onToggleSafe?: () => void;
  onEnableOnlyThis?: (folder: ModFolder) => void;
  onSyncWithDb?: (folder: ModFolder) => void;
  hasConflict?: boolean;
  isSwitchPending?: boolean;
}

function getTypeChip(
  typeChip: WorkspaceTypeChip | null,
  t: ReturnType<typeof useTranslation>['t'],
): { label: string; className: string; icon: LucideIcon } | null {
  if (typeChip === 'mod_pack') {
    return {
      label: t('card.mod_pack'),
      className: 'bg-info/20 text-info',
      icon: Package,
    };
  }

  if (typeChip === 'variant') {
    return {
      label: t('card.variants'),
      className: 'bg-secondary/20 text-secondary',
      icon: Layers,
    };
  }

  if (typeChip === 'flat_mod') {
    return {
      label: t('card.flat_mod'),
      className: 'bg-base-300 text-base-content/70',
      icon: Folder,
    };
  }

  return null;
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
  isSwitchPending = false,
}: FolderListRowProps) {
  const { t } = useTranslation(['grid', 'common']);
  const typeChip = getTypeChip(item.type_chip, t);
  const inactiveReasonText = formatWorkspaceReason(t, item.inactive_reason);
  const primaryWarningText = formatWorkspaceWarning(t, item.primary_warning);
  const switchPolicy = useMemo(() => buildWorkspaceSwitchPolicy(t, item), [item, t]);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const { data: thumbnailPath, isLoading: thumbLoading } = useThumbnail(
    activeGameId || '',
    item.path,
  );
  const contextActions = useModContextMenuActions(item);
  const [imgError, setImgError] = useState(false);
  const thumbnailSrc = thumbnailPath && !imgError ? thumbnailPath : null;
  const isBulkSelection =
    isSelected && selectionSize > 1 && useAppStore.getState().activePane === 'folderGrid';
  const contextItems = useModContextMenuItems({
    folder: item,
    onRename: () => onRename?.(item),
    onDelete: () => onDelete?.(item),
    onToggleEnabled: () => onToggleEnabled?.(item),
    onToggleFavorite: () => onToggleFavorite?.(item),
    onEnableOnlyThis: onEnableOnlyThis ? () => onEnableOnlyThis(item) : undefined,
    onOpenMoveDialog,
    onToggleSafe,
    onSyncWithDb: onSyncWithDb ? () => onSyncWithDb(item) : undefined,
    onOpenExplorer: contextActions.openExplorer,
    onPasteThumbnail: contextActions.pasteThumbnailFromClipboard,
    onImportThumbnail: contextActions.importThumbnail,
  });

  // Reset error state when thumbnail path changes
  useEffect(() => {
    setImgError(false);
  }, [thumbnailPath]);

  const handleClick = (e: React.MouseEvent) => {
    if (item.display_mode === 'internal_assets') {
      return;
    }
    if (e.ctrlKey || e.shiftKey) {
      toggleSelection(item.path, true, e.shiftKey);
    } else {
      onActivate?.(item.path);
    }
  };

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
          <BulkContextMenu
            count={selectionSize}
            onToggle={onBulkToggle}
            onDelete={onBulkDelete}
            onTag={onBulkTag}
            onFavorite={onBulkFavorite}
            onSafe={onBulkSafe}
            onPin={onBulkPin}
            onMoveToObject={onBulkMoveToObject}
          />
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
                      contextItem.danger
                        ? 'text-error hover:bg-error/10'
                        : 'hover:bg-base-200'
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
        flex items-center gap-3 p-2 rounded-md border cursor-pointer active:scale-[0.99] transition-all h-full
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

          {/* Bulk Multi-Select Checkbox Overlay: positioned in top-right corner */}
          <div
            className={`absolute top-0.5 right-0.5 transition-all duration-200 z-20
              ${isSelected ? 'opacity-100 scale-100' : 'opacity-0 scale-75 group-hover:opacity-100 group-hover:scale-100'}`}
          >
            <input
              type="checkbox"
              className="checkbox checkbox-primary checkbox-xs border shadow-sm bg-base-100"
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
            ${!switchPolicy.checked ? 'line-through text-base-content/50' : ''}
            ${!item.is_effectively_active && switchPolicy.checked ? 'text-base-content/55' : ''}`}
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

          {/* Conflict badge */}
          {hasConflict && (
            <div
              className="flex items-center gap-0.5 px-1.5 py-0.5 bg-warning/20 text-warning rounded-md shrink-0"
              title={t('card.hash_conflict_title')}
            >
              <Copy size={10} />
              <span className="text-[9px] font-bold">{t('card.conflict')}</span>
            </div>
          )}

          {/* Corrupt badge */}
          {!hasConflict && item.warnings.length > 0 && (
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
                onToggleFavorite?.(item);
              }}
              className={`p-1 rounded-full transition-all duration-200
                 ${
                   item.is_favorite
                     ? 'text-warning opacity-100 hover:scale-110'
                     : 'text-base-content/20 opacity-0 group-hover:opacity-100 hover:text-warning hover:scale-110'
                 }
               `}
              title={t(item.is_favorite ? 'card.unfavorite' : 'card.favorite')}
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
                node={item}
                policy={switchPolicy}
                isPending={isSwitchPending}
                size="xs"
                ariaLabel={t('common:actions.toggle')}
                onToggle={() => {
                  onToggleEnabled?.(item);
                }}
              />
              <WorkspaceSwitchLabel
                node={item}
                policy={switchPolicy}
                className="text-[10px] font-semibold text-base-content/40 hidden sm:inline"
              />
              {inactiveReasonText && !switchPolicy.checked && (
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
